//! Security check: Git plaintext credential stores.
//!
//! What this detects:
//! - The default Git credential-store file at `$HOME/.git-credentials`.
//! - Additional `credential.helper = store` files declared in `$HOME/.gitconfig`
//!   or the XDG Git config path.
//! - HTTP(S) credential URLs that include non-empty userinfo passwords, such as
//!   `https://user:token@example.com/repo.git`.
//!
//! Why this matters:
//! - Git's `store` helper writes credentials to plaintext files.
//! - Agent subprocesses, editors, shell tools, malware running as the same user,
//!   and accidental logs can read these files without a keychain prompt.
//! - GitHub, GitLab, and internal Git tokens commonly have repository read/write
//!   authority, so plaintext exposure is treated as high severity.
//!
//! Evidence used:
//! - File existence is not enough; the file must contain an HTTP(S) URL with a
//!   `user:password@host` component.
//! - Blank lines, comments, host-only URLs, and username-only URLs are ignored.
//! - Custom store paths are resolved from Git config, including `--file path`,
//!   `--file=path`, `~`, and `~/...`.
//!
//! Known issues:
//! - This is a conservative parser, not a full Git config implementation.
//! - It does not understand every possible quoting or shell expansion accepted
//!   by Git helper configuration.
//! - It reports the file path for configured stores, but not the credential
//!   value or affected host, to avoid printing secrets.
//!
//! Known omissions:
//! - Non-HTTP credential formats are not inspected.
//! - Credentials stored by other helpers are handled by separate detectors.
//! - Repository-local `.git/config` files are not scanned yet.
//! - Included Git config files (`include.path`, `includeIf`) are not followed.
//!
//! Safety notes:
//! - This detector only reads files under the supplied home/config paths.
//! - Missing or unreadable files are treated as clean to avoid noisy scans.
//! - No credential material is returned in findings.

use std::path::{Path, PathBuf};

use crate::{AffectedFile, Finding};

use super::config::{self, git_config_paths, read_to_string};

const NAME: &str = "git-credentials-file";
const DOCS_URL: &str =
    "https://github.com/automic-vault/radioisotopes/tree/main/git-credentials-file";

pub(crate) const PLAINTEXT_GIT_CREDENTIALS: &str =
    "Git credential store contains plaintext credentials";

fn high(
    explanation: impl Into<String>,
    solution: impl Into<String>,
    affected: Vec<AffectedFile>,
) -> Finding {
    config::high(NAME, DOCS_URL, explanation, solution, affected)
}

fn affected(path: &Path, line: usize) -> AffectedFile {
    config::affected(path, line)
}

pub(crate) fn findings(home: &Path) -> Vec<Finding> {
    credential_store_paths(home)
        .into_iter()
        .filter_map(|path| {
            let contents = read_to_string(&path)?;
            let affected = credential_file_secret_lines(&contents)
                .into_iter()
                .map(|line| affected(&path, line))
                .collect::<Vec<_>>();
            if affected.is_empty() {
                return None;
            }
            if path == home.join(".git-credentials") {
                Some(high(
                    PLAINTEXT_GIT_CREDENTIALS,
                    credential_file_solution(&path),
                    affected,
                ))
            } else {
                Some(high(
                    format!(
                        "Git credential store contains plaintext credentials: {}",
                        path.display()
                    ),
                    credential_file_solution(&path),
                    affected,
                ))
            }
        })
        .collect()
}

fn credential_file_solution(path: &Path) -> String {
    format!(
        "Run `rm {}` or edit it to remove the credential; then use SSH remotes.",
        shell_quote(path)
    )
}

fn shell_quote(path: &Path) -> String {
    let value = path.display().to_string();
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | ':'))
    {
        value
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn credential_store_paths(home: &Path) -> Vec<PathBuf> {
    let mut paths = vec![home.join(".git-credentials")];
    for config in git_config_paths(home) {
        let Some(contents) = read_to_string(&config) else {
            continue;
        };
        paths.extend(config::store_paths(home, &contents));
    }
    paths.sort();
    paths.dedup();
    paths
}

fn credential_file_secret_lines(contents: &str) -> Vec<usize> {
    contents
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let trimmed = line.trim();
            (!trimmed.is_empty()
                && !trimmed.starts_with('#')
                && url_contains_userinfo_secret(trimmed))
            .then_some(index + 1)
        })
        .collect()
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

        assert_eq!(
            findings(&home),
            vec![high(
                PLAINTEXT_GIT_CREDENTIALS,
                credential_file_solution(&home.join(".git-credentials")),
                vec![affected(&home.join(".git-credentials"), 1)]
            )]
        );

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
        assert!(findings[0].explanation.contains("custom-git-credentials"));
        assert!(findings[0].solution.contains("rm "));
        assert_eq!(findings[0].affected[0].line, Some(1));

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn plaintext_secret_requires_userinfo_password() {
        assert_eq!(
            credential_file_secret_lines("https://user:secret@example.com/repo.git\n"),
            vec![1]
        );
        assert!(credential_file_secret_lines("https://example.com/repo.git\n").is_empty());
        assert!(credential_file_secret_lines("https://user@example.com/repo.git\n").is_empty());
    }

    #[test]
    fn solution_shell_quotes_paths_with_spaces() {
        assert!(
            credential_file_solution(Path::new("/tmp/home dir/.git-credentials"))
                .contains("rm '/tmp/home dir/.git-credentials'")
        );
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
