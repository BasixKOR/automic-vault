use std::path::{Path, PathBuf};

use crate::Finding;

use super::{git_config, git_config_paths, high, read_to_string};

pub(crate) const PLAINTEXT_GIT_CREDENTIALS: &str =
    "Git credential store contains plaintext credentials";

pub(crate) fn findings(home: &Path) -> Vec<Finding> {
    credential_store_paths(home)
        .into_iter()
        .filter(|path| {
            path.exists()
                && read_to_string(path)
                    .as_deref()
                    .is_some_and(credential_file_contains_plaintext_secret)
        })
        .map(|path| {
            if path == home.join(".git-credentials") {
                high(PLAINTEXT_GIT_CREDENTIALS)
            } else {
                high(format!(
                    "Git credential store contains plaintext credentials: {}",
                    path.display()
                ))
            }
        })
        .collect()
}

fn credential_store_paths(home: &Path) -> Vec<PathBuf> {
    let mut paths = vec![home.join(".git-credentials")];
    for config in git_config_paths(home) {
        let Some(contents) = read_to_string(&config) else {
            continue;
        };
        paths.extend(git_config::store_paths(home, &contents));
    }
    paths.sort();
    paths.dedup();
    paths
}

fn credential_file_contains_plaintext_secret(contents: &str) -> bool {
    contents.lines().any(|line| {
        let trimmed = line.trim();
        !trimmed.is_empty() && !trimmed.starts_with('#') && url_contains_userinfo_secret(trimmed)
    })
}

fn url_contains_userinfo_secret(value: &str) -> bool {
    let Some(rest) = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
    else {
        return false;
    };
    let Some(userinfo_end) = rest.find('@') else {
        return false;
    };
    let host_end = rest.find('/').unwrap_or(rest.len());
    if userinfo_end >= host_end {
        return false;
    }
    rest[..userinfo_end]
        .split_once(':')
        .is_some_and(|(_, password)| !password.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn missing_git_credentials_file_is_clean() {
        let home = temp_home("missing");

        assert!(findings(&home).is_empty());

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn git_credentials_without_url_are_clean() {
        let home = temp_home("clean");
        fs::write(home.join(".git-credentials"), "not a credential url\n").unwrap();

        assert!(findings(&home).is_empty());

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn git_credentials_with_url_trigger_finding() {
        let home = temp_home("triggered");
        fs::write(
            home.join(".git-credentials"),
            "https://user:token@example.com\n",
        )
        .unwrap();

        assert_eq!(findings(&home), vec![high(PLAINTEXT_GIT_CREDENTIALS)]);

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn configured_store_file_triggers_finding() {
        let home = temp_home("custom");
        fs::write(
            home.join(".gitconfig"),
            "[credential]\nhelper = store --file ~/.custom-git-credentials\n",
        )
        .unwrap();
        fs::write(
            home.join(".custom-git-credentials"),
            "https://user:token@example.com\n",
        )
        .unwrap();

        let findings = findings(&home);

        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("custom-git-credentials"));

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn plaintext_secret_requires_userinfo_password() {
        assert!(credential_file_contains_plaintext_secret(
            "https://user:secret@example.com/repo.git\n"
        ));
        assert!(!credential_file_contains_plaintext_secret(
            "https://example.com/repo.git\n"
        ));
        assert!(!credential_file_contains_plaintext_secret(
            "https://user@example.com/repo.git\n"
        ));
    }

    fn temp_home(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("av-git-{label}-{}-{nanos}", process::id()));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
