use std::ffi::OsString;
use std::io::{IsTerminal, Write};

const USAGE: &str = "Usage: av scan";

mod isotopes;
mod scan;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Finding {
    source: &'static str,
    homepage: &'static str,
    severity: &'static str,
    explanation: String,
    solution: String,
    affected: Vec<AffectedFile>,
    docs_url: &'static str,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AffectedFile {
    path: String,
    line: usize,
}

pub fn run<I, W, E>(args: I, stdout: &mut W, stderr: &mut E) -> i32
where
    I: IntoIterator<Item = OsString>,
    W: Write,
    E: Write,
{
    run_with_style(args, stdout, stderr, scan::Style::plain())
}

pub fn run_terminal<I>(args: I) -> i32
where
    I: IntoIterator<Item = OsString>,
{
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    let color = stdout.is_terminal() && color_enabled();
    run_with_style(args, &mut stdout, &mut stderr, scan::Style { color })
}

fn run_with_style<I, W, E>(args: I, stdout: &mut W, stderr: &mut E, style: scan::Style) -> i32
where
    I: IntoIterator<Item = OsString>,
    W: Write,
    E: Write,
{
    let mut args = args.into_iter();
    let _program = args.next();

    match (args.next(), args.next()) {
        (Some(command), None) if command == "scan" => scan::run(stdout, style),
        _ => {
            let _ = writeln!(stderr, "{USAGE}");
            2
        }
    }
}

fn color_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none()
        && std::env::var_os("TERM").is_none_or(|term| term != "dumb")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_args(args: &[&str]) -> (i32, String, String) {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = run(args.iter().map(OsString::from), &mut stdout, &mut stderr);
        (
            code,
            String::from_utf8(stdout).unwrap(),
            String::from_utf8(stderr).unwrap(),
        )
    }

    #[test]
    fn scan_prints_clean_report() {
        let (code, stdout, stderr) = run_args(&["av", "scan"]);

        assert_eq!(code, 0);
        assert!(stdout.starts_with("Automic Vault scan\n"));
        assert_eq!(stderr, "");
    }

    #[test]
    fn only_scan_is_supported() {
        for args in [&["av"][..], &["av", "harden"], &["av", "scan", "--json"]] {
            let (code, stdout, stderr) = run_args(args);
            assert_eq!(code, 2);
            assert_eq!(stdout, "");
            assert_eq!(stderr, "Usage: av scan\n");
        }
    }
}
