use std::process::{Command, Output};

fn run_isotope(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_av"))
        .arg("inject")
        .args(args)
        .output()
        .unwrap()
}

fn run_save(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_av"))
        .arg("save")
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
fn subs_isotope_cli_covers_help_version_and_missing_target() {
    let output = run_isotope(&["--help"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: av inject"));
    assert!(stdout(&output).contains("--replace-existing-env"));
    assert!(stdout(&output).contains("+KEY"));

    let output = run_isotope(&["--version"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("av inject 0.1.0"));

    let output = run_isotope(&[]);
    assert!(!output.status.success());
    assert!(stdout(&output).contains("Usage: av inject"));
    assert!(stderr(&output).contains("missing key and target binary"));
}

#[test]
fn subs_save_cli_covers_help_version_and_parse_errors() {
    let output = run_save(&["--help"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: av save"));
    assert!(stdout(&output).contains("stdin"));
    assert!(stdout(&output).contains("prompted without echo"));

    let output = run_save(&["--version"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("av save 0.1.0"));

    let output = run_save(&[]);
    assert!(!output.status.success());
    assert!(stdout(&output).contains("Usage: av save"));
    assert!(stderr(&output).contains("missing KEY"));

    let output = run_save(&["FOO=bar"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("KEY only"));

    let output = run_save(&["--allow", "/usr/bin/env", "FOO"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("removed"));
}

#[test]
fn subs_isotope_rejects_relative_targets_before_execution() {
    let output = run_isotope(&["+TOKEN", "./tool"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("target binary path must be absolute"));
}

#[test]
fn subs_isotope_rejects_removed_import_and_migrate_flags() {
    let output = run_isotope(&["--import", "./tool"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("no longer supported"));

    let output = run_isotope(&["--migrate", "./tool"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("no longer supported"));
}

#[test]
fn subs_isotope_rejects_removed_force_flag() {
    let output = run_isotope(&["--force", "+TOKEN", "/bin/echo"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("--replace-existing-env"));
}

#[test]
fn subs_isotope_rejects_removed_allow_existing_env_flag() {
    let output = run_isotope(&["--allow-existing-env", "+TOKEN", "/bin/echo"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("--replace-existing-env"));
}

#[test]
fn subs_isotope_rejects_root_execution_when_invoked_as_root() {
    if unsafe { libc::geteuid() } != 0 {
        return;
    }

    let output = run_isotope(&["+TOKEN", "/bin/echo", "hi"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("must not be run as root"));
}
