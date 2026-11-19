//! จัดกลุ่ม commit ตามวัน/ประเภท และ render ผลลัพธ์

use std::collections::BTreeMap;
use std::fmt::Write;

use chrono::{Datelike, Local, NaiveDate};

const DATE_FORMAT: &str = "%Y-%m-%d";
const THAI_MONTHS: [&str; 12] = [
    "ม.ค.",
    "ก.พ.",
    "มี.ค.",
    "เม.ย.",
    "พ.ค.",
    "มิ.ย.",
    "ก.ค.",
    "ส.ค.",
    "ก.ย.",
    "ต.ค.",
    "พ.ย.",
    "ธ.ค.",
];

/// ลำดับของ variant คือลำดับที่แสดงผล: ui ก่อน service
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Ui,
    Service,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ui => "ui",
            Self::Service => "service",
        }
    }
}

/// date -> kind -> repo -> subjects (`BTreeMap` จึงเรียงตามวันและชื่อ repo อยู่แล้ว)
#[derive(Debug, Default)]
pub struct Report(BTreeMap<String, BTreeMap<Kind, BTreeMap<String, Vec<String>>>>);

impl Report {
    pub fn add(&mut self, date: &str, kind: Kind, repo: &str, subject: String) {
        self.0
            .entry(date.to_owned())
            .or_default()
            .entry(kind)
            .or_default()
            .entry(repo.to_owned())
            .or_default()
            .push(subject);
    }

    pub fn render(&self, md: bool) -> String {
        let fence = if md { "```\n" } else { "" };
        let mut b = String::new();
        for (date, kinds) in &self.0 {
            b.push_str(&thai_date(date));
            b.push('\n');
            b.push_str(fence);
            for (kind, repos) in kinds {
                // เขียนลง String ไม่มีทาง error
                let _ = writeln!(b, "[{}]", kind.as_str());
                for (name, subjects) in repos {
                    let _ = writeln!(b, "- {name}: {}", subjects.join(" · "));
                }
            }
            b.push_str(fence);
            b.push('\n');
        }
        b
    }
}

pub fn today() -> String {
    Local::now().date_naive().format(DATE_FORMAT).to_string()
}

fn parse_date(d: &str) -> Option<NaiveDate> {
    // ต้องเป็น YYYY-MM-DD เต็ม 10 ตัว เหมือน time.DateOnly ของ Go
    if d.len() != 10 {
        return None;
    }
    NaiveDate::parse_from_str(d, DATE_FORMAT).ok()
}

pub fn validate_date(d: &str) -> Result<(), String> {
    parse_date(d)
        .map(|_| ())
        .ok_or_else(|| format!("วันที่ {d:?} ต้องเป็น YYYY-MM-DD"))
}

fn thai_date(d: &str) -> String {
    parse_date(d).map_or_else(
        || d.to_owned(),
        |t| format!("{} {}", t.day(), THAI_MONTHS[t.month0() as usize]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render() {
        let mut r = Report::default();
        r.add("2026-09-14", Kind::Service, "app-api", "a".into());
        r.add("2026-09-14", Kind::Ui, "app-ui", "b".into());
        r.add("2026-09-14", Kind::Ui, "app-ui", "c".into());
        let want = "14 ก.ย.\n[ui]\n- app-ui: b · c\n[service]\n- app-api: a\n\n";
        assert_eq!(r.render(false), want);

        let mut r = Report::default();
        r.add("2026-10-02", Kind::Ui, "x", "y".into());
        assert!(r.render(true).contains("```"), "md ไม่มี ```");
    }

    #[test]
    fn dates() {
        assert_eq!(thai_date("2026-01-05"), "5 ม.ค.");
        assert_eq!(thai_date("bad"), "bad");
        assert!(validate_date("2026-02-28").is_ok());
        for bad in ["2026-02-30", "2026-9-14", "14/09/2026", ""] {
            assert!(validate_date(bad).is_err(), "{bad}");
        }
    }
}
