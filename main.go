// commit-report: สรุป commit ของตัวเองจากทุก git repo ใต้ root แยกตามวัน และแยกเป็น [ui]/[service]
package main

import (
	"flag"
	"fmt"
	"os"
	"os/exec"
	"strings"
	"time"
)

type options struct {
	start, end string
	authors    []string
}

func main() {
	today := time.Now().Format(time.DateOnly)
	root := flag.String("root", ".", "โฟลเดอร์ที่จะค้นหา git repo")
	start := flag.String("start", today, "วันเริ่ม (YYYY-MM-DD)")
	end := flag.String("end", today, "วันสิ้นสุด (YYYY-MM-DD)")
	author := flag.String("author", "", "email ผู้ commit คั่นด้วย , (ค่าเริ่มต้น: git config user.email)")
	depth := flag.Int("depth", 4, "ความลึกสูงสุดที่ค้นหา repo")
	md := flag.Bool("md", false, "ครอบแต่ละวันด้วย ``` สำหรับ copy")
	flag.Parse()

	authors := resolveAuthors(*author)
	if len(authors) == 0 {
		fmt.Fprintln(os.Stderr, "ไม่พบ author ใช้ -author")
		os.Exit(1)
	}

	r := collect(findRepos(*root, *depth), options{*start, *end, authors})
	fmt.Print(r.render(*md))
}

// author ที่ระบุ หรือถ้าไม่ระบุใช้ git config user.email
func resolveAuthors(s string) []string {
	if authors := splitAuthors(s); len(authors) > 0 {
		return authors
	}
	out, _ := exec.Command("git", "config", "--global", "user.email").Output()
	return splitAuthors(string(out))
}

func splitAuthors(s string) []string {
	var res []string
	for a := range strings.SplitSeq(s, ",") {
		if a = strings.TrimSpace(a); a != "" {
			res = append(res, a)
		}
	}
	return res
}
