//! Security check: `git credential fill` GitHub token exposure.
//!
//! What this detects:
//! - Git config that delegates GitHub credentials to
//!   `gh auth git-credential`, which exposes the GitHub CLI token through
//!   Git's credential protocol.
//!
//! Why this matters:
//! - `git credential fill` is intentionally scriptable; any same-user process
//!   can ask Git for credentials if helper policy allows it.
//! - Agents often run shell commands and can trigger the same credential lookup.
//! - A GitHub token exposed this way may carry broad repository authority.
//!
//! Evidence used:
//! - A GitHub-scoped `credential.helper` command invoking
//!   `gh auth git-credential` produces a finding.
//! - The affected file list points at the Git config line that enables the
//!   helper.
//!
//! Known issues:
//! - This relies on the shared Git config parser and inherits its limitations.
//! - The detector may report a helper command even when `gh` is no longer
//!   installed.
//!
//! Known omissions:
//! - It does not run `git credential fill` directly. That command can expose a
//!   token, but the result cannot always be attributed to a file and line, and
//!   isotope reports require file/line evidence.
//! - The detector does not inspect token scopes or validate token shape.
//! - It does not remediate helper configuration.
//! - It does not query repository-local credential context.
//!
//! Safety notes:
//! - This detector reads Git config only.
//! - It reports the helper line, not any token value.

use std::path::Path;

use crate::Finding;

use super::{affected, git_config, git_config_paths, high, read_to_string};

const GH_HELPER_MESSAGE: &str = "Git credential helper delegates github.com credentials to `gh auth git-credential`, exposing the GitHub CLI token through `git credential fill`. Click Learn More to learn how to fix it.";

pub(crate) fn findings(home: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    for path in git_config_paths(home) {
        let Some(contents) = read_to_string(&path) else {
            continue;
        };
        for line in git_config::gh_auth_git_credential_lines(&contents) {
            findings.push(high(GH_HELPER_MESSAGE, vec![affected(&path, line)]));
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detects_github_cli_credential_helper_without_running_git() {
        let home = temp_home("gh-helper");
        fs::write(
            home.join(".gitconfig"),
            "[credential \"https://github.com\"]\nhelper = !gh auth git-credential\n",
        )
        .unwrap();

        let findings = findings(&home);

        assert_eq!(
            findings,
            vec![high(
                GH_HELPER_MESSAGE,
                vec![affected(&home.join(".gitconfig"), 2)]
            )]
        );

        let _ = fs::remove_dir_all(home);
    }

    fn temp_home(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "av-git-fill-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
