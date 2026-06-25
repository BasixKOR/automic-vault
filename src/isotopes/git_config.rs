//! Shared security parser: Git credential-helper config.
//!
//! What this supports:
//! - Extracting `credential.helper` values from Git config text.
//! - Resolving whether helper configuration applies to `github.com`.
//! - Finding plaintext `store` helper file paths.
//! - Recognizing `gh auth git-credential` helper commands.
//!
//! Why this matters:
//! - Multiple Git detectors need the same boundary logic: which helper applies,
//!   which host it targets, and which file path it references.
//! - Keeping this logic shared prevents detector drift while avoiding a full
//!   Git config parser until it is needed.
//!
//! Evidence model:
//! - Supports section form, such as `[credential]` and
//!   `[credential "https://github.com"]`.
//! - Supports key form, such as `credential.helper = ...` and
//!   `credential.https://github.com.helper = ...`.
//! - Treats global credential helper settings as applying to GitHub.
//! - Expands only `~` and `~/...` for store helper paths.
//!
//! Known issues:
//! - This is intentionally smaller than Git's parser.
//! - It does not implement Git's escape rules, include directives, conditional
//!   includes, multiline values, or platform-specific config precedence.
//! - Shell parsing for helper commands is minimal and only needs enough to
//!   identify `gh auth git-credential`.
//!
//! Known omissions:
//! - Repository-local config is not represented here.
//! - System config and global config outside the supplied scan home are not
//!   included by this helper.
//! - Non-GitHub host matching is only present where needed for exclusion.
//!
//! Safety notes:
//! - This module parses strings only; it does not read files or spawn commands.
//! - Callers decide which config files are in scope for a scan.

use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
enum GitConfigSection {
    Other,
    Credential { applies_to_github: bool },
}

pub(super) fn store_paths(home: &Path, contents: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for helper in credential_helpers(contents) {
        let value = helper.value;
        if value
            .split_whitespace()
            .next()
            .is_some_and(|word| word == "store")
        {
            paths.push(
                store_helper_file_path(home, value)
                    .unwrap_or_else(|| home.join(".git-credentials")),
            );
        }
    }
    paths
}

pub(super) fn gh_auth_git_credential_lines(contents: &str) -> Vec<usize> {
    credential_helpers(contents)
        .into_iter()
        .filter_map(|helper| {
            (helper.applies_to_github && helper_invokes_gh_auth_git_credential(helper.value))
                .then_some(helper.line)
        })
        .collect()
}

struct CredentialHelper<'a> {
    value: &'a str,
    applies_to_github: bool,
    line: usize,
}

fn credential_helpers(contents: &str) -> Vec<CredentialHelper<'_>> {
    let mut helpers = Vec::new();
    let mut section = GitConfigSection::Other;

    for (index, line) in contents.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if let Some(next_section) = git_config_section(trimmed) {
            section = next_section;
            continue;
        }

        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let Some(applies_to_github) = credential_helper_applies_to_github(key.trim(), section)
        else {
            continue;
        };
        helpers.push(CredentialHelper {
            value: git_config_value(value),
            applies_to_github,
            line: index + 1,
        });
    }

    helpers
}

fn git_config_section(trimmed: &str) -> Option<GitConfigSection> {
    let name = trimmed.strip_prefix('[')?.strip_suffix(']')?.trim();
    let Some(rest) = name.strip_prefix("credential") else {
        return Some(GitConfigSection::Other);
    };
    let rest = rest.trim();
    if rest.is_empty() {
        return Some(GitConfigSection::Credential {
            applies_to_github: true,
        });
    }
    let scope = rest.trim_matches('"').trim_matches('\'');
    Some(GitConfigSection::Credential {
        applies_to_github: credential_scope_applies_to_github(scope),
    })
}

fn git_config_value(value: &str) -> &str {
    value.trim().trim_matches('"').trim_matches('\'')
}

fn credential_helper_applies_to_github(key: &str, section: GitConfigSection) -> Option<bool> {
    if key == "helper" {
        return match section {
            GitConfigSection::Credential { applies_to_github } => Some(applies_to_github),
            GitConfigSection::Other => None,
        };
    }
    if key == "credential.helper" {
        return Some(true);
    }
    let scope = key
        .strip_prefix("credential.")
        .and_then(|rest| rest.strip_suffix(".helper"))?;
    Some(credential_scope_applies_to_github(scope))
}

fn credential_scope_applies_to_github(scope: &str) -> bool {
    let scope = scope.trim();
    if scope.is_empty() {
        return true;
    }
    let scope = scope
        .strip_prefix("https://")
        .or_else(|| scope.strip_prefix("http://"))
        .unwrap_or(scope);
    let host = scope
        .split(['/', ':'])
        .next()
        .unwrap_or(scope)
        .trim_end_matches('.');
    host.eq_ignore_ascii_case("github.com")
}

fn helper_invokes_gh_auth_git_credential(value: &str) -> bool {
    let Some(command) = value.trim().strip_prefix('!') else {
        return false;
    };
    let words = shell_words(command);
    words.len() >= 3
        && command_name_is_gh(&words[0])
        && words[1] == "auth"
        && words[2] == "git-credential"
}

fn command_name_is_gh(command: &str) -> bool {
    Path::new(command)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "gh" || name == "gh.exe")
}

fn shell_words(value: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;

    for ch in value.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if let Some(quote_ch) = quote {
            if ch == quote_ch {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        if ch == '\'' || ch == '"' {
            quote = Some(ch);
            continue;
        }
        if ch.is_whitespace() {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(ch);
    }

    if escaped {
        current.push('\\');
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn store_helper_file_path(home: &Path, value: &str) -> Option<PathBuf> {
    let mut words = value.split_whitespace().peekable();
    while let Some(word) = words.next() {
        if let Some(path) = word.strip_prefix("--file=") {
            return Some(expand_home_path(home, path));
        }
        if word == "--file" {
            return words
                .next()
                .map(|path| expand_home_path(home, path))
                .filter(|path| !path.as_os_str().is_empty());
        }
    }
    None
}

fn expand_home_path(home: &Path, value: &str) -> PathBuf {
    if value == "~" {
        return home.to_path_buf();
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return home.join(rest);
    }
    PathBuf::from(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_store_paths() {
        let home = Path::new("/tmp/home");

        assert_eq!(
            store_paths(home, "[credential]\nhelper = store --file ~/.git-store\n"),
            vec![PathBuf::from("/tmp/home/.git-store")]
        );
        assert_eq!(
            store_paths(home, "credential.helper = store --file=/tmp/tokens\n"),
            vec![PathBuf::from("/tmp/tokens")]
        );
    }

    #[test]
    fn detects_github_gh_helper_only_for_github_scope() {
        assert_eq!(
            gh_auth_git_credential_lines(
                "[credential \"https://github.com\"]\nhelper = !'/Applications/GitHub CLI.app/Contents/MacOS/gh' auth git-credential\n"
            ),
            vec![2]
        );
        assert!(
            gh_auth_git_credential_lines(
                "[credential \"https://example.com\"]\nhelper = !gh auth git-credential\n"
            )
            .is_empty()
        );
    }
}
