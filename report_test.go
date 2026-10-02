package main

import (
	"strings"
	"testing"
)

func TestRender(t *testing.T) {
	got := report{
		"2026-09-14": {
			"service": {"app-api": {"a"}},
			"ui":      {"app-ui": {"b", "c"}},
		},
	}.render(false)
	want := "14 ก.ย.\n[ui]\n- app-ui: b · c\n[service]\n- app-api: a\n\n"
	if got != want {
		t.Fatalf("render =\n%s\nwant\n%s", got, want)
	}
	if !strings.Contains(report{"2026-10-02": {"ui": {"x": {"y"}}}}.render(true), "```") {
		t.Fatal("md ไม่มี ```")
	}
}
