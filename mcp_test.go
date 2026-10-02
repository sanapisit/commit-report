package main

import (
	"strings"
	"testing"
)

func TestRunReport(t *testing.T) {
	if _, err := runReport(reportArgs{Start: "14/09/2026", Author: "x@y"}, ".", 1); err == nil {
		t.Fatal("วันที่ผิดรูปแบบต้อง error")
	}
	text, err := runReport(reportArgs{Start: "2000-01-01", End: "2000-01-01", Author: "nobody@invalid"}, ".", 1)
	if err != nil || !strings.HasPrefix(text, "ไม่มี commit") {
		t.Fatalf("runReport = %q, %v", text, err)
	}
}
