use std::path::{Path, PathBuf};

use crate::{AffectedFile, Finding};

mod config;
mod credential_fill;
mod credential_oauth;
pub(crate) mod credentials_file;

pub(crate) const NAME: &str = "git";
pub(crate) const HOMEPAGE: &str = "https://git-scm.com/";
pub(crate) const HIGH: &str = "high";
pub(crate) const DOCS_URL: &str =
    "https://github.com/automic-vault/automic-vault/main/docs/securing-git.md";

const DETECTORS: &[fn(&Path) -> Vec<Finding>] = &[
    credentials_file::findings,
    credential_fill::findings,
    credential_oauth::findings,
];

pub(crate) fn findings(home: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    for detector in DETECTORS {
        findings.extend(detector(home));
    }
    findings
}

fn high(
    explanation: impl Into<String>,
    solution: impl Into<String>,
    affected: Vec<AffectedFile>,
) -> Finding {
    Finding {
        source: NAME,
        homepage: HOMEPAGE,
        severity: HIGH,
        explanation: explanation.into(),
        solution: solution.into(),
        affected,
        docs_url: DOCS_URL,
    }
}

fn high_unattributed(explanation: impl Into<String>, solution: impl Into<String>) -> Finding {
    high(explanation, solution, Vec::new())
}

fn affected(path: &Path, line: usize) -> AffectedFile {
    AffectedFile {
        path: path.display().to_string(),
        line,
    }
}

fn git_config_paths(home: &Path) -> Vec<PathBuf> {
    let mut paths = vec![home.join(".gitconfig")];
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty())
    {
        paths.push(PathBuf::from(config_home).join("git/config"));
    } else {
        paths.push(home.join(".config/git/config"));
    }
    paths.sort();
    paths.dedup();
    paths
}

fn read_to_string(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scan_runs_every_registered_detector() {
        let home = temp_home("all-detectors");
        fs::write(
            home.join(".git-credentials"),
            "https://user:token@example.com\n",
        )
        .unwrap();
        fs::write(
            home.join(".gitconfig"),
            "[credential \"https://github.com\"]\n\
             helper = !gh auth git-credential\n\
             helper = oauth -device\n\
             oauthClientSecret = abcdefgh\n",
        )
        .unwrap();

        let messages = findings(&home)
            .into_iter()
            .map(|finding| finding.explanation)
            .collect::<Vec<_>>();

        assert_eq!(DETECTORS.len(), 3);
        assert!(
            messages
                .iter()
                .any(|message| message == credentials_file::PLAINTEXT_GIT_CREDENTIALS)
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("gh auth git-credential"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("git-credential-oauth"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("OAuth client secret"))
        );

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn every_detector_report_has_required_security_fields() {
        let home = temp_home("required-fields");
        fs::write(
            home.join(".git-credentials"),
            "https://user:token@example.com\n",
        )
        .unwrap();
        fs::write(
            home.join(".gitconfig"),
            "[credential \"https://github.com\"]\n\
             helper = !gh auth git-credential\n\
             helper = oauth -device\n\
             oauthClientSecret = abcdefgh\n",
        )
        .unwrap();

        for finding in findings(&home) {
            assert_eq!(finding.source, NAME);
            assert_eq!(finding.homepage, HOMEPAGE);
            assert_eq!(finding.severity, HIGH);
            assert!(!finding.explanation.is_empty());
            assert!(!finding.solution.is_empty());
            assert_eq!(finding.docs_url, DOCS_URL);
            for affected in finding.affected {
                assert!(!affected.path.is_empty());
                assert!(affected.line > 0);
            }
        }

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn module_metadata_names_git_without_isotope_prefix() {
        assert_eq!(NAME, "git");
        assert_eq!(HOMEPAGE, "https://git-scm.com/");
    }

    fn temp_home(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "av-git-isotope-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
