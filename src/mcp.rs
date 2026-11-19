//! MCP server ผ่าน stdio (JSON-RPC 2.0 บรรทัดละ message) มี tool `commit_report`

use std::io::{self, BufRead, Write};
use std::path::Path;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::{VERSION, git, report};

const TOOL_NAME: &str = "commit_report";
/// เวอร์ชันโปรโตคอลที่รองรับ ตัวแรกคือล่าสุด
const PROTOCOL_VERSIONS: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ReportArgs {
    start: String,
    end: String,
    author: String,
    root: String,
    depth: i64,
}

/// ค่าเริ่มต้นของ root/depth มาจาก flag ตอนสั่งรัน server
struct Server<'a> {
    root: &'a str,
    depth: usize,
}

/// รันเป็น MCP server ผ่าน stdio จนกว่า stdin จะปิด
pub fn serve(root: &str, depth: usize) -> io::Result<()> {
    let server = Server { root, depth };
    let mut out = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(resp) = server.handle_line(&line) {
            serde_json::to_writer(&mut out, &resp)?;
            out.write_all(b"\n")?;
            out.flush()?;
        }
    }
    Ok(())
}

impl Server<'_> {
    /// คืน response หรือ `None` ถ้าเป็น notification / response จาก client
    fn handle_line(&self, line: &str) -> Option<Value> {
        let msg: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => return Some(error(&Value::Null, PARSE_ERROR, &e.to_string())),
        };
        let Some(obj) = msg.as_object() else {
            return Some(error(
                &Value::Null,
                INVALID_REQUEST,
                "expected a JSON object",
            ));
        };
        let id = obj.get("id")?;
        let method = obj.get("method").and_then(Value::as_str)?;
        let params = obj.get("params").unwrap_or(&Value::Null);
        Some(match self.dispatch(method, params) {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err((code, message)) => error(id, code, &message),
        })
    }

    fn dispatch(&self, method: &str, params: &Value) -> Result<Value, (i64, String)> {
        match method {
            "initialize" => Ok(initialize(params)),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": [tool_definition()] })),
            "tools/call" => self.call_tool(params),
            _ => Err((METHOD_NOT_FOUND, format!("method not found: {method}"))),
        }
    }

    fn call_tool(&self, params: &Value) -> Result<Value, (i64, String)> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if name != TOOL_NAME {
            return Err((INVALID_PARAMS, format!("unknown tool: {name}")));
        }
        let args = match params.get("arguments") {
            None | Some(Value::Null) => ReportArgs::default(),
            Some(v) => ReportArgs::deserialize(v)
                .map_err(|e| (INVALID_PARAMS, format!("invalid arguments: {e}")))?,
        };
        // error จาก tool ส่งกลับเป็นผลลัพธ์ที่มี isError ให้ model เห็นและแก้ได้
        let (text, is_error) = match self.run_report(args) {
            Ok(text) => (text, false),
            Err(err) => (err, true),
        };
        Ok(json!({ "content": [{ "type": "text", "text": text }], "isError": is_error }))
    }

    fn run_report(&self, a: ReportArgs) -> Result<String, String> {
        let today = report::today();
        let start = or_default(a.start, &today);
        let end = or_default(a.end, &today);
        for d in [&start, &end] {
            report::validate_date(d)?;
        }
        let authors = git::resolve_authors(&a.author);
        if authors.is_empty() {
            return Err("ไม่พบ author ให้ระบุ author".to_owned());
        }
        let depth = usize::try_from(a.depth)
            .ok()
            .filter(|d| *d > 0)
            .unwrap_or(self.depth);
        let root = or_default(a.root, self.root);
        let opt = git::Options {
            start,
            end,
            authors,
        };
        let text = git::collect(&git::find_repos(Path::new(&root), depth), &opt).render(false);
        if text.is_empty() {
            return Ok(format!("ไม่มี commit ระหว่าง {} ถึง {}", opt.start, opt.end));
        }
        Ok(text)
    }
}

fn or_default(value: String, default: &str) -> String {
    if value.is_empty() {
        default.to_owned()
    } else {
        value
    }
}

fn initialize(params: &Value) -> Value {
    let requested = params.get("protocolVersion").and_then(Value::as_str);
    let version = requested
        .filter(|v| PROTOCOL_VERSIONS.contains(v))
        .unwrap_or(PROTOCOL_VERSIONS[0]);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": "commit-report", "version": VERSION },
    })
}

fn tool_definition() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "สรุป commit ของผู้ใช้จากทุก git repo ใต้ root ในช่วงวันที่กำหนด แยกตามวัน และแยก [ui]/[service]",
        "inputSchema": {
            "type": "object",
            "properties": {
                "start": { "type": "string", "description": "วันเริ่ม YYYY-MM-DD (ค่าเริ่มต้น: วันนี้)" },
                "end": { "type": "string", "description": "วันสิ้นสุด YYYY-MM-DD (ค่าเริ่มต้น: วันนี้)" },
                "author": { "type": "string", "description": "email ผู้ commit คั่นด้วย , (ค่าเริ่มต้น: git config user.email)" },
                "root": { "type": "string", "description": "โฟลเดอร์ที่จะค้นหา git repo (ค่าเริ่มต้น: -root ของ server)" },
                "depth": { "type": "integer", "description": "ความลึกสูงสุดที่ค้นหา repo (ค่าเริ่มต้น: -depth ของ server)" },
            },
            "additionalProperties": false,
        },
    })
}

fn error(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVER: Server<'static> = Server {
        root: ".",
        depth: 1,
    };

    fn args(start: &str, end: &str, author: &str) -> ReportArgs {
        ReportArgs {
            start: start.into(),
            end: end.into(),
            author: author.into(),
            ..ReportArgs::default()
        }
    }

    #[test]
    fn run_report() {
        assert!(
            SERVER.run_report(args("14/09/2026", "", "x@y")).is_err(),
            "วันที่ผิดรูปแบบต้อง error"
        );
        let text = SERVER
            .run_report(args("2000-01-01", "2000-01-01", "nobody@invalid"))
            .unwrap();
        assert!(text.starts_with("ไม่มี commit"), "{text}");
    }

    #[test]
    fn protocol() {
        let init = SERVER
            .handle_line(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#)
            .unwrap();
        assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
        assert_eq!(init["result"]["serverInfo"]["version"], VERSION);

        assert!(
            SERVER
                .handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
                .is_none()
        );

        let list = SERVER
            .handle_line(r#"{"jsonrpc":"2.0","id":"a","method":"tools/list"}"#)
            .unwrap();
        assert_eq!(list["id"], "a");
        assert_eq!(list["result"]["tools"][0]["name"], TOOL_NAME);

        let bad = SERVER
            .handle_line(r#"{"jsonrpc":"2.0","id":2,"method":"nope"}"#)
            .unwrap();
        assert_eq!(bad["error"]["code"], METHOD_NOT_FOUND);

        let call = SERVER
            .handle_line(r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"commit_report","arguments":{"start":"bad"}}}"#)
            .unwrap();
        assert_eq!(call["result"]["isError"], true);

        assert_eq!(
            SERVER.handle_line("{").unwrap()["error"]["code"],
            PARSE_ERROR
        );
    }
}
