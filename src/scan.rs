use std::ffi::OsString;
use std::io::Write;
use std::path::Path;

use crate::{Finding, isotopes};

pub(crate) fn run<W: Write>(stdout: &mut W) -> i32 {
    let findings = scan_home(home());
    print(stdout, &findings);
    0
}

fn home() -> OsString {
    std::env::var_os("HOME").unwrap_or_default()
}

fn scan_home(home: impl AsRef<Path>) -> Vec<Finding> {
    isotopes::findings(home.as_ref())
}

fn print<W: Write>(stdout: &mut W, findings: &[Finding]) {
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
    use crate::{GIT_SOURCE, HIGH};
    use std::fs;
    use std::path::PathBuf;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};

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
                message: isotopes::git_credentials_file::PLAINTEXT_GIT_CREDENTIALS.to_string(),
            }]
        );

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn print_displays_findings() {
        let mut stdout = Vec::new();

        print(
            &mut stdout,
            &[Finding {
                source: GIT_SOURCE,
                severity: HIGH,
                message: isotopes::git_credentials_file::PLAINTEXT_GIT_CREDENTIALS.to_string(),
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
