use std::fs;
use std::path::Path;

use crate::{Finding, GIT_SOURCE, HIGH};

pub(crate) const PLAINTEXT_GIT_CREDENTIALS: &str =
    "Git credential store contains plaintext credentials";

pub(crate) fn findings(home: &Path) -> Vec<Finding> {
    fs::read_to_string(home.join(".git-credentials"))
        .unwrap_or_default()
        .contains("://")
        .then(|| Finding {
            source: GIT_SOURCE,
            severity: HIGH,
            message: PLAINTEXT_GIT_CREDENTIALS,
        })
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
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

        assert_eq!(
            findings(&home),
            vec![Finding {
                source: GIT_SOURCE,
                severity: HIGH,
                message: PLAINTEXT_GIT_CREDENTIALS,
            }]
        );

        let _ = fs::remove_dir_all(home);
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
