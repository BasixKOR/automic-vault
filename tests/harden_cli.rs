use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn harden_installs_stub_then_migrates_direct_token() {
    let root = fixture("direct");
    let config = root.join("doctl.yaml");
    prepare(&root, "doctl");
    fs::write(&config, "access-token: do_secret\ncontext: default\n").unwrap();

    let output = av(&root, "doctl")
        .env("DIGITALOCEAN_CONFIG", &config)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(root.join("stubs/doctl").exists());
    assert_eq!(
        fs::read_to_string(root.join("keychain/DIGITALOCEAN_ACCESS_TOKEN")).unwrap(),
        "do_secret"
    );
    assert_eq!(
        fs::read_to_string(config).unwrap(),
        "access-token: \"\"\ncontext: default\n"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn harden_migrates_assignment_bundle() {
    let root = fixture("assignments");
    let config = root.join("edgerc");
    prepare(&root, "akamai");
    fs::write(
        &config,
        "[default]\nhost = example.invalid\nclient_token = tok\nclient_secret = sec\naccess_token = acc\n",
    )
    .unwrap();

    let output = av(&root, "akamai")
        .env("AKAMAI_EDGERC", &config)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        fs::read_to_string(root.join("keychain/AKAMAI_ENV_ASSIGNMENTS")).unwrap(),
        "AKAMAI_HOST=example.invalid\nAKAMAI_CLIENT_TOKEN=tok\nAKAMAI_CLIENT_SECRET=sec\nAKAMAI_ACCESS_TOKEN=acc"
    );
    assert!(
        !fs::read_to_string(config)
            .unwrap()
            .contains("client_token = tok")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_secret_storage_leaves_plaintext_usable_behind_stub() {
    let root = fixture("store-failure");
    let config = root.join("doctl.yaml");
    prepare(&root, "doctl");
    fs::write(&config, "access-token: do_secret\ncontext: default\n").unwrap();
    fs::write(root.join("keychain"), "not a directory").unwrap();

    let output = av(&root, "doctl")
        .env("DIGITALOCEAN_CONFIG", &config)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(root.join("stubs/doctl").exists());
    assert_eq!(
        fs::read_to_string(config).unwrap(),
        "access-token: do_secret\ncontext: default\n"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_stub_install_does_not_migrate_credentials() {
    let root = fixture("install-failure");
    let config = root.join("doctl.yaml");
    prepare(&root, "doctl");
    fs::write(&config, "access-token: do_secret\ncontext: default\n").unwrap();
    fs::set_permissions(root.join("stubs"), fs::Permissions::from_mode(0o555)).unwrap();

    let output = av(&root, "doctl")
        .env("DIGITALOCEAN_CONFIG", &config)
        .output()
        .unwrap();

    fs::set_permissions(root.join("stubs"), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(!output.status.success());
    assert_eq!(
        fs::read_to_string(config).unwrap(),
        "access-token: do_secret\ncontext: default\n"
    );
    assert!(!root.join("keychain/DIGITALOCEAN_ACCESS_TOKEN").exists());
    fs::remove_dir_all(root).unwrap();
}

fn av(root: &Path, hardener: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_av"));
    command.args(["harden", hardener, "--yes"]);
    command.env("HOME", root.join("home"));
    command.env("AUTOMIC_VAULT_TEST_EUID", "0");
    command.env(
        "AUTOMIC_VAULT_TEST_ENV_WRAPPER_TARGET_DIR",
        root.join("targets"),
    );
    command.env(
        "AUTOMIC_VAULT_TEST_ENV_WRAPPER_STUB_DIR",
        root.join("stubs"),
    );
    command.env("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR", root.join("keychain"));
    command
}

fn prepare(root: &Path, command: &str) {
    fs::create_dir_all(root.join("targets")).unwrap();
    fs::create_dir_all(root.join("stubs")).unwrap();
    fs::create_dir_all(root.join("home")).unwrap();
    fs::write(root.join("targets").join(command), "#!/bin/sh\n").unwrap();
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn fixture(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("av-harden-{label}-{}-{nanos}", std::process::id()))
}
