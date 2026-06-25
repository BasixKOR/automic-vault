use std::ffi::OsString;
use std::io::Write;
use std::path::Path;

use crate::{Finding, isotopes};

#[derive(Clone, Copy)]
pub(crate) struct Style {
    pub(crate) color: bool,
}

impl Style {
    pub(crate) fn plain() -> Self {
        Self { color: false }
    }

    fn paint(self, code: &str, text: impl AsRef<str>) -> String {
        let text = text.as_ref();
        if self.color {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }
}

pub(crate) fn run<W: Write>(stdout: &mut W, style: Style) -> i32 {
    let findings = scan_home(home());
    print(stdout, &findings, style);
    0
}

fn home() -> OsString {
    std::env::var_os("HOME").unwrap_or_default()
}

fn scan_home(home: impl AsRef<Path>) -> Vec<Finding> {
    isotopes::findings(home.as_ref())
}

fn print<W: Write>(stdout: &mut W, findings: &[Finding], style: Style) {
    let _ = writeln!(stdout, "{}", style.paint("1;36", "Automic Vault scan"));
    let _ = writeln!(
        stdout,
        "╭─ {}",
        style.paint("36", "credential exposure audit")
    );
    let _ = writeln!(stdout, "│");
    if findings.is_empty() {
        let _ = writeln!(
            stdout,
            "◇ {}",
            style.paint("32", "No plaintext credential paths found")
        );
        let _ = writeln!(stdout, "│");
        let _ = writeln!(stdout, "╰─ {}", style.paint("2", "vault sealed"));
        return;
    }

    let finding_summary = if findings.len() == 1 {
        "1 finding requires attention".to_string()
    } else {
        format!("{} findings require attention", findings.len())
    };
    let _ = writeln!(stdout, "◆ {}", style.paint("33", finding_summary));
    let _ = writeln!(stdout, "│");
    for (index, finding) in findings.iter().enumerate() {
        let branch = if index + 1 == findings.len() {
            "└"
        } else {
            "├"
        };
        let _ = writeln!(
            stdout,
            "{branch}─ {} {}",
            style.paint("1", format!("{}.", index + 1)),
            style.paint("1;35", finding.source)
        );
        let _ = writeln!(
            stdout,
            "│  {} {}",
            style.paint("2", "severity"),
            style.paint("31;1", finding.severity.to_ascii_uppercase())
        );
        let _ = writeln!(
            stdout,
            "│  {} {}",
            style.paint("2", "homepage"),
            finding.homepage
        );
        let _ = writeln!(stdout, "│");
        let _ = writeln!(stdout, "│  {}", style.paint("1", "problem"));
        let _ = writeln!(stdout, "│  {}", finding.explanation);
        let _ = writeln!(stdout, "│");
        let _ = writeln!(stdout, "│  {}", style.paint("1", "affected files"));
        if finding.affected.is_empty() {
            let _ = writeln!(stdout, "│  • not reported by this detector");
        } else {
            for affected in &finding.affected {
                let _ = writeln!(
                    stdout,
                    "│  • {}",
                    style.paint("36", format!("{}:{}", affected.path, affected.line))
                );
            }
        }
        let _ = writeln!(stdout, "│");
        let _ = writeln!(stdout, "│  {}", style.paint("1", "read more"));
        let _ = writeln!(stdout, "│  {}", finding.docs_url);
        let _ = writeln!(stdout, "│");
    }
    let _ = writeln!(stdout, "╰─ {}", style.paint("2", "scan complete"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isotopes::git::{self, DOCS_URL, HIGH, HOMEPAGE, NAME};
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
                source: NAME,
                homepage: HOMEPAGE,
                severity: HIGH,
                explanation: git::credentials_file::PLAINTEXT_GIT_CREDENTIALS.to_string(),
                affected: vec![crate::AffectedFile {
                    path: home.join(".git-credentials").display().to_string(),
                    line: 1,
                }],
                docs_url: DOCS_URL,
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
                source: NAME,
                homepage: HOMEPAGE,
                severity: HIGH,
                explanation: git::credentials_file::PLAINTEXT_GIT_CREDENTIALS.to_string(),
                affected: vec![crate::AffectedFile {
                    path: "/tmp/home/.git-credentials".to_string(),
                    line: 1,
                }],
                docs_url: DOCS_URL,
            }],
            Style::plain(),
        );

        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "Automic Vault scan\n╭─ credential exposure audit\n│\n◆ 1 finding requires attention\n│\n└─ 1. git\n│  severity HIGH\n│  homepage https://git-scm.com/\n│\n│  problem\n│  Git credential store contains plaintext credentials\n│\n│  affected files\n│  • /tmp/home/.git-credentials:1\n│\n│  read more\n│  https://github.com/automic-vault/automic-vault/main/docs/securing-git.md\n│\n╰─ scan complete\n"
        );
    }

    #[test]
    fn print_displays_unattributed_findings_without_fake_file_location() {
        let mut stdout = Vec::new();

        print(
            &mut stdout,
            &[Finding {
                source: NAME,
                homepage: HOMEPAGE,
                severity: HIGH,
                explanation: "Git credential helper exposes a GitHub token".to_string(),
                affected: Vec::new(),
                docs_url: DOCS_URL,
            }],
            Style::plain(),
        );

        assert!(
            String::from_utf8(stdout)
                .unwrap()
                .contains("│  • not reported by this detector\n")
        );
    }

    #[test]
    fn styled_output_uses_ansi() {
        let mut stdout = Vec::new();

        print(&mut stdout, &[], Style { color: true });

        assert!(
            String::from_utf8(stdout)
                .unwrap()
                .starts_with("\x1b[1;36mAutomic Vault scan\x1b[0m\n")
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
