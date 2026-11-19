//! ค้นหา repo, รัน `git log`, กรองและแปลง commit

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{LazyLock, Mutex, PoisonError};
use std::thread;

use regex::Regex;

use crate::report::{Kind, Report};

const UI_MARKERS: [&str; 3] = ["angular.json", "pubspec.yaml", "index.html"];

// `\w`, `\s`, `\b` ของ Go เป็น ASCII จึงปิด Unicode (`-u`) ให้ผลเหมือนกัน
/// type(scope)!: msg
static CONVENTIONAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?-u:\w)+(?:\(([^)]*)\))?!?:(?-u:\s)*").expect("valid regex"));

/// commit ที่แค่ bump version ไม่ต้องรายงาน
static VERSION_BUMP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^(?-u:(chore(\(release\))?:\s*)?((update|bump)\s+)?version\b)|ปรับเวอร์ชัน|อัปเดตเวอร์ชัน|^\[tag\]",
    )
    .expect("valid regex")
});

pub struct Options {
    pub start: String,
    pub end: String,
    pub authors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Commit {
    date: String,
    subject: String,
}

/// author ที่ระบุ หรือถ้าไม่ระบุใช้ git config user.email
pub fn resolve_authors(s: &str) -> Vec<String> {
    let authors = split_authors(s);
    if !authors.is_empty() {
        return authors;
    }
    Command::new("git")
        .args(["config", "--global", "user.email"])
        .output()
        .map(|out| split_authors(&String::from_utf8_lossy(&out.stdout)))
        .unwrap_or_default()
}

fn split_authors(s: &str) -> Vec<String> {
    s.split(',')
        .map(str::trim)
        .filter(|a| !a.is_empty())
        .map(str::to_owned)
        .collect()
}

/// รัน git log ทุก repo พร้อมกัน จำกัดจำนวน thread ตาม CPU
pub fn collect(repos: &[PathBuf], opt: &Options) -> Report {
    let report = Mutex::new(Report::default());
    let next = AtomicUsize::new(0);
    let workers = thread::available_parallelism()
        .map_or(1, usize::from)
        .min(repos.len());
    thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                while let Some(repo) = repos.get(next.fetch_add(1, Ordering::Relaxed)) {
                    collect_repo(repo, opt, &report);
                }
            });
        }
    });
    report.into_inner().unwrap_or_else(PoisonError::into_inner)
}

fn collect_repo(repo: &Path, opt: &Options, report: &Mutex<Report>) {
    let commits = git_log(repo, opt);
    if commits.is_empty() {
        return;
    }
    let (kind, name) = (repo_kind(repo), repo_name(repo));
    let mut r = report.lock().unwrap_or_else(PoisonError::into_inner);
    for c in commits {
        r.add(&c.date, kind, &name, c.subject);
    }
}

/// หา repo ใต้ root ไม่เข้า `node_modules` และไม่ลงไปใน repo ที่เจอแล้ว
pub fn find_repos(root: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut repos = Vec::new();
    walk(root, 0, max_depth, &mut repos);
    repos
}

fn walk(dir: &Path, level: usize, max_depth: usize, repos: &mut Vec<PathBuf>) {
    // level 0 คือ root เอง, level n คือโฟลเดอร์ที่ลึกลงไป n ชั้น
    let is_node_modules = dir.file_name().is_some_and(|n| n == "node_modules");
    if is_node_modules || level.saturating_sub(1) >= max_depth {
        return;
    }
    if dir.join(".git").exists() {
        repos.push(dir.to_path_buf());
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut subdirs: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.path())
        .collect();
    subdirs.sort();
    for sub in subdirs {
        walk(&sub, level + 1, max_depth, repos);
    }
}

/// ชื่อโฟลเดอร์ของ repo ถ้า root เป็น `.` ใช้ชื่อจริงของโฟลเดอร์
fn repo_name(repo: &Path) -> String {
    let base = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned());
    base(repo)
        .or_else(|| fs::canonicalize(repo).ok().as_deref().and_then(base))
        .unwrap_or_else(|| repo.display().to_string())
}

/// ui ถ้าชื่อ repo มี -ui หรือเป็นโปรเจค Angular/Flutter ที่เหลือเป็น service
fn repo_kind(repo: &Path) -> Kind {
    if repo_name(repo).contains("-ui") || UI_MARKERS.iter().any(|f| repo.join(f).exists()) {
        Kind::Ui
    } else {
        Kind::Service
    }
}

fn git_log(repo: &Path, opt: &Options) -> Vec<Commit> {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(repo)
        .args(["log", "--all", "--no-merges", "--reverse"])
        .arg(format!("--since={} 00:00", opt.start))
        .arg(format!("--until={} 23:59:59", opt.end))
        .args(["--date=format:%Y-%m-%d", "--pretty=%ad%x09%s"])
        .args(opt.authors.iter().map(|a| format!("--author={a}")));
    match cmd.output() {
        Ok(out) if out.status.success() => parse_log(&String::from_utf8_lossy(&out.stdout)),
        _ => Vec::new(),
    }
}

/// แปลง output ของ git log เป็น commit ตัด version bump และ subject ซ้ำ (cherry-pick ข้าม branch)
fn parse_log(out: &str) -> Vec<Commit> {
    let mut seen = HashSet::new();
    out.lines()
        .filter_map(|line| line.split_once('\t'))
        .filter(|(_, subj)| !VERSION_BUMP.is_match(subj))
        .map(|(date, subj)| Commit {
            date: date.to_owned(),
            subject: clean_subject(subj),
        })
        .filter(|c| !c.subject.is_empty() && seen.insert(c.clone()))
        .collect()
}

/// "feat(scope): เพิ่มคอลัมน์" -> "scope เพิ่มคอลัมน์"
fn clean_subject(s: &str) -> String {
    let s = s.trim();
    let Some(caps) = CONVENTIONAL.captures(s) else {
        return s.to_owned();
    };
    let scope = caps.get(1).map_or("", |m| m.as_str().trim());
    let rest = &s[caps.get(0).map_or(0, |m| m.end())..];
    format!("{scope} {rest}").trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_log_filters_and_dedups() {
        let log = "2026-09-14\tfeat(abc1x010): เปิด sort\n\
                   2026-09-14\tfeat(abc1x010): เปิด sort\n\
                   2026-09-14\tupdate version : 1.0.81\n\
                   2026-09-14\tchore: อัปเดตเวอร์ชันเป็น 0.0.51\n\
                   2026-09-14\tfix: ปิดการแปลภาษา\n";
        let subjects: Vec<_> = parse_log(log).into_iter().map(|c| c.subject).collect();
        assert_eq!(subjects, ["abc1x010 เปิด sort", "ปิดการแปลภาษา"]);
    }

    #[test]
    fn clean_subject_cases() {
        assert_eq!(clean_subject("feat!: x"), "x");
        assert_eq!(clean_subject("fix( a ):   y"), "a y");
        assert_eq!(clean_subject("เพิ่ม: z"), "เพิ่ม: z");
        assert_eq!(clean_subject("  plain  "), "plain");
    }

    #[test]
    fn version_bump() {
        for s in [
            "bump version",
            "chore(release): version 1",
            "[TAG] 1.0",
            "ปรับเวอร์ชัน",
        ] {
            assert!(VERSION_BUMP.is_match(s), "{s}");
        }
        for s in ["versioning api", "feat: add version"] {
            assert!(!VERSION_BUMP.is_match(s), "{s}");
        }
    }

    #[test]
    fn split_authors_trims() {
        assert_eq!(split_authors(" a@x , ,b@y\n"), ["a@x", "b@y"]);
    }

    #[test]
    fn find_repos_respects_depth() {
        let root = std::env::temp_dir().join(format!("commit-report-test-{}", std::process::id()));
        for d in ["a/.git", "b/c/.git", "b/node_modules/x/.git", "d/e/f/.git"] {
            fs::create_dir_all(root.join(d)).unwrap();
        }
        let names = |depth| -> Vec<String> {
            find_repos(&root, depth)
                .iter()
                .map(|p| repo_name(p))
                .collect()
        };
        assert_eq!(names(4), ["a", "c", "f"]);
        assert_eq!(names(2), ["a", "c"]);
        assert_eq!(names(0), Vec::<String>::new());
        fs::remove_dir_all(&root).unwrap();
    }
}
