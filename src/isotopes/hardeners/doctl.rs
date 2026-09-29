use super::{
    HardenerDetection, HardenerDiagnostic, RequiredExecutable, RequiredIdentity,
    SecretGateDescriptor, SecretGateRoute, StubRequirements, isotope,
};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

pub(crate) const VERSION: &str = "1.175.0-av.1";
pub(crate) const ROOT: &str = "/opt/av/doctl";
pub(crate) const TARGET: &str = "/opt/av/doctl/1.175.0-av.1/doctl";
pub(crate) const LAUNCHER: &str = "/usr/local/bin/doctl";
pub(crate) const STUB: &str = "#!/usr/local/bin/av __doctl\n";
const MAX_ARCHIVE: u64 = 32 * 1024 * 1024;
const MAX_BINARY: u64 = 64 * 1024 * 1024;

fn release() -> Result<(&'static str, &'static str, &'static str, &'static str), String> {
    match std::env::consts::ARCH {
        "aarch64" => Ok((
            "arm64",
            "a4565466d4541c7e223d338c7c7647b2bf22088e4103d3c534e050e6ae64f3b5",
            "9c762b2b1167dab0d799dffdd82c6c47c946b6451171c7d63e0de70e370c6fbd",
            "doctl",
        )),
        "x86_64" => Ok((
            "amd64",
            "cd227796d404f9dfb23d88bee85ce7d21ff5d78a974243050d378e783a5bf2a4",
            "6e6cf4e285b88ac9f679749e99e23319d305f02018840555b01387291f236dc5",
            "doctl",
        )),
        _ => Err("Automic Vault-signed doctl requires macOS arm64 or x86_64".into()),
    }
}

pub(crate) fn run(stdout: &mut dyn Write, yes: bool) -> Result<(), String> {
    super::PrivilegeMode::Mixed.require_user("doctl", false)?;
    super::env_wrapper::validate_privileged_av(Path::new("/usr/local/bin/av"))?;
    writeln!(
        stdout,
        "Install Automic Vault-signed doctl {VERSION} under {ROOT} and migrate the supported token."
    )
    .ok();
    if !yes {
        write!(stdout, "Continue? [y/N] ").map_err(|e| e.to_string())?;
        stdout.flush().map_err(|e| e.to_string())?;
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|e| e.to_string())?;
        if !matches!(input.trim(), "y" | "Y" | "yes") {
            return Ok(());
        }
    }
    let temporary = isotope::TemporaryDirectory::new_in(&std::env::temp_dir(), "doctl-release")?;
    let archive = temporary.path.join("doctl.tar.gz");
    download(&archive)?;
    let status = Command::new("/usr/bin/sudo")
        .args(["--", "/usr/local/bin/av", "__install-doctl-release"])
        .arg(&archive)
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("doctl installation failed; credentials retained".into());
    }
    verify_installation()?;
    let resolved = std::env::var_os("PATH")
        .and_then(|path| {
            std::env::split_paths(&path)
                .map(|dir| dir.join("doctl"))
                .find(|path| super::executable(path))
        })
        .and_then(|path| path.canonicalize().ok());
    if resolved.as_deref() != Some(Path::new(LAUNCHER)) {
        return Err(format!(
            "PATH must resolve doctl to {LAUNCHER}; adjust PATH and rerun `av harden doctl`. Credentials retained."
        ));
    }
    super::migrations::run("doctl").ok_or("missing doctl migration")??;
    writeln!(
        stdout,
        "Hardened doctl. Run `hash -r` to refresh command lookup."
    )
    .ok();
    super::write_secret_gate_notice(stdout, "doctl");
    Ok(())
}

fn download(destination: &Path) -> Result<(), String> {
    let (arch, hash, _, _) = release()?;
    let url = format!(
        "https://github.com/automic-vault/doctl/releases/download/v{VERSION}/doctl-{VERSION}-darwin-{arch}.tar.gz"
    );
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .https_only(true)
        .max_redirects(5)
        .timeout_global(Some(Duration::from_secs(180)))
        .build()
        .into();
    let mut body = agent
        .get(&url)
        .call()
        .map_err(|e| format!("doctl download failed: {e}"))?
        .into_body()
        .into_reader()
        .take(MAX_ARCHIVE + 1);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(destination)
        .map_err(|e| e.to_string())?;
    if std::io::copy(&mut body, &mut file).map_err(|e| e.to_string())? > MAX_ARCHIVE {
        return Err("doctl archive exceeds 32 MiB".into());
    }
    file.sync_all().map_err(|e| e.to_string())?;
    if isotope::sha256_file(destination)? != hash {
        return Err("Automic Vault-signed doctl archive checksum mismatch".into());
    }
    Ok(())
}

pub(crate) fn install_privileged(archive: &Path) -> Result<(), String> {
    if unsafe { libc::geteuid() } != 0 {
        return Err("doctl installation requires root".into());
    }
    if std::env::vars_os().any(|(key, _)| key.to_string_lossy().starts_with("AUTOMIC_VAULT_TEST_"))
    {
        return Err("test overrides are forbidden during doctl installation".into());
    }
    prepare_directory(Path::new(ROOT))?;
    let staging = isotope::TemporaryDirectory::new_in(Path::new(ROOT), "install")?;
    let trusted = staging.path.join("archive.tar.gz");
    let mut input = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(archive)
        .map_err(|e| e.to_string())?;
    if !input
        .metadata()
        .is_ok_and(|m| m.is_file() && m.len() <= MAX_ARCHIVE)
    {
        return Err("invalid doctl archive file".into());
    }
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&trusted)
        .map_err(|e| e.to_string())?;
    if std::io::copy(
        &mut Read::by_ref(&mut input).take(MAX_ARCHIVE + 1),
        &mut output,
    )
    .map_err(|e| e.to_string())?
        > MAX_ARCHIVE
    {
        return Err("doctl archive grew during copy".into());
    }
    output.sync_all().map_err(|e| e.to_string())?;
    if isotope::sha256_file(&trusted)? != release()?.1 {
        return Err("privileged doctl archive checksum mismatch".into());
    }
    let staged_binary = staging.path.join("doctl");
    extract_binary(&trusted, &staged_binary, true)?;
    fs::set_permissions(&staged_binary, fs::Permissions::from_mode(0o755))
        .map_err(|e| e.to_string())?;
    verify_binary(&staged_binary)?;
    prepare_directory(Path::new(TARGET).parent().unwrap())?;
    prepare_directory(Path::new("/usr/local/bin"))?;
    for directory in Path::new(TARGET)
        .parent()
        .unwrap()
        .ancestors()
        .chain(Path::new(LAUNCHER).parent().unwrap().ancestors())
    {
        verify_directory(directory)?;
    }
    let old = match fs::symlink_metadata(LAUNCHER) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.to_string()),
        Ok(_) => {
            protected(Path::new(LAUNCHER), 0o755, false)?;
            let bytes = fs::read(LAUNCHER).map_err(|e| e.to_string())?;
            if bytes != STUB.as_bytes() && !super::env_wrapper::is_legacy_doctl_stub(&bytes) {
                return Err("refusing to replace an unmanaged doctl command".into());
            }
            Some(bytes)
        }
    };
    // A failed launcher activation may leave an unused verified generation, never an unsigned fallback.
    fs::rename(&staged_binary, TARGET).map_err(|e| e.to_string())?;
    write_launcher(STUB.as_bytes())?;
    if let Err(error) = verify_installation() {
        let rollback = match old {
            Some(bytes) => write_launcher(&bytes),
            None => fs::remove_file(LAUNCHER).map_err(|e| e.to_string()),
        };
        return Err(match rollback {
            Ok(()) => error,
            Err(e) => format!("{error}; launcher rollback failed: {e}"),
        });
    }
    Ok(())
}

fn write_launcher(bytes: &[u8]) -> Result<(), String> {
    let staging =
        isotope::TemporaryDirectory::new_in(Path::new("/usr/local/bin"), "doctl-launcher")?;
    let path = staging.path.join("doctl");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o755)
        .open(&path)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    fs::rename(path, LAUNCHER).map_err(|e| e.to_string())
}

fn extract_binary(archive: &Path, destination: &Path, drop_privileges: bool) -> Result<(), String> {
    let member = "doctl";
    let mut command = Command::new("/usr/bin/tar");
    command
        .args(["-xzOf", "-", member])
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .stdin(File::open(archive).map_err(|e| e.to_string())?)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if drop_privileges {
        command.uid(65534).gid(65534);
    }
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(destination)
            .map_err(|e| e.to_string())?;
        let size = std::io::copy(
            &mut child.stdout.take().unwrap().take(MAX_BINARY + 1),
            &mut file,
        )
        .map_err(|e| e.to_string())?;
        if size == 0 || size > MAX_BINARY {
            return Err("invalid doctl binary size".into());
        }
        file.sync_all().map_err(|e| e.to_string())?;
        if !child.wait().map_err(|e| e.to_string())?.success() {
            return Err("doctl extraction failed".into());
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

pub(crate) fn verify_binary(path: &Path) -> Result<(), String> {
    let (_, _, hash, identifier) = release()?;
    if isotope::sha256_file(path)? != hash {
        return Err("doctl binary does not match the reviewed release".into());
    }
    let requirement = format!(
        "=identifier \"{identifier}\" and anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] exists and certificate leaf[field.1.2.840.113635.100.6.1.13] exists and certificate leaf[subject.OU] = \"ZU76A67LGU\""
    );
    let status = Command::new("/usr/bin/codesign")
        .args(["--verify", "--strict", "-R", &requirement])
        .arg(path)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("Automic Vault-signed doctl Developer ID signature is invalid".into());
    }
    // The pinned digest also binds the reviewed Hardened Runtime flags and empty entitlements.
    Ok(())
}

fn protected(path: &Path, mode: u32, directory: bool) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if metadata.uid() != 0
        || metadata.gid() != 0
        || metadata.mode() & 0o7777 != mode
        || if directory {
            !metadata.is_dir()
        } else {
            !metadata.is_file() || metadata.nlink() != 1
        }
    {
        return Err(format!(
            "unsafe doctl installation entry: {}",
            path.display()
        ));
    }
    Ok(())
}

pub(crate) fn verify_stub(path: &Path, contents: &str) -> Result<(), String> {
    protected(path, 0o755, false)?;
    if fs::read(path).ok().as_deref() != Some(contents.as_bytes()) {
        return Err("doctl launcher contents changed".into());
    }
    Ok(())
}

fn prepare_directory(path: &Path) -> Result<(), String> {
    for directory in path.ancestors().collect::<Vec<_>>().into_iter().rev() {
        match fs::symlink_metadata(directory) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(directory).map_err(|e| e.to_string())?;
                fs::set_permissions(directory, fs::Permissions::from_mode(0o755))
                    .map_err(|e| e.to_string())?;
            }
            Err(error) => return Err(error.to_string()),
            Ok(_) => {}
        }
        verify_directory(directory)?;
    }
    Ok(())
}

fn verify_directory(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_dir() || metadata.uid() != 0 || metadata.mode() & 0o022 != 0 {
        return Err(format!("unsafe doctl directory: {}", path.display()));
    }
    no_acl(path)
}

fn no_acl(path: &Path) -> Result<(), String> {
    // Reject ACLs rather than treating mode bits as proof against same-user writes.
    let result = Command::new("/bin/ls")
        .args(["-lde", "--"])
        .arg(path)
        .env_clear()
        .output()
        .map_err(|e| e.to_string())?;
    if !result.status.success()
        || result
            .stdout
            .split(|b| *b == b'\n')
            .filter(|line| !line.is_empty())
            .count()
            != 1
    {
        return Err(format!(
            "ACLs are not supported on doctl installation paths: {}",
            path.display()
        ));
    }
    Ok(())
}

pub(crate) fn verify_installation() -> Result<(), String> {
    for directory in Path::new(TARGET)
        .parent()
        .unwrap()
        .ancestors()
        .chain(Path::new(LAUNCHER).parent().unwrap().ancestors())
    {
        verify_directory(directory)?;
    }
    protected(Path::new(TARGET), 0o755, false)?;
    no_acl(Path::new(TARGET))?;
    verify_binary(Path::new(TARGET))?;
    verify_stub(Path::new(LAUNCHER), STUB)?;
    no_acl(Path::new(LAUNCHER))
}

pub(crate) fn detect() -> HardenerDetection {
    let verification = verify_installation();
    let mut detection = HardenerDetection::command(
        verification.is_ok(),
        "doctl",
        Some(LAUNCHER.into()),
        TARGET.into(),
    );
    detection.applicable |= Path::new(LAUNCHER).exists();
    detection.commands[0].injected_keys = vec!["DIGITALOCEAN_ACCESS_TOKEN".into()];
    detection.commands[0].required_paths = vec![RequiredExecutable {
        name: "Automic Vault CLI",
        path: "/usr/local/bin/av".into(),
    }];
    detection.commands[0].stub_requirements = Some(StubRequirements {
        mode: 0o755,
        owner: RequiredIdentity {
            name: "root",
            id: Some(0),
        },
        group: RequiredIdentity {
            name: "wheel",
            id: Some(0),
        },
    });
    if let Err(error) = verification {
        detection.diagnostics.push(HardenerDiagnostic {
            kind: "doctl-release",
            message: error,
            remediation: "Run `av harden doctl` to install the verified signed release.".into(),
            path: Some(TARGET.into()),
        });
    }
    detection
}

pub(crate) fn secret_gate() -> SecretGateDescriptor {
    SecretGateDescriptor {
        id: "doctl",
        key_patterns: vec!["DIGITALOCEAN_ACCESS_TOKEN".into()],
        routes: vec![SecretGateRoute {
            operation: "inject",
            script_path: None,
            target_path: TARGET.into(),
            caller_identifiers: vec!["com.automicvault.av"],
            key_patterns: vec!["DIGITALOCEAN_ACCESS_TOKEN".into()],
            replace_existing_env: false,
            allow_missing_keys: true,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_has_only_the_native_doctl_route() {
        let gates = super::super::secret_gates();
        let gates = gates
            .iter()
            .filter(|gate| gate.id == "doctl")
            .collect::<Vec<_>>();
        assert_eq!(gates.len(), 1);
        assert_eq!(gates[0].routes.len(), 1);
        let route = &gates[0].routes[0];
        assert_eq!(route.operation, "inject");
        assert_eq!(route.target_path, TARGET);
        assert!(route.script_path.is_none());
        assert_eq!(route.key_patterns, ["DIGITALOCEAN_ACCESS_TOKEN"]);
        assert!(!route.replace_existing_env);
        assert!(route.allow_missing_keys);
    }

    #[test]
    fn rejects_unsigned_or_changed_release() {
        let temp =
            isotope::TemporaryDirectory::new_in(&std::env::temp_dir(), "doctl-test").unwrap();
        let binary = temp.path.join("doctl");
        fs::write(&binary, b"not the reviewed binary").unwrap();
        assert!(verify_binary(&binary).is_err());
        assert!(verify_stub(&binary, STUB).is_err());
        std::os::unix::fs::symlink(&binary, temp.path.join("link")).unwrap();
        assert!(protected(&temp.path.join("link"), 0o755, false).is_err());
    }

    #[test]
    #[ignore = "requires AV_DOCTL_RELEASE_FIXTURE with the reviewed architecture archive"]
    fn official_release_extracts_and_verifies() {
        let archive =
            std::path::PathBuf::from(std::env::var_os("AV_DOCTL_RELEASE_FIXTURE").unwrap());
        assert_eq!(
            isotope::sha256_file(&archive).unwrap(),
            release().unwrap().1
        );
        let temp =
            isotope::TemporaryDirectory::new_in(&std::env::temp_dir(), "doctl-fixture").unwrap();
        let binary = temp.path.join("doctl");
        extract_binary(&archive, &binary, false).unwrap();
        verify_binary(&binary).unwrap();
        let mut bytes = fs::read(&binary).unwrap();
        bytes[1024] ^= 1;
        fs::write(&binary, bytes).unwrap();
        assert!(verify_binary(&binary).is_err());
    }
}
