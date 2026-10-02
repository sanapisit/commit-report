# commit-report

สรุป commit ของตัวเองจากทุก git repo ใต้โฟลเดอร์ที่กำหนด แยกตามวัน และแยกเป็น `[ui]` / `[service]`

```
14 ก.ย.
[ui]
- app-ui: abc1x010 เปิด sort คอลัมน์ · ปิดการแปลภาษาอัตโนมัติ
[service]
- app-api: abc1x010 เพิ่ม endpoint
```

## ติดตั้ง

ต้องใช้ Go 1.25 ขึ้นไป และมี `git` อยู่ใน PATH

```bash
go install github.com/sanapisit/commit-report@latest
```

หรือ build เอง

```bash
go build -o commit-report.exe .
```

## โครงสร้าง

| ไฟล์ | หน้าที่ |
|---|---|
| `main.go` | อ่าน flag และเรียกใช้งาน |
| `git.go` | ค้นหา repo, รัน `git log`, กรองและแปลง commit |
| `report.go` | จัดกลุ่มตามวัน/ประเภท และ render ผลลัพธ์ |

## วิธีใช้

```bash
./commit-report.exe -root /d/Workspace -start 2026-09-14 -end 2026-10-02
```

| flag | ค่าเริ่มต้น | คำอธิบาย |
|---|---|---|
| `-root` | `.` | โฟลเดอร์ที่จะค้นหา git repo |
| `-start` | วันนี้ | วันเริ่ม (YYYY-MM-DD) |
| `-end` | วันนี้ | วันสิ้นสุด (YYYY-MM-DD) |
| `-author` | `git config user.email` | email ผู้ commit ใส่หลายคนคั่นด้วย `,` |
| `-depth` | `4` | ความลึกสูงสุดที่ค้นหา repo |
| `-md` | `false` | ครอบแต่ละวันด้วย ```` ``` ```` เพื่อให้ copy ง่าย |

## กติกา

- **ui** ถ้าชื่อ repo มี `-ui` หรือมีไฟล์ `angular.json`, `pubspec.yaml`, `index.html` ที่ root ของ repo นอกนั้นเป็น **service**
- ข้าม merge commit, commit ที่แค่ bump version และ `[tag]`
- commit ที่ subject ซ้ำในวันเดียวกัน (เช่น cherry-pick ข้าม branch) แสดงครั้งเดียว
- `feat(scope): ข้อความ` แสดงเป็น `scope ข้อความ`
- ค้นหาทุก branch (`--all`) และข้าม `node_modules`

## Test

```bash
go test .
```
