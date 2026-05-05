use std::process::{Command, Output};

fn run_vault(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_av"))
        .arg("contain")
        .args(args)
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn subs_vault_cli_covers_help_version_and_tooling_commands() {
    let output = run_vault(&["--help"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: av contain"));

    let output = run_vault(&["--version"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("av contain 1.0.0"));

    let output = run_vault(&["toolchain", "--help"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: av contain toolchain"));

    let output = run_vault(&["sandbox-profile", "--help"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: av contain sandbox-profile"));

    let output = run_vault(&["--proxy"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("missing proxy stub path"));
}

#[test]
fn subs_vault_proxy_fails_closed_when_daemon_is_unavailable() {
    let output = Command::new(env!("CARGO_BIN_EXE_av"))
        .args([
            "contain",
            "--proxy",
            "/tmp/automic-vault.test/bin/git",
            "status",
        ])
        .env("VAULT_SOCKET_PATH", "/tmp/does-not-exist/vault.sock")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(stderr(&output).contains("vaultd unavailable"));
}

#[test]
fn subs_vault_internal_exec_rejects_untrusted_callers() {
    let output = run_vault(&["internal-exec", "echo", "hi"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("internal-exec is restricted"));
}
