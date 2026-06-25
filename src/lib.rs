use std::ffi::OsString;
use std::io::Write;

const USAGE: &str = "Usage: av scan";

mod isotopes;
mod scan;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Finding {
    source: &'static str,
    severity: &'static str,
    explanation: String,
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
    let mut args = args.into_iter();
    let _program = args.next();

    match (args.next(), args.next()) {
        (Some(command), None) if command == "scan" => scan::run(stdout),
        _ => {
            let _ = writeln!(stderr, "{USAGE}");
            2
        }
    }
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
