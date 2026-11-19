//! อ่าน flag แบบเดียวกับ package `flag` ของ Go: รับทั้ง `-root x`, `--root x`, `-root=x`
//! เพื่อให้คำสั่งเดิม (เช่น `commit-report -mcp -root D:/Workspace`) ใช้ต่อได้

use crate::VERSION;

const DEFAULT_DEPTH: usize = 4;

#[derive(Debug, PartialEq, Eq)]
pub struct Args {
    pub root: String,
    pub start: String,
    pub end: String,
    pub author: String,
    pub depth: usize,
    pub md: bool,
    pub mcp: bool,
    pub version: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Parsed {
    Run(Args),
    Help,
}

/// แปลง argument เป็น [`Args`] หยุดอ่านเมื่อเจอ `--` หรือ argument ที่ไม่ใช่ flag แบบเดียวกับ Go
pub fn parse(raw: impl IntoIterator<Item = String>, today: &str) -> Result<Parsed, String> {
    let mut args = Args {
        root: ".".to_owned(),
        start: today.to_owned(),
        end: today.to_owned(),
        author: String::new(),
        depth: DEFAULT_DEPTH,
        md: false,
        mcp: false,
        version: false,
    };
    let mut it = raw.into_iter();
    while let Some(arg) = it.next() {
        let Some(flag) = flag_body(&arg) else { break };
        let (name, inline) = match flag.split_once('=') {
            Some((n, v)) => (n, Some(v.to_owned())),
            None => (flag, None),
        };
        match name {
            "h" | "help" => return Ok(Parsed::Help),
            "md" | "mcp" | "version" => set_bool(&mut args, name, inline.as_deref())?,
            "root" | "start" | "end" | "author" | "depth" => {
                let value = inline
                    .or_else(|| it.next())
                    .ok_or_else(|| format!("flag needs an argument: -{name}"))?;
                set_value(&mut args, name, value)?;
            }
            _ => return Err(format!("flag provided but not defined: -{name}")),
        }
    }
    Ok(Parsed::Run(args))
}

/// ตัด `-`/`--` ออก คืน `None` ถ้าไม่ใช่ flag
fn flag_body(arg: &str) -> Option<&str> {
    if arg == "--" {
        return None;
    }
    let body = arg.strip_prefix("--").or_else(|| arg.strip_prefix('-'))?;
    (!body.is_empty() && !body.starts_with(['-', '='])).then_some(body)
}

fn set_bool(args: &mut Args, name: &str, value: Option<&str>) -> Result<(), String> {
    let v = match value.unwrap_or("true") {
        "1" | "t" | "T" | "true" | "TRUE" | "True" => true,
        "0" | "f" | "F" | "false" | "FALSE" | "False" => false,
        other => return Err(format!("invalid boolean value {other:?} for -{name}")),
    };
    match name {
        "md" => args.md = v,
        "mcp" => args.mcp = v,
        _ => args.version = v,
    }
    Ok(())
}

fn set_value(args: &mut Args, name: &str, value: String) -> Result<(), String> {
    match name {
        "root" => args.root = value,
        "start" => args.start = value,
        "end" => args.end = value,
        "author" => args.author = value,
        _ => {
            args.depth = value
                .parse()
                .map_err(|_| format!("invalid value {value:?} for flag -{name}"))?;
        }
    }
    Ok(())
}

pub fn usage(today: &str) -> String {
    format!(
        "commit-report {VERSION}
สรุป commit ของตัวเองจากทุก git repo ใต้ -root แยกตามวัน และแยกเป็น [ui]/[service]

ตัวอย่าง:
  commit-report -root D:/Workspace -start 2026-09-14 -end 2026-10-02
  commit-report -mcp -root D:/Workspace

flags:
  -author string
    \temail ผู้ commit คั่นด้วย , (ค่าเริ่มต้น: git config user.email)
  -depth int
    \tความลึกสูงสุดที่ค้นหา repo (default {DEFAULT_DEPTH})
  -end string
    \tวันสิ้นสุด (YYYY-MM-DD) (default \"{today}\")
  -mcp
    \tรันเป็น MCP server ผ่าน stdio
  -md
    \tครอบแต่ละวันด้วย code block สำหรับ copy
  -root string
    \tโฟลเดอร์ที่จะค้นหา git repo (default \".\")
  -start string
    \tวันเริ่ม (YYYY-MM-DD) (default \"{today}\")
  -version
    \tแสดงเวอร์ชันแล้วจบ
"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODAY: &str = "2026-11-20";

    fn run(raw: &[&str]) -> Result<Parsed, String> {
        parse(raw.iter().map(|s| (*s).to_owned()), TODAY)
    }

    fn run_args(raw: &[&str]) -> Args {
        match run(raw) {
            Ok(Parsed::Run(args)) => args,
            other => panic!("parse({raw:?}) = {other:?}"),
        }
    }

    #[test]
    fn defaults() {
        let a = run_args(&[]);
        assert_eq!(
            (a.root.as_str(), a.start.as_str(), a.depth),
            (".", TODAY, 4)
        );
        assert!(!a.md && !a.mcp && !a.version);
    }

    #[test]
    fn go_style_flags() {
        let a = run_args(&[
            "-mcp",
            "-root",
            "D:/W",
            "--start=2026-09-14",
            "-depth=2",
            "-md=false",
        ]);
        assert!(a.mcp && !a.md);
        assert_eq!(
            (a.root.as_str(), a.start.as_str(), a.depth),
            ("D:/W", "2026-09-14", 2)
        );
    }

    #[test]
    fn stops_at_positional() {
        assert!(!run_args(&["x", "-mcp"]).mcp);
        assert!(!run_args(&["--", "-mcp"]).mcp);
    }

    #[test]
    fn errors_and_help() {
        assert_eq!(run(&["-h"]), Ok(Parsed::Help));
        assert!(run(&["-nope"]).is_err());
        assert!(run(&["-root"]).is_err());
        assert!(run(&["-depth", "x"]).is_err());
    }
}
