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
//! - Static config evidence wins first: a GitHub-scoped `credential.helper`
//!   command invoking `gh auth git-credential` produces a finding without
//!   spawning `git`.
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
//!
//! Known omissions:
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
//! - Tests require explicit opt-in for the live probe, and CLI integration tests
//!   disable it for hermetic output.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use crate::Finding;

use super::{git_config, git_config_paths, high, read_to_string};

const GITHUB_CREDENTIAL_FILL_INPUT: &[u8] = b"protocol=https\nhost=github.com\n\n";
const GITHUB_CREDENTIAL_FILL_TIMEOUT: Duration = Duration::from_secs(3);
const GH_HELPER_MESSAGE: &str = "Git credential helper delegates github.com credentials to `gh auth git-credential`, exposing the GitHub CLI token through `git credential fill`. Click Learn More to learn how to fix it.";
const FILL_MESSAGE: &str = "Git credential helper exposes a GitHub token through `git credential fill` for github.com. Click Learn More to learn how to fix it.";

pub(crate) fn findings(home: &Path) -> Vec<Finding> {
    if git_config_paths(home).into_iter().any(|path| {
        read_to_string(&path)
            .as_deref()
            .is_some_and(git_config::exposes_github_token_via_gh_helper)
    }) {
        return vec![high(GH_HELPER_MESSAGE)];
    }

    match git_credential_fill_exposes_github_token() {
        Ok(true) => vec![high(FILL_MESSAGE)],
        Ok(false) | Err(_) => Vec::new(),
    }
}

fn git_credential_fill_exposes_github_token() -> Result<bool, String> {
    if !git_credential_fill_probe_enabled() {
        return Ok(false);
    }

    let mut child = match Command::new("git")
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

        assert_eq!(findings, vec![high(GH_HELPER_MESSAGE)]);

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
    fn wait_for_credential_fill_reads_fake_git_password() {
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
        let child = Command::new(&git)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let output = wait_for_credential_fill(child, GITHUB_CREDENTIAL_FILL_TIMEOUT).unwrap();

        assert!(git_credential_fill_output_exposes_github_token(&output));

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
