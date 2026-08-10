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
//! - A GitHub-scoped `credential.helper` command invoking an untrusted
//!   `gh auth git-credential` produces a finding.
//! - A helper chain is exempt only when an empty helper resets inherited
//!   helpers and every effective helper is an absolute, executable `gh` path
//!   carrying the Automic Vault Isotope signature.
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
//! - Config includes make the effective helper chain uncertain, so they disable
//!   the signed-Isotope exemption and preserve the live probe.
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

use crate::isotopes::hardeners::{executable, isotope};
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
    findings_with(
        home,
        |path| executable(path) && isotope::signature_valid(path, "gh"),
        || git_credential_fill_exposes_github_token().unwrap_or(false),
    )
}

fn findings_with(
    home: &Path,
    trusted_gh_isotope: impl Fn(&Path) -> bool,
    credential_fill_exposes_token: impl Fn() -> bool,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut effective_helpers = Vec::new();
    let mut saw_reset = false;
    let mut config_chain_is_complete = git_config_environment_is_default();
    for path in git_config_paths(home) {
        let Some(contents) = read_to_string(&path) else {
            continue;
        };
        if config::has_include_directive(&contents) {
            config_chain_is_complete = false;
        }
        for helper in config::github_credential_helpers(&contents) {
            if helper.value.is_empty() {
                effective_helpers.clear();
                saw_reset = true;
                continue;
            }
            effective_helpers.push((path.clone(), helper.line, helper.value.to_string()));
        }
    }

    let mut all_effective_helpers_are_trusted_isotopes = !effective_helpers.is_empty();
    for (path, line, value) in &effective_helpers {
        let Some(helper_executable) = config::gh_auth_git_credential_executable(value) else {
            all_effective_helpers_are_trusted_isotopes = false;
            continue;
        };
        let exact_helper_executable = config::exact_gh_auth_git_credential_executable(value);
        if exact_helper_executable.as_deref() == Some(helper_executable.as_str())
            && gh_helper_is_trusted_isotope(&helper_executable, &trusted_gh_isotope)
        {
            continue;
        }
        all_effective_helpers_are_trusted_isotopes = false;
        findings.push(high(
            GH_HELPER_MESSAGE,
            GH_HELPER_SOLUTION,
            vec![affected(path, *line)],
        ));
    }

    let trusted_isotope_chain =
        saw_reset && config_chain_is_complete && all_effective_helpers_are_trusted_isotopes;
    if findings.is_empty() && !trusted_isotope_chain && credential_fill_exposes_token() {
        findings.push(high_unattributed(FILL_MESSAGE, FILL_SOLUTION));
    }

    findings
}

fn gh_helper_is_trusted_isotope(
    helper_executable: &str,
    trusted_gh_isotope: impl Fn(&Path) -> bool,
) -> bool {
    let path = Path::new(helper_executable);
    path.is_absolute()
        && path.file_name().is_some_and(|name| name == "gh")
        && trusted_gh_isotope(path)
}

fn git_config_environment_is_default() -> bool {
    [
        "GIT_CONFIG_GLOBAL",
        "GIT_CONFIG_SYSTEM",
        "GIT_CONFIG_NOSYSTEM",
        "GIT_CONFIG_COUNT",
        "GIT_CONFIG_PARAMETERS",
    ]
    .into_iter()
    .all(|key| std::env::var_os(key).is_none())
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
    fn accepts_reset_chain_containing_only_signed_gh_isotope() {
        let home = temp_home("signed-gh-helper");
        let gh = Path::new("/opt/homebrew/bin/gh");
        fs::write(
            home.join(".gitconfig"),
            "[credential \"https://github.com\"]\nhelper =\nhelper = !/opt/homebrew/bin/gh auth git-credential\n",
        )
        .unwrap();
        let probe_ran = std::cell::Cell::new(false);

        let findings = findings_with(
            &home,
            |path| path == gh,
            || {
                probe_ran.set(true);
                true
            },
        );

        assert!(findings.is_empty());
        assert!(!probe_ran.get());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn signed_gh_without_reset_does_not_hide_live_exposure() {
        let home = temp_home("signed-gh-without-reset");
        fs::write(
            home.join(".gitconfig"),
            "[credential \"https://github.com\"]\nhelper = !/opt/homebrew/bin/gh auth git-credential\n",
        )
        .unwrap();

        let findings = findings_with(
            &home,
            |path| path == Path::new("/opt/homebrew/bin/gh"),
            || true,
        );

        assert_eq!(
            findings,
            vec![high_unattributed(FILL_MESSAGE, FILL_SOLUTION)]
        );
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn unsigned_absolute_gh_helper_remains_a_finding() {
        let home = temp_home("unsigned-gh-helper");
        fs::write(
            home.join(".gitconfig"),
            "[credential \"https://github.com\"]\nhelper =\nhelper = !/usr/local/bin/gh auth git-credential\n",
        )
        .unwrap();

        let findings = findings_with(&home, |_| false, || false);

        assert_eq!(
            findings,
            vec![high(
                GH_HELPER_MESSAGE,
                GH_HELPER_SOLUTION,
                vec![affected(&home.join(".gitconfig"), 3)]
            )]
        );
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn signed_gh_with_trailing_shell_command_remains_a_finding() {
        let home = temp_home("signed-gh-shell-command");
        fs::write(
            home.join(".gitconfig"),
            "[credential \"https://github.com\"]\nhelper =\nhelper = !/opt/homebrew/bin/gh auth git-credential ; printf password=stolen\n",
        )
        .unwrap();

        let findings = findings_with(&home, |_| true, || false);

        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].affected,
            vec![affected(&home.join(".gitconfig"), 3)]
        );
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn config_include_disables_signed_isotope_exemption() {
        let home = temp_home("included-helper");
        fs::write(
            home.join(".gitconfig"),
            "[include]\npath = ~/.gitconfig-extra\n[credential \"https://github.com\"]\nhelper =\nhelper = !/opt/homebrew/bin/gh auth git-credential\n",
        )
        .unwrap();

        let findings = findings_with(&home, |_| true, || true);

        assert_eq!(
            findings,
            vec![high_unattributed(FILL_MESSAGE, FILL_SOLUTION)]
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
