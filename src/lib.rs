use std::ffi::OsString;
use std::io::Write;
use std::path::Path;

const USAGE: &str = "Usage: av scan";
pub(crate) const GIT_SOURCE: &str = "isotope:git";
pub(crate) const HIGH: &str = "high";

mod isotopes;

#[derive(Debug, PartialEq, Eq)]
struct Finding {
    source: &'static str,
    severity: &'static str,
    message: String,
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
        (Some(command), None) if command == "scan" => scan(stdout),
        _ => {
            let _ = writeln!(stderr, "{USAGE}");
            2
        }
    }
}

fn scan<W: Write>(stdout: &mut W) -> i32 {
    let findings = scan_home(home());
    print_scan(stdout, &findings);
    0
}

fn home() -> OsString {
    std::env::var_os("HOME").unwrap_or_default()
}

fn scan_home(home: impl AsRef<Path>) -> Vec<Finding> {
    isotopes::findings(home.as_ref())
}

fn print_scan<W: Write>(stdout: &mut W, findings: &[Finding]) {
    let _ = writeln!(stdout, "Automic Vault scan");
    if findings.is_empty() {
        let _ = writeln!(stdout, "No problems found.");
        return;
    }

    let _ = writeln!(stdout, "Findings:");
    for (index, finding) in findings.iter().enumerate() {
        let _ = writeln!(
            stdout,
            "{}. {} {} - {}",
            index + 1,
            finding.severity,
            finding.source,
            finding.message
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};

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
        assert_eq!(stdout, "Automic Vault scan\nNo problems found.\n");
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

    #[test]
    fn scan_home_aggregates_git_findings() {
        let home = temp_home("aggregate");
        fs::write(
            home.join(".git-credentials"),
            "https://user:token@example.com\n",
        )
        .unwrap();

        assert_eq!(
            scan_home(&home),
            vec![Finding {
                source: GIT_SOURCE,
                severity: HIGH,
                message: isotopes::git_credentials::PLAINTEXT_GIT_CREDENTIALS.to_string(),
            }]
        );

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn print_scan_displays_findings() {
        let mut stdout = Vec::new();

        print_scan(
            &mut stdout,
            &[Finding {
                source: GIT_SOURCE,
                severity: HIGH,
                message: isotopes::git_credentials::PLAINTEXT_GIT_CREDENTIALS.to_string(),
            }],
        );

        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "Automic Vault scan\nFindings:\n1. high isotope:git - Git credential store contains plaintext credentials\n"
        );
    }

    fn temp_home(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("av-{label}-{}-{nanos}", process::id()));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
