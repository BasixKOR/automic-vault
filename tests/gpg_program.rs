use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("av-gpg-{label}-{}-{nanos}", std::process::id()))
}

#[test]
fn signing_requests_stream_to_the_adjacent_av_command() {
    let directory = temp_dir("forwarding");
    fs::create_dir_all(&directory).unwrap();
    let adapter = directory.join("av-gpg");
    let av = directory.join("av");
    let args = directory.join("args");
    let captured_payload = directory.join("payload");
    fs::copy(env!("CARGO_BIN_EXE_av-gpg"), &adapter).unwrap();
    fs::write(
        &av,
        r#"#!/bin/sh
test "$1" = "gpg-sign" || exit 2
shift
printf '%s\n' "$@" > "$AV_GPG_TEST_ARGS"
cat > "$AV_GPG_TEST_PAYLOAD"
printf '%s\n' '[GNUPG:] SIG_CREATED D 1 10 00 0 0 0 0' >&2
printf '%s\n' '-----BEGIN PGP SIGNATURE-----' '' 'test' '-----END PGP SIGNATURE-----'
"#,
    )
    .unwrap();
    fs::set_permissions(&av, fs::Permissions::from_mode(0o755)).unwrap();
    let payload = b"tree 0000000000000000000000000000000000000000\n\nmessage\n";
    let mut child = Command::new(&adapter)
        .args(["--status-fd=2", "-bsau", "DEADBEEF"])
        .env("AV_GPG_TEST_ARGS", &args)
        .env("AV_GPG_TEST_PAYLOAD", &captured_payload)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(payload).unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("BEGIN PGP SIGNATURE")
    );
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("SIG_CREATED")
    );
    assert_eq!(fs::read(captured_payload).unwrap(), payload);
    assert_eq!(
        fs::read_to_string(args).unwrap(),
        "--status-fd=2\n-bsau\nDEADBEEF\n"
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn git_can_sign_through_an_app_path_containing_spaces() {
    let directory = temp_dir("app-path");
    let app_macos = directory.join("Automic Vault.app/Contents/MacOS");
    fs::create_dir_all(&app_macos).unwrap();
    let adapter = app_macos.join("av-gpg");
    let av = app_macos.join("av");
    fs::copy(env!("CARGO_BIN_EXE_av-gpg"), &adapter).unwrap();
    fs::write(
        &av,
        r#"#!/bin/sh
test "$1" = "gpg-sign" || exit 2
cat >/dev/null
printf '%s\n' '[GNUPG:] SIG_CREATED D 1 10 00 0 0 0 0' >&2
printf '%s\n' '-----BEGIN PGP SIGNATURE-----' '' 'dGVzdA==' '=n9Tk' '-----END PGP SIGNATURE-----'
"#,
    )
    .unwrap();
    fs::set_permissions(&av, fs::Permissions::from_mode(0o755)).unwrap();
    let repository = directory.join("repository");
    fs::create_dir(&repository).unwrap();
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&repository)
            .status()
            .unwrap()
            .success()
    );
    for (key, value) in [
        ("user.name", "av-gpg test"),
        ("user.email", "av-gpg@example.invalid"),
        ("user.signingKey", "DEADBEEF"),
        ("gpg.format", "openpgp"),
        ("gpg.program", adapter.to_str().unwrap()),
        ("commit.gpgSign", "true"),
    ] {
        assert!(
            Command::new("git")
                .args(["config", "--local", key, value])
                .current_dir(&repository)
                .status()
                .unwrap()
                .success()
        );
    }

    let commit = Command::new("git")
        .args([
            "commit",
            "--allow-empty",
            "--message",
            "signed through av-gpg",
        ])
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(
        commit.status.success(),
        "git commit failed: {}",
        String::from_utf8_lossy(&commit.stderr)
    );
    let object = Command::new("git")
        .args(["cat-file", "commit", "HEAD"])
        .current_dir(&repository)
        .output()
        .unwrap();
    assert!(object.status.success());
    assert!(
        String::from_utf8(object.stdout)
            .unwrap()
            .contains("gpgsig -----BEGIN PGP SIGNATURE-----")
    );
    fs::remove_dir_all(directory).unwrap();
}
