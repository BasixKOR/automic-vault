use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn av_brew_stub_binary_is_built() {
    let output = Command::new(env!("CARGO_BIN_EXE_av-brew-stub"))
        .arg("--automic-vault-brew-stub-marker")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "AUTOMIC_VAULT_BREW_STUB_V11"
    );
}

#[test]
fn av_brew_stub_does_not_require_a_readable_current_directory() {
    let root = std::env::temp_dir().join(format!(
        "av-brew-stub-cwd-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(root.join("cwd")).unwrap();

    let output = Command::new("/bin/sh")
        .args([
            "-c",
            "cd \"$1/cwd\" && chmod 000 \"$1\" && exec \"$2\" --version",
            "sh",
        ])
        .arg(&root)
        .arg(env!("CARGO_BIN_EXE_av-brew-stub"))
        .output()
        .unwrap();

    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    fs::remove_dir_all(&root).unwrap();
    assert!(
        !String::from_utf8_lossy(&output.stderr)
            .contains("failed to read current directory: Permission denied")
    );
}
