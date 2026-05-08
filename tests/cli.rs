use std::process::{Command, Output};
use std::{fs, path::PathBuf};

#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

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

fn debug_opt_root() -> PathBuf {
    PathBuf::from("/tmp/opt")
}

fn write_test_receipt(package_name: &str, version: &str, source: serde_json::Value) -> PathBuf {
    let root = debug_opt_root().join(package_name);
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    fs::create_dir_all(root.join(".pkg")).unwrap();
    fs::write(
        root.join(".pkg/root-receipt.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "package_name": package_name,
            "version": version,
            "source": source,
            "metadata": {},
        }))
        .unwrap(),
    )
    .unwrap();
    root
}

struct PackageRootGuard {
    root: PathBuf,
}

impl PackageRootGuard {
    fn install(package_name: &str, version: &str, source: serde_json::Value) -> Self {
        Self {
            root: write_test_receipt(package_name, version, source),
        }
    }
}

impl Drop for PackageRootGuard {
    fn drop(&mut self) {
        if self.root.exists() {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }
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

    let output = run_nuke(&["help", "i"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: av i"));

    let output = run_nuke(&["help", "uninstall"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: av uninstall"));

    let output = run_nuke(&["help", "outdated"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: av outdated"));

    let output = run_nuke(&["help", "list"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: av list"));

    let output = run_nuke(&["help", "info"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: av info"));

    let output = run_nuke(&["help", "search"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: av search"));

    let output = run_nuke(&["help", "serve"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: av serve"));

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
    assert!(stdout(&output).contains(&format!("av gate {}", pkg_version())));

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

#[test]
fn subs_query_commands_cover_success_and_output_modes() {
    let output = run_nuke(&["search", "rg"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("ripgrep"));

    let output = run_nuke(&["search", "--json", "rg"]);
    assert!(output.status.success());
    let search: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(search.as_array().unwrap().iter().any(|package| {
        package["package_name"] == "ripgrep" || package["package_name"] == "rg"
    }));

    let output = run_nuke(&["search", "rg", "--jsonl"]);
    assert!(output.status.success());
    assert!(
        stdout(&output)
            .lines()
            .all(|line| serde_json::from_str::<serde_json::Value>(line).is_ok())
    );

    let output = run_nuke(&["info", "--json", "rg"]);
    assert!(output.status.success());
    let info: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(info["package_name"] == "ripgrep" || info["package_name"] == "rg");

    let output = run_nuke(&["info", "rg", "--jsonl"]);
    assert!(output.status.success());
    let info: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(info["package_name"] == "ripgrep" || info["package_name"] == "rg");

    let output = run_nuke(&["info", "rg"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("ripgrep"));
    assert!(stdout(&output).contains("Installed"));

    let output = run_nuke(&["list", "--json"]);
    assert!(output.status.success());
    serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap();

    let output = run_nuke(&["list", "--jsonl"]);
    assert!(output.status.success());
    assert!(
        stdout(&output)
            .lines()
            .all(|line| serde_json::from_str::<serde_json::Value>(line).is_ok())
    );
}

#[test]
fn subs_serve_command_covers_non_server_paths() {
    let output = run_nuke(&["serve", "--help"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: av serve"));

    let output = run_nuke(&["serve", "--version"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains(&format!("av serve {}", pkg_version())));

    let output = run_nuke(&["serve", "--bad"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("unknown argument '--bad'"));
}

#[test]
fn subs_list_and_outdated_commands_cover_requested_outputs() {
    let _installed = PackageRootGuard::install(
        "coverage-cli-status",
        "0.0.1",
        serde_json::json!({
            "kind": "isotope",
            "isotope_name": "gh"
        }),
    );

    let output = run_nuke(&["list", "coverage-cli-status"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("coverage-cli-status 0.0.1"));

    let output = run_nuke(&["list", "coverage-cli-status", "--json"]);
    assert!(output.status.success());
    let listed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(listed[0]["package_name"], "coverage-cli-status");

    let output = run_nuke(&["list", "coverage-cli-status", "--jsonl"]);
    assert!(output.status.success());
    let listed: serde_json::Value = serde_json::from_str(stdout(&output).trim()).unwrap();
    assert_eq!(listed["package_name"], "coverage-cli-status");

    let output = run_nuke(&["outdated", "coverage-cli-status"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("coverage-cli-status 0.0.1 ->"));

    let output = run_nuke(&["outdated", "coverage-cli-status", "--json"]);
    assert!(output.status.success());
    let outdated: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(outdated.as_array().unwrap().len(), 1);
    assert_eq!(outdated[0]["package_name"], "coverage-cli-status");

    let output = run_nuke(&["outdated", "coverage-cli-status", "--jsonl"]);
    assert!(output.status.success());
    let outdated: serde_json::Value = serde_json::from_str(stdout(&output).trim()).unwrap();
    assert_eq!(outdated["package_name"], "coverage-cli-status");
}

#[test]
fn subs_help_fallback_and_non_utf8_arguments_report_errors() {
    let output = run_nuke(&["help", "wat"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("USAGE"));
    assert!(stdout(&output).contains("av <subcommand> [args...]"));

    #[cfg(unix)]
    {
        let output = Command::new(env!("CARGO_BIN_EXE_av"))
            .arg(std::ffi::OsString::from_vec(vec![0xff]))
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(stderr(&output).contains("av: subcommand must be valid UTF-8"));

        let output = Command::new(env!("CARGO_BIN_EXE_av"))
            .arg("gate")
            .arg(std::ffi::OsString::from_vec(vec![0xff]))
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(stderr(&output).contains("av gate: gate message must be valid UTF-8"));
    }
}
