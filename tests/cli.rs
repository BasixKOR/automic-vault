use std::process::{Command, Output};

fn pkg_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn run_nuke(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_av"))
        .args(args)
        .output()
        .unwrap()
}

fn run_nuke_with_columns(args: &[&str], columns: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_av"))
        .env("COLUMNS", columns)
        .args(args)
        .output()
        .unwrap()
}

fn run_nuke_with_forced_color(args: &[&str], columns: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_av"))
        .env("CLICOLOR_FORCE", "1")
        .env_remove("NO_COLOR")
        .env("COLUMNS", columns)
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
fn subs_top_level_cli_paths_cover_help_version_and_unknown_subcommands() {
    let output = run_nuke(&[]);
    assert!(!output.status.success());
    assert!(stdout(&output).contains("USAGE"));
    assert!(stdout(&output).contains("av <subcommand> [args...]"));
    assert!(stderr(&output).contains("av: missing subcommand"));

    let output = run_nuke_with_columns(&["--help"], "140");
    assert!(output.status.success());
    assert!(stdout(&output).contains("PACKAGE SYSTEM"));
    assert!(stdout(&output).contains("▪ PACKAGE SYSTEM"));
    assert!(stdout(&output).contains("install (i)"));
    assert!(stdout(&output).contains("list (ls)"));
    assert!(stdout(&output).contains("─"));
    assert!(stdout(&output).contains("LEGEND"));

    let output = run_nuke_with_columns(&["--help"], "90");
    assert!(output.status.success());
    assert!(stdout(&output).starts_with("────────────────"));
    assert!(!stdout(&output).contains("LEGEND"));

    let output = run_nuke_with_forced_color(&["--help"], "90");
    let colored_stdout = stdout(&output);
    assert!(output.status.success());
    assert!(!colored_stdout.contains("\x1b[38;2;214;198;165m"));
    assert!(colored_stdout.contains("\x1b[2m"));
    assert!(colored_stdout.contains("\x1b[38;2;224;90;71m"));

    let output = run_nuke(&["--version"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains(&format!("av {}", pkg_version())));

    let output = run_nuke(&["help", "update"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: av update"));

    let output = run_nuke(&["help", "info"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: av info"));

    let output = run_nuke(&["help", "inject"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: av inject"));

    let output = run_nuke(&["help", "save"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: av save"));

    let output = run_nuke(&["help", "gate"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: av gate"));

    let output = run_nuke(&["help", "contain"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: av contain"));

    let output = run_nuke(&["wat"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("av: unknown subcommand 'wat'"));

    let output = run_nuke(&["x"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("av: unknown subcommand 'x'"));

    let output = run_nuke(&["run"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("av: unknown subcommand 'run'"));
}

#[test]
fn subs_gate_cli_covers_help_version_and_parse_errors() {
    let output = run_nuke(&["gate", "--help"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: av gate"));

    let output = run_nuke(&["gate", "--version"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("av gate 0.1.0"));

    let output = run_nuke(&["gate"]);
    assert!(!output.status.success());
    assert!(stdout(&output).contains("Usage: av gate"));
    assert!(stderr(&output).contains("missing gate message"));

    let output = run_nuke(&["gate", "   "]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("empty gate message"));

    let output = run_nuke(&["gate", "approve", "extra"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("single gate message"));
}

#[test]
fn subs_subcommand_parsing_covers_help_version_and_non_root_failures() {
    let version = pkg_version();
    let cases = [
        (vec!["i", "--help"], true, "Usage: av i".to_string()),
        (vec!["i", "--version"], true, format!("av i {version}")),
        (
            vec!["update", "--help"],
            true,
            "Usage: av update".to_string(),
        ),
        (
            vec!["update", "--version"],
            true,
            format!("av update {version}"),
        ),
        (vec!["list", "--help"], true, "Usage: av list".to_string()),
        (
            vec!["list", "--version"],
            true,
            format!("av list {version}"),
        ),
        (vec!["info", "--help"], true, "Usage: av info".to_string()),
        (
            vec!["info", "--version"],
            true,
            format!("av info {version}"),
        ),
        (
            vec!["outdated", "--help"],
            true,
            "Usage: av outdated".to_string(),
        ),
        (
            vec!["outdated", "--version"],
            true,
            format!("av outdated {version}"),
        ),
        (
            vec!["uninstall", "--help"],
            true,
            "Usage: av uninstall".to_string(),
        ),
        (
            vec!["uninstall", "--version"],
            true,
            format!("av uninstall {version}"),
        ),
    ];

    for (args, success, needle) in cases {
        let output = run_nuke(&args);
        let stdout = stdout(&output);
        assert_eq!(output.status.success(), success, "{args:?}");
        assert!(stdout.contains(&needle), "{args:?}: {stdout}");
    }

    let output = run_nuke(&["info"]);
    assert!(!output.status.success());
    assert!(stdout(&output).contains("Usage: av info"));
    assert!(stderr(&output).contains("av: missing package name"));

    if !cfg!(debug_assertions) && unsafe { libc::geteuid() } != 0 {
        let output = run_nuke(&["i", "bun"]);
        assert!(!output.status.success());
        assert!(stderr(&output).contains("av: must be run as root"));

        let output = run_nuke(&["update"]);
        assert!(!output.status.success());
        assert!(stderr(&output).contains("av: must be run as root"));
    }
}
