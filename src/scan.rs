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
        let _ = writeln!(stdout, "✓ No problems found.");
        return;
    }

    let _ = writeln!(stdout, "⚠ Findings: {}", findings.len());
    for (index, finding) in findings.iter().enumerate() {
        let _ = writeln!(stdout);
        let _ = writeln!(stdout, "{}. {}", index + 1, finding.source);
        let _ = writeln!(stdout, "   Severity: {}", finding.severity);
        let _ = writeln!(stdout, "   Problem: {}", finding.explanation);
        let _ = writeln!(stdout, "   Affected files:");
        if finding.affected.is_empty() {
            let _ = writeln!(stdout, "     not reported by this detector");
        } else {
            for affected in &finding.affected {
                let _ = writeln!(stdout, "     {}:{}", affected.path, affected.line);
            }
        }
        let _ = writeln!(stdout, "   Read more: {}", finding.docs_url);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isotopes::{GIT_DOCS_URL, GIT_SOURCE, HIGH};
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
                explanation: isotopes::git_credentials_file::PLAINTEXT_GIT_CREDENTIALS.to_string(),
                affected: vec![crate::AffectedFile {
                    path: home.join(".git-credentials").display().to_string(),
                    line: 1,
                }],
                docs_url: GIT_DOCS_URL,
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
                explanation: isotopes::git_credentials_file::PLAINTEXT_GIT_CREDENTIALS.to_string(),
                affected: vec![crate::AffectedFile {
                    path: "/tmp/home/.git-credentials".to_string(),
                    line: 1,
                }],
                docs_url: GIT_DOCS_URL,
            }],
        );

        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "Automic Vault scan\n⚠ Findings: 1\n\n1. isotope:git\n   Severity: high\n   Problem: Git credential store contains plaintext credentials\n   Affected files:\n     /tmp/home/.git-credentials:1\n   Read more: https://github.com/automic-vault/automic-vault/main/docs/securing-git.md\n"
        );
    }

    #[test]
    fn print_displays_unattributed_findings_without_fake_file_location() {
        let mut stdout = Vec::new();

        print(
            &mut stdout,
            &[Finding {
                source: GIT_SOURCE,
                severity: HIGH,
                explanation: "Git credential helper exposes a GitHub token".to_string(),
                affected: Vec::new(),
                docs_url: GIT_DOCS_URL,
            }],
        );

        assert!(
            String::from_utf8(stdout)
                .unwrap()
                .contains("     not reported by this detector\n")
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
