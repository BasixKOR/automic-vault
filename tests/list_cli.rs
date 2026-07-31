use std::fs;
use std::process::Command;

#[test]
fn av_list_prints_names_without_values() {
    let keychain = std::env::temp_dir().join(format!("av-list-cli-{}", std::process::id()));
    let _ = fs::remove_dir_all(&keychain);
    fs::create_dir_all(&keychain).unwrap();
    fs::write(keychain.join("Z_SECRET"), "must-not-leak").unwrap();
    fs::write(keychain.join("A_SECRET"), "also-secret").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_av"))
        .arg("list")
        .env("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR", &keychain)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "A_SECRET\nZ_SECRET\n"
    );
    assert!(output.stderr.is_empty());
    let _ = fs::remove_dir_all(keychain);
}
