use std::fs;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn av_scan_reports_clean_home() {
    let home = temp_home("clean");
    let output = av_scan(&home);

    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        "╭─ system exposure audit\n│\n◇ No problems found\n│\n╰─ vault sealed\n"
    );
    assert_eq!(stderr(&output), "");

    let _ = fs::remove_dir_all(home);
}

#[test]
fn av_scan_reports_findings() {
    let home = temp_home("triggered");
    fs::write(
        home.join(".git-credentials"),
        "https://user:token@example.com\n",
    )
    .unwrap();

    let output = av_scan(&home);
    let stdout = stdout(&output);

    assert!(output.status.success());
    assert!(stdout.contains("│  solution\n"));
    assert!(stdout.contains("│  Run `rm"));
    assert!(stdout.contains("│  full details & caveats\n"));
    assert!(!stdout.contains("│  read more\n"));
    assert!(stdout.contains(".git-credentials"));
    assert!(stdout.contains("│  affected files\n"));
    assert!(stdout.contains("╰─ scan complete\n"));
    for line in stdout.lines().filter(|line| !line.is_empty()) {
        assert!(
            line.starts_with('╭')
                || line.starts_with('◆')
                || line.starts_with('└')
                || line.starts_with('├')
                || line.starts_with('╰')
                || line.starts_with('│'),
            "{line}"
        );
        if !line.starts_with("│  https://") {
            assert!(line.chars().count() <= 78, "{line}");
        }
    }
    assert_eq!(stderr(&output), "");

    let _ = fs::remove_dir_all(home);
}

#[test]
fn av_scan_json_reports_findings() {
    let home = temp_home("json");
    fs::write(
        home.join(".git-credentials"),
        "https://user:token@example.com\n",
    )
    .unwrap();

    let output = av_scan_json(&home);
    let stdout = stdout(&output);

    assert!(output.status.success());
    assert!(stdout.starts_with(r#"{"findings":[{"#));
    assert!(stdout.contains(r#""source":"git-credentials-file""#));
    assert!(stdout.contains(".git-credentials"));
    assert_eq!(stderr(&output), "");

    let _ = fs::remove_dir_all(home);
}

fn av_scan(home: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_av"))
        .arg("scan")
        .env("HOME", home)
        .env(
            "AUTOMIC_VAULT_TEST_BREW_TARGET",
            home.join("missing-opt-homebrew/bin/brew"),
        )
        .env("AUTOMIC_VAULT_DISABLE_GIT_CREDENTIAL_FILL_DETECTOR", "1")
        .env("AUTOMIC_VAULT_DISABLE_SUDO_DETECTOR", "1")
        .output()
        .unwrap()
}

fn av_scan_json(home: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_av"))
        .args(["scan", "--json"])
        .env("HOME", home)
        .env(
            "AUTOMIC_VAULT_TEST_BREW_TARGET",
            home.join("missing-opt-homebrew/bin/brew"),
        )
        .env("AUTOMIC_VAULT_DISABLE_GIT_CREDENTIAL_FILL_DETECTOR", "1")
        .env("AUTOMIC_VAULT_DISABLE_SUDO_DETECTOR", "1")
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

fn temp_home(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("av-cli-{label}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path).unwrap();
    path
}
