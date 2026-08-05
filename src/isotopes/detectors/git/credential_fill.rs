//! Security check: `git credential fill` GitHub token exposure.
//!
//! What this detects:
//! - Git credential helpers that return a password/token when asked for
//!   `protocol=https` and `host=github.com`.
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
//! - Otherwise, the detector runs `git credential fill` with prompts disabled
//!   and checks whether stdout includes a non-empty `password` for GitHub.
//! - A missing `host` in output is treated as GitHub-scoped because the query
//!   was for `github.com`.
//!
//! Known issues:
//! - Running `git credential fill` can invoke third-party helper binaries.
//! - A helper may return different results depending on machine state, helper
//!   cache, keychain unlock state, or network availability.
//! - The process is timeout-bound, but helper startup cost can still make scans
//!   slower than pure file checks.
//! - This relies on the shared Git config parser and inherits its limitations.
//! - The detector may report a helper command even when `gh` is no longer
//!   installed.
//!
//! Known omissions:
//! - Live `git credential fill` findings may have no affected file because Git
//!   does not report which helper supplied the credential.
//! - Only `github.com` is queried today.
//! - The detector does not inspect token scopes or validate token shape.
//! - It does not remediate helper configuration.
//! - It does not query repository-local credential context.
//!
//! Safety notes:
//! - Prompts are disabled with `GIT_TERMINAL_PROMPT=0` and
//!   `GCM_INTERACTIVE=never`.
//! - The child process is killed on timeout.
//! - Returned passwords are never printed; only the exposure condition is
//!   reported.
//! - Config-backed findings report the helper line, not any token value.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use crate::{AffectedFile, Finding};

use super::config::{self, git_config_paths, read_to_string};

const NAME: &str = "git-credential-fill";
const DOCS_URL: &str = "https://github.com/automic-vault/automic-vault/blob/main/src/isotopes/detectors/git/credential_fill.md";
const GITHUB_CREDENTIAL_FILL_INPUT: &[u8] = b"protocol=https\nhost=github.com\n\n";
const GITHUB_CREDENTIAL_FILL_TIMEOUT: Duration = Duration::from_secs(3);
const GH_HELPER_MESSAGE: &str = "Git credential helper delegates github.com credentials to `gh auth git-credential`, exposing the GitHub CLI token through `git credential fill`. Click Learn More to learn how to fix it.";
const GH_HELPER_SOLUTION: &str = "Edit the affected Git config and remove the `helper = !gh auth git-credential` line; then change GitHub remotes to SSH with `git remote set-url origin git@github.com:OWNER/REPO.git`.";
const FILL_MESSAGE: &str = "Git credential helper exposes a GitHub token through `git credential fill` for github.com. Click Learn More to learn how to fix it.";
const FILL_SOLUTION: &str = "Run `printf 'protocol=https\\nhost=github.com\\n\\n' | git credential reject`, then remove or disable the credential helper that returned the token and use SSH remotes.";

fn high(
    explanation: impl Into<String>,
    solution: impl Into<String>,
    affected: Vec<AffectedFile>,
) -> Finding {
    config::high(NAME, DOCS_URL, explanation, solution, affected)
}

fn high_unattributed(explanation: impl Into<String>, solution: impl Into<String>) -> Finding {
    config::high_unattributed(NAME, DOCS_URL, explanation, solution)
}

fn affected(path: &Path, line: usize) -> AffectedFile {
    config::affected(path, line)
}

pub(crate) fn findings(home: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    for path in git_config_paths(home) {
        let Some(contents) = read_to_string(&path) else {
            continue;
        };
        for line in config::gh_auth_git_credential_lines(&contents) {
            findings.push(high(
                GH_HELPER_MESSAGE,
                GH_HELPER_SOLUTION,
                vec![affected(&path, line)],
            ));
        }
    }
    if findings.is_empty() && git_credential_fill_exposes_github_token().unwrap_or(false) {
        findings.push(high_unattributed(FILL_MESSAGE, FILL_SOLUTION));
    }

    findings
}

fn git_credential_fill_exposes_github_token() -> Result<bool, String> {
    if !git_credential_fill_probe_enabled() {
        return Ok(false);
    }

    let mut command = Command::new("git");
    git_credential_fill_command_exposes_github_token(&mut command)
}

fn git_credential_fill_command_exposes_github_token(command: &mut Command) -> Result<bool, String> {
    let mut child = match command
        .args(["credential", "fill"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to run git credential fill: {err}")),
    };

    if let Some(mut stdin) = child.stdin.take() {
        if let Err(err) = stdin.write_all(GITHUB_CREDENTIAL_FILL_INPUT) {
            if err.kind() == std::io::ErrorKind::BrokenPipe {
                return Ok(false);
            }
            return Err(format!(
                "failed to send GitHub credential query to git: {err}"
            ));
        }
    }

    let output = wait_for_credential_fill(child, GITHUB_CREDENTIAL_FILL_TIMEOUT)?;
    Ok(git_credential_fill_output_exposes_github_token(&output))
}

fn git_credential_fill_probe_enabled() -> bool {
    if env_flag("AUTOMIC_VAULT_DISABLE_GIT_CREDENTIAL_FILL_DETECTOR") {
        return false;
    }

    #[cfg(test)]
    {
        env_flag("AUTOMIC_VAULT_TEST_GIT_CREDENTIAL_FILL_DETECTOR")
    }

    #[cfg(not(test))]
    {
        true
    }
}

fn env_flag(key: &str) -> bool {
    std::env::var_os(key).is_some_and(|value| {
        let value = value.to_string_lossy();
        !value.is_empty() && value != "0" && value != "false"
    })
}

fn wait_for_credential_fill(
    mut child: std::process::Child,
    timeout: Duration,
) -> Result<Output, String> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                return child
                    .wait_with_output()
                    .map_err(|err| format!("failed to read git credential fill output: {err}"));
            }
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("git credential fill timed out while checking github.com".to_string());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(err) => return Err(format!("failed to wait for git credential fill: {err}")),
        }
    }
}

fn git_credential_fill_output_exposes_github_token(output: &Output) -> bool {
    let Ok(stdout) = std::str::from_utf8(&output.stdout) else {
        return false;
    };

    let mut saw_host = false;
    let mut host_is_github = false;
    let mut has_password = false;
    for line in stdout.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "host" => {
                saw_host = true;
                host_is_github = value.eq_ignore_ascii_case("github.com");
            }
            "password" => has_password = !value.trim().is_empty(),
            _ => {}
        }
    }

    has_password && (!saw_host || host_is_github)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::{fs::PermissionsExt, process::ExitStatusExt};

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
                GH_HELPER_SOLUTION,
                vec![affected(&home.join(".gitconfig"), 2)]
            )]
        );

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn parses_github_credential_fill_password_without_requiring_token_shape() {
        let output = Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout:
                b"protocol=https\nhost=github.com\nusername=x-access-token\npassword=ghp_secret\n\n"
                    .to_vec(),
            stderr: Vec::new(),
        };

        assert!(git_credential_fill_output_exposes_github_token(&output));
    }

    #[test]
    fn ignores_credential_fill_output_without_password() {
        let output = Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: b"protocol=https\nhost=github.com\nusername=monalisa\n\n".to_vec(),
            stderr: Vec::new(),
        };

        assert!(!git_credential_fill_output_exposes_github_token(&output));
    }

    #[test]
    fn ignores_credential_fill_output_for_other_hosts() {
        let output = Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: b"protocol=https\nhost=example.com\nusername=u\npassword=p\n\n".to_vec(),
            stderr: Vec::new(),
        };

        assert!(!git_credential_fill_output_exposes_github_token(&output));
    }

    #[test]
    fn credential_fill_probe_reports_fake_git_password_without_affected_file() {
        let temp = temp_home("fill");
        let bin = temp.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let git = bin.join("git");
        fs::write(
            &git,
            "#!/bin/sh\nprintf 'protocol=https\\nhost=github.com\\nusername=x-access-token\\npassword=ghp_secret\\n\\n'\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&git).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&git, permissions).unwrap();

        let mut command = Command::new(&git);
        assert!(git_credential_fill_command_exposes_github_token(&mut command).unwrap());

        let _ = fs::remove_dir_all(temp);
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
