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
        "Automic Vault scan\n╭─ credential exposure audit\n│\n◇ No plaintext credential paths found\n│\n╰─ vault sealed\n"
    );
    assert_eq!(stderr(&output), "");

    let _ = fs::remove_dir_all(home);
}

#[test]
fn av_scan_reports_git_credentials() {
    let home = temp_home("triggered");
    fs::write(
        home.join(".git-credentials"),
        "https://user:token@example.com\n",
    )
    .unwrap();

    let output = av_scan(&home);

    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        format!(
            "Automic Vault scan\n╭─ credential exposure audit\n│\n◆ 1 finding requires attention\n│\n└─ 1. git\n│  severity HIGH\n│  homepage https://git-scm.com/\n│\n│  problem\n│  Git credential store contains plaintext credentials\n│\n│  affected files\n│  • {}:1\n│\n│  read more\n│  https://github.com/automic-vault/automic-vault/main/docs/securing-git.md\n│\n╰─ scan complete\n",
            home.join(".git-credentials").display()
        )
    );
    assert_eq!(stderr(&output), "");

    let _ = fs::remove_dir_all(home);
}

fn av_scan(home: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_av"))
        .arg("scan")
        .env("HOME", home)
        .env("AUTOMIC_VAULT_DISABLE_GIT_CREDENTIAL_FILL_DETECTOR", "1")
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
