use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn av_harden_aws_migrates_default_credentials() {
    let home = temp_home("harden-aws");
    let keychain = home.join("keychain");
    let aws = home.join(".aws");
    fs::create_dir_all(&keychain).unwrap();
    fs::create_dir_all(&aws).unwrap();
    fs::write(
        aws.join("credentials"),
        "[default]\naws_access_key_id = AKIA\naws_secret_access_key = secret\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_av"))
        .args(["harden", "aws"])
        .env("HOME", &home)
        .env("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR", &keychain)
        .env("AUTOMIC_VAULT_TEST_AWS_PATH", "/tmp/aws")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("╭─ harden aws"));
    assert!(stdout.contains("✓ moved credentials to Keychain"));
    assert!(stdout.contains("sudo av harden /tmp/aws"));
    assert_eq!(
        fs::read_to_string(keychain.join("AWS_ACCESS_KEY_ID")).unwrap(),
        "AKIA"
    );
    assert!(
        !fs::read_to_string(aws.join("credentials"))
            .unwrap()
            .contains("aws_secret_access_key")
    );

    let _ = fs::remove_dir_all(home);
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
