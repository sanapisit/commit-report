package main

import "testing"

func TestParseLog(t *testing.T) {
	log := "2026-09-14\tfeat(abc1x010): เปิด sort\n" +
		"2026-09-14\tfeat(abc1x010): เปิด sort\n" + // ซ้ำจาก branch อื่น
		"2026-09-14\tupdate version : 1.0.81\n" +
		"2026-09-14\tchore: อัปเดตเวอร์ชันเป็น 0.0.51\n" +
		"2026-09-14\tfix: ปิดการแปลภาษา\n"
	cs := parseLog([]byte(log))
	if len(cs) != 2 || cs[0].subject != "abc1x010 เปิด sort" || cs[1].subject != "ปิดการแปลภาษา" {
		t.Fatalf("parseLog = %+v", cs)
	}
}
