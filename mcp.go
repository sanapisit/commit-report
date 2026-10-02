package main

import (
	"cmp"
	"context"
	"fmt"
	"time"

	"github.com/modelcontextprotocol/go-sdk/mcp"
)

type reportArgs struct {
	Start  string `json:"start,omitempty" jsonschema:"วันเริ่ม YYYY-MM-DD (ค่าเริ่มต้น: วันนี้)"`
	End    string `json:"end,omitempty" jsonschema:"วันสิ้นสุด YYYY-MM-DD (ค่าเริ่มต้น: วันนี้)"`
	Author string `json:"author,omitempty" jsonschema:"email ผู้ commit คั่นด้วย , (ค่าเริ่มต้น: git config user.email)"`
	Root   string `json:"root,omitempty" jsonschema:"โฟลเดอร์ที่จะค้นหา git repo (ค่าเริ่มต้น: -root ของ server)"`
	Depth  int    `json:"depth,omitempty" jsonschema:"ความลึกสูงสุดที่ค้นหา repo (ค่าเริ่มต้น: -depth ของ server)"`
}

// รันเป็น MCP server ผ่าน stdio โดยใช้ root/depth จาก flag เป็นค่าเริ่มต้น
func serveMCP(root string, depth int) error {
	server := mcp.NewServer(&mcp.Implementation{Name: "commit-report", Version: version}, nil)
	mcp.AddTool(server, &mcp.Tool{
		Name:        "commit_report",
		Description: "สรุป commit ของผู้ใช้จากทุก git repo ใต้ root ในช่วงวันที่กำหนด แยกตามวัน และแยก [ui]/[service]",
	}, func(_ context.Context, _ *mcp.CallToolRequest, a reportArgs) (*mcp.CallToolResult, any, error) {
		text, err := runReport(a, root, depth)
		if err != nil {
			return nil, nil, err
		}
		return &mcp.CallToolResult{Content: []mcp.Content{&mcp.TextContent{Text: text}}}, nil, nil
	})
	return server.Run(context.Background(), &mcp.StdioTransport{})
}

func runReport(a reportArgs, root string, depth int) (string, error) {
	today := time.Now().Format(time.DateOnly)
	a.Start, a.End = cmp.Or(a.Start, today), cmp.Or(a.End, today)
	for _, d := range []string{a.Start, a.End} {
		if _, err := time.Parse(time.DateOnly, d); err != nil {
			return "", fmt.Errorf("วันที่ %q ต้องเป็น YYYY-MM-DD", d)
		}
	}
	authors := resolveAuthors(a.Author)
	if len(authors) == 0 {
		return "", fmt.Errorf("ไม่พบ author ให้ระบุ author")
	}
	if a.Depth <= 0 {
		a.Depth = depth
	}
	text := collect(findRepos(cmp.Or(a.Root, root), a.Depth), options{a.Start, a.End, authors}).render(false)
	if text == "" {
		text = fmt.Sprintf("ไม่มี commit ระหว่าง %s ถึง %s", a.Start, a.End)
	}
	return text, nil
}
