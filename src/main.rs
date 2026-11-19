//! commit-report: สรุป commit ของตัวเองจากทุก git repo ใต้ root แยกตามวัน และแยกเป็น [ui]/[service]

mod cli;
mod git;
mod mcp;
mod report;

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use cli::{Args, Parsed};

/// เวอร์ชันมาจาก `version` ใน Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    let today = report::today();
    match cli::parse(std::env::args().skip(1), &today) {
        Ok(Parsed::Help) => {
            eprint!("{}", cli::usage(&today));
            ExitCode::SUCCESS
        }
        Ok(Parsed::Run(args)) => run(&args),
        Err(err) => {
            eprintln!("{err}");
            eprint!("{}", cli::usage(&today));
            ExitCode::from(2)
        }
    }
}

fn run(args: &Args) -> ExitCode {
    if args.version {
        println!("{VERSION}");
        return ExitCode::SUCCESS;
    }
    if args.mcp {
        return exit_on_error(mcp::serve(&args.root, args.depth).map_err(|e| e.to_string()));
    }
    exit_on_error(print_report(args))
}

fn print_report(args: &Args) -> Result<(), String> {
    for date in [&args.start, &args.end] {
        report::validate_date(date)?;
    }
    let authors = git::resolve_authors(&args.author);
    if authors.is_empty() {
        return Err("ไม่พบ author ใช้ -author".to_owned());
    }
    let opt = git::Options {
        start: args.start.clone(),
        end: args.end.clone(),
        authors,
    };
    let repos = git::find_repos(Path::new(&args.root), args.depth);
    let text = git::collect(&repos, &opt).render(args.md);
    std::io::stdout()
        .lock()
        .write_all(text.as_bytes())
        .map_err(|e| e.to_string())
}

fn exit_on_error(result: Result<(), String>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}
