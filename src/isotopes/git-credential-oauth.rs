//! Security check: `git-credential-oauth` ambient helper configuration.
//!
//! What this detects:
//! - Git config lines that enable the `oauth` credential helper.
//! - Git config lines that contain a likely real `oauthClientSecret` value.
//!
//! Why this matters:
//! - OAuth credential helpers can make credentials available to Git without an
//!   explicit user decision at command time.
//! - A plaintext OAuth client secret in Git config is readable by any same-user
//!   process and may allow impersonation of the configured OAuth app/client.
//! - Agent-run commands can exercise ambient helper configuration in the same
//!   environment as the user.
//!
//! Evidence used:
//! - `$HOME/.gitconfig` and the XDG Git config path are scanned.
//! - Comment suffixes beginning with `#` or `;` are ignored.
//! - A helper token equal to `oauth` triggers the helper finding.
//! - `oauthClientSecret` triggers only when the value is non-trivial: at least
//!   six characters, not `${...}`, and not the placeholder word `secret`.
//!
//! Known issues:
//! - This is a line-oriented detector and does not fully parse Git config.
//! - Inline comments inside quoted values are not preserved.
//! - The detector may report helper configuration even when the helper binary is
//!   not installed.
//!
//! Known omissions:
//! - Repository-local `.git/config` files are not scanned yet.
//! - Included Git config files are not followed.
//! - The detector does not inspect helper caches or OAuth refresh-token stores.
//! - It does not validate whether a client secret is active.
//!
//! Safety notes:
//! - This detector only reads config files.
//! - It reports the config path but not the secret value.
//! - Missing or unreadable config files are treated as clean.

use std::path::Path;

use crate::Finding;

use super::{git_config_paths, high, read_to_string};

pub(crate) fn findings(home: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    for path in git_config_paths(home) {
        let Some(contents) = read_to_string(&path) else {
            continue;
        };
        if git_config_uses_oauth_helper(&contents) {
            findings.push(high(format!(
                "Git config enables git-credential-oauth as an ambient credential helper: {}",
                path.display()
            )));
        }
        if git_config_contains_oauth_client_secret(&contents) {
            findings.push(high(format!(
                "Git config contains a plaintext OAuth client secret: {}",
                path.display()
            )));
        }
    }
    findings
}

fn git_config_uses_oauth_helper(contents: &str) -> bool {
    contents.lines().any(|line| {
        let line = uncomment(line).trim();
        let Some((key, value)) = line.split_once('=') else {
            return false;
        };
        key.trim().ends_with("helper")
            && value
                .split_whitespace()
                .any(|word| word.trim_matches('"').trim_matches('\'') == "oauth")
    })
}

fn git_config_contains_oauth_client_secret(contents: &str) -> bool {
    contents.lines().any(|line| {
        let line = uncomment(line).trim();
        let Some((key, value)) = line.split_once('=') else {
            return false;
        };
        key.trim().ends_with("oauthClientSecret") && secret_value_is_real(value)
    })
}

fn uncomment(line: &str) -> &str {
    line.split(['#', ';']).next().unwrap_or("")
}

fn secret_value_is_real(value: &str) -> bool {
    let value = value.trim().trim_matches('"').trim_matches('\'');
    value.len() >= 6 && !value.contains("${") && !value.eq_ignore_ascii_case("secret")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detects_oauth_helper_and_client_secret() {
        assert!(git_config_uses_oauth_helper(
            "[credential]\nhelper = cache --timeout 21600\nhelper = oauth -device\n"
        ));
        assert!(git_config_contains_oauth_client_secret(
            "[credential \"https://gitlab.example.com\"]\noauthClientSecret = abcdefgh\n"
        ));
    }

    #[test]
    fn config_file_triggers_oauth_findings() {
        let home = temp_home("oauth");
        fs::write(
            home.join(".gitconfig"),
            "[credential]\nhelper = oauth -device\noauthClientSecret = abcdefgh\n",
        )
        .unwrap();

        let findings = findings(&home);

        assert_eq!(findings.len(), 2);
        assert!(findings[0].message.contains("git-credential-oauth"));
        assert!(findings[1].message.contains("OAuth client secret"));

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn ignores_placeholder_client_secret() {
        assert!(!git_config_contains_oauth_client_secret(
            "oauthClientSecret = ${TOKEN}\n"
        ));
        assert!(!git_config_contains_oauth_client_secret(
            "oauthClientSecret = secret\n"
        ));
    }

    fn temp_home(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "av-git-oauth-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
