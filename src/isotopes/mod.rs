use std::path::{Path, PathBuf};

use crate::{Finding, GIT_SOURCE, HIGH};

mod git_config;
mod git_credential_fill;
mod git_credential_oauth;
pub(crate) mod git_credentials_file;

const DETECTORS: &[fn(&Path) -> Vec<Finding>] = &[
    git_credentials_file::findings,
    git_credential_fill::findings,
    git_credential_oauth::findings,
];

pub(crate) fn findings(home: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    for detector in DETECTORS {
        findings.extend(detector(home));
    }
    findings
}

fn high(message: impl Into<String>) -> Finding {
    Finding {
        source: GIT_SOURCE,
        severity: HIGH,
        message: message.into(),
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
            .map(|finding| finding.message)
            .collect::<Vec<_>>();

        assert_eq!(DETECTORS.len(), 3);
        assert!(
            messages
                .iter()
                .any(|message| message == git_credentials_file::PLAINTEXT_GIT_CREDENTIALS)
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

    fn temp_home(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "av-isotopes-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
