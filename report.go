package main

import (
	"fmt"
	"maps"
	"slices"
	"strings"
	"time"
)

const (
	kindUI      = "ui"
	kindService = "service"
)

var (
	kinds      = []string{kindUI, kindService}
	thaiMonths = []string{"ม.ค.", "ก.พ.", "มี.ค.", "เม.ย.", "พ.ค.", "มิ.ย.", "ก.ค.", "ส.ค.", "ก.ย.", "ต.ค.", "พ.ย.", "ธ.ค."}
)

// report: date -> kind -> repo -> subjects
type report map[string]map[string]map[string][]string

func (r report) add(date, kind, repo string, subjects ...string) {
	if r[date] == nil {
		r[date] = map[string]map[string][]string{}
	}
	if r[date][kind] == nil {
		r[date][kind] = map[string][]string{}
	}
	r[date][kind][repo] = append(r[date][kind][repo], subjects...)
}

func (r report) render(md bool) string {
	fence := ""
	if md {
		fence = "```\n"
	}
	var b strings.Builder
	for _, d := range slices.Sorted(maps.Keys(r)) {
		b.WriteString(thaiDate(d) + "\n" + fence)
		for _, kind := range kinds {
			repos := r[d][kind]
			if len(repos) == 0 {
				continue
			}
			fmt.Fprintf(&b, "[%s]\n", kind)
			for _, n := range slices.Sorted(maps.Keys(repos)) {
				fmt.Fprintf(&b, "- %s: %s\n", n, strings.Join(repos[n], " · "))
			}
		}
		b.WriteString(fence + "\n")
	}
	return b.String()
}

func thaiDate(d string) string {
	t, err := time.Parse(time.DateOnly, d)
	if err != nil {
		return d
	}
	return fmt.Sprintf("%d %s", t.Day(), thaiMonths[t.Month()-1])
}
