use std::ffi::OsString;
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Output};

fn pkg_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

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

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|candidate| candidate == needle)
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
    assert!(stdout(&output).contains(&format!("av inject {}", pkg_version())));

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
    assert!(stdout(&output).contains(&format!("av save {}", pkg_version())));

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
fn subs_isotope_accepts_single_argument_shebang_dispatch() {
    let temp = tempfile::tempdir().unwrap();
    let script = temp.path().join("tool");
    fs::write(
        &script,
        format!(
            "#!{} inject +SOME_SECRET /bin/echo\n",
            env!("CARGO_BIN_EXE_av")
        ),
    )
    .unwrap();
    let mut permissions = fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&script, permissions).unwrap();

    let output = Command::new(&script)
        .env("SOME_SECRET", "expected")
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains(&script.to_string_lossy().into_owned()));
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

#[test]
fn subs_isotope_preserves_non_utf8_environment_values() {
    if unsafe { libc::geteuid() } == 0 {
        return;
    }

    let output = Command::new(env!("CARGO_BIN_EXE_av"))
        .arg("inject")
        .args(["+SOME_SECRET", "/usr/bin/env"])
        .env("SOME_SECRET", "expected")
        .env("_", OsString::from_vec(b"/tmp/v\xffrp/script".to_vec()))
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "status: {:?}\nstderr: {}",
        output.status,
        stderr(&output)
    );
    assert!(contains_bytes(&output.stdout, b"SOME_SECRET=expected\n"));
    assert!(contains_bytes(&output.stdout, b"_=/tmp/v\xffrp/script\n"));
}
