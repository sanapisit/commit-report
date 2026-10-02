package main

import (
	"bufio"
	"bytes"
	"io/fs"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"runtime"
	"slices"
	"strings"
	"sync"
)

var (
	uiMarkers = []string{"angular.json", "pubspec.yaml", "index.html"}

	// type(scope)!: msg
	conventional = regexp.MustCompile(`^\w+(?:\(([^)]*)\))?!?:\s*`)
	// commit ที่แค่ bump version ไม่ต้องรายงาน
	versionBump = regexp.MustCompile(`(?i)^(chore(\(release\))?:\s*)?((update|bump)\s+)?version\b|ปรับเวอร์ชัน|อัปเดตเวอร์ชัน|^\[tag\]`)
)

type commit struct{ date, subject string }

// รัน git log ทุก repo พร้อมกัน จำกัดจำนวนตาม CPU
func collect(repos []string, opt options) report {
	r := report{}
	var mu sync.Mutex
	var wg sync.WaitGroup
	sem := make(chan struct{}, runtime.NumCPU())
	for _, repo := range repos {
		wg.Go(func() {
			sem <- struct{}{}
			defer func() { <-sem }()
			commits := gitLog(repo, opt)
			if len(commits) == 0 {
				return
			}
			kind, name := repoKind(repo), filepath.Base(repo)
			mu.Lock()
			defer mu.Unlock()
			for _, c := range commits {
				r.add(c.date, kind, name, c.subject)
			}
		})
	}
	wg.Wait()
	return r
}

func findRepos(root string, maxDepth int) []string {
	var repos []string
	filepath.WalkDir(root, func(p string, d fs.DirEntry, err error) error {
		if err != nil || !d.IsDir() {
			return nil
		}
		rel, _ := filepath.Rel(root, p)
		if d.Name() == "node_modules" || strings.Count(rel, string(os.PathSeparator)) >= maxDepth {
			return filepath.SkipDir
		}
		if exists(filepath.Join(p, ".git")) {
			repos = append(repos, p)
			return filepath.SkipDir
		}
		return nil
	})
	return repos
}

func exists(p string) bool {
	_, err := os.Stat(p)
	return err == nil
}

// ui ถ้าชื่อ repo มี -ui หรือเป็นโปรเจค Angular/Flutter, ที่เหลือเป็น service
func repoKind(repo string) string {
	if strings.Contains(filepath.Base(repo), "-ui") ||
		slices.ContainsFunc(uiMarkers, func(f string) bool { return exists(filepath.Join(repo, f)) }) {
		return kindUI
	}
	return kindService
}

func gitLog(repo string, opt options) []commit {
	args := []string{"-C", repo, "log", "--all", "--no-merges", "--reverse",
		"--since=" + opt.start + " 00:00", "--until=" + opt.end + " 23:59:59",
		"--date=format:%Y-%m-%d", "--pretty=%ad%x09%s"}
	for _, a := range opt.authors {
		args = append(args, "--author="+a)
	}
	out, err := exec.Command("git", args...).Output()
	if err != nil {
		return nil
	}
	return parseLog(out)
}

// แปลง output ของ git log เป็น commit ตัด version bump และ subject ซ้ำ (cherry-pick ข้าม branch)
func parseLog(out []byte) []commit {
	var res []commit
	seen := map[commit]bool{}
	sc := bufio.NewScanner(bytes.NewReader(out))
	for sc.Scan() {
		date, subj, ok := strings.Cut(sc.Text(), "\t")
		if !ok || versionBump.MatchString(subj) {
			continue
		}
		c := commit{date, cleanSubject(subj)}
		if c.subject == "" || seen[c] {
			continue
		}
		seen[c] = true
		res = append(res, c)
	}
	return res
}

// "feat(scope): เพิ่มคอลัมน์" -> "scope เพิ่มคอลัมน์"
func cleanSubject(s string) string {
	s = strings.TrimSpace(s)
	if m := conventional.FindStringSubmatch(s); m != nil {
		s = strings.TrimSpace(strings.TrimSpace(m[1]) + " " + s[len(m[0]):])
	}
	return s
}
