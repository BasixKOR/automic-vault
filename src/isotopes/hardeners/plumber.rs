use std::fs::{self, OpenOptions};
use std::io::{self, IsTerminal, Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

use super::{
    HardenerCommand, HardenerDetection, HardenerDiagnostic, RequiredExecutable,
    SecretGateDescriptor, SecretGateRoute,
};

const AV_PATH: &str = "/usr/local/bin/av";
const MARKER: &str = "{\"automic_vault\":\"plumber-config-v1\"}\n";
const MAX_CONFIG_BYTES: u64 = 1024 * 1024;
const TEAM_IDENTIFIER: &str = "ZU76A67LGU";

pub(crate) fn run(stdout: &mut dyn Write, yes: bool) -> Result<(), String> {
    let testing = test_config_path().is_some();
    super::PrivilegeMode::Mixed.require_user("plumber", testing)?;
    if !testing {
        crate::secrets::ensure_plumber_helper_ready()?;
    }
    let path = config_path()?;
    let existed = path.exists();
    let original = read_config(&path)?;
    let (sanitized, credential) = sanitize_config(&original, existed)?;
    let target = target();
    let plan = super::isotope::plan(super::isotope::PLUMBER)?;

    writeln!(stdout, "╭─ harden plumber").ok();
    writeln!(stdout, "│").ok();
    plan.write(stdout, super::isotope::PLUMBER);
    writeln!(
        stdout,
        "├─ migrate the complete local config without printing it"
    )
    .ok();
    writeln!(stdout, "├─ leave only a fixed custody marker on disk").ok();
    writeln!(stdout, "│").ok();
    if !confirm(stdout, yes)? {
        writeln!(stdout, "╰─ cancelled").ok();
        return Ok(());
    }
    plan.apply(super::isotope::PLUMBER)?;
    verify_target(&target)?;
    if !testing {
        verify_command_resolution()?;
    }
    if let Some(credential) = credential {
        crate::secrets::store_secret_if_absent_or_equal(
            crate::cli::plumber_credential::SECRET_NAME,
            &credential,
        )?;
    }
    if original != sanitized || !path.exists() && !sanitized.is_empty() {
        write_config(&path, &sanitized)?;
    }
    writeln!(stdout, "╰─ hardened plumber").ok();
    super::write_secret_gate_notice(stdout, "plumber");
    Ok(())
}

pub(crate) fn detect() -> HardenerDetection {
    let testing = test_config_path().is_some();
    let target = target();
    let target_valid = verify_target(&target).is_ok();
    let command_resolves = test_config_path().is_some() || verify_command_resolution().is_ok();
    let path = config_path().ok();
    let config_valid = path.as_ref().is_some_and(|path| {
        read_config(path).is_ok_and(|contents| {
            sanitize_config(&contents, path.exists())
                .is_ok_and(|(sanitized, credential)| credential.is_none() && sanitized == contents)
        })
    });
    let hardened = target_valid && command_resolves && config_valid;
    let isotope = super::isotope::detect(super::isotope::PLUMBER)
        .commands
        .into_iter()
        .next()
        .and_then(|command| command.isotope);
    let command = HardenerCommand {
        name: "plumber".into(),
        hardened,
        stub_valid: true,
        stub_path: None,
        target_path: target.display().to_string(),
        required_paths: if testing {
            Vec::new()
        } else {
            vec![RequiredExecutable {
                name: "Automic Vault CLI",
                path: AV_PATH.into(),
            }]
        },
        stub_requirements: None,
        injected_keys: Vec::new(),
        assignment_keys: Vec::new(),
        isotope,
    };
    let mut detection = HardenerDetection::commands(hardened, vec![command]);
    detection.applicable = path.as_ref().is_some_and(|path| path.exists()) || target.exists();
    if target.exists() && !target_valid && !testing {
        detection.diagnostics.push(HardenerDiagnostic {
            kind: "plumber_target_invalid",
            message: verify_target(&target).unwrap_err(),
            remediation: "Rerun `av harden plumber` to install the signed Plumber Isotope.".into(),
            path: Some(target.display().to_string()),
        });
    }
    if target_valid && !command_resolves && !testing {
        detection.diagnostics.push(HardenerDiagnostic {
            kind: "plumber_command_shadowed",
            message: verify_command_resolution().unwrap_err(),
            remediation: "Rerun `av harden plumber` after correcting PATH.".into(),
            path: Some(target.display().to_string()),
        });
    }
    if let Some(path) = path.filter(|path| {
        path.exists()
            && match read_config(path) {
                Ok(contents) => match sanitize_config(&contents, true) {
                    Ok((sanitized, credential)) => credential.is_some() || sanitized != contents,
                    Err(_) => true,
                },
                Err(_) => true,
            }
    }) {
        detection.diagnostics.push(HardenerDiagnostic {
            kind: "plumber_plaintext_or_unsupported_config",
            message: "Plumber local config is not in the supported Hardened State.".into(),
            remediation: "Rerun `av harden plumber`; invalid JSON must be resolved manually."
                .into(),
            path: Some(path.display().to_string()),
        });
    }
    detection
}

pub(crate) fn secret_gate() -> SecretGateDescriptor {
    SecretGateDescriptor {
        id: "plumber",
        key_patterns: vec![crate::cli::plumber_credential::SECRET_NAME.into()],
        routes: vec![SecretGateRoute {
            operation: "plumber-get",
            script_path: None,
            target_path: target().display().to_string(),
            caller_identifiers: vec!["com.automicvault.av"],
            key_patterns: vec![crate::cli::plumber_credential::SECRET_NAME.into()],
            replace_existing_env: false,
            allow_missing_keys: false,
        }],
    }
}

fn sanitize_config(contents: &str, existed: bool) -> Result<(String, Option<String>), String> {
    if contents.is_empty() {
        if existed {
            return Err("existing Plumber config is empty".into());
        }
        return Ok((String::new(), None));
    }
    let parsed: Value = serde_json::from_str(contents)
        .map_err(|error| format!("invalid Plumber config JSON: {error}"))?;
    let object = parsed
        .as_object()
        .ok_or_else(|| "Plumber config must be a JSON object".to_string())?;
    if object.len() == 1
        && object.get("automic_vault").and_then(Value::as_str) == Some("plumber-config-v1")
    {
        return Ok((MARKER.into(), None));
    }
    let credential = crate::cli::plumber_credential::parse_config(contents)?;
    Ok((MARKER.into(), Some(credential)))
}

fn config_path() -> Result<PathBuf, String> {
    if let Some(path) = test_config_path() {
        return Ok(path);
    }
    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
    Ok(PathBuf::from(home).join(".batchsh/plumber.json"))
}

fn test_config_path() -> Option<PathBuf> {
    crate::test_env_var("AUTOMIC_VAULT_TEST_PLUMBER_CONFIG").map(PathBuf::from)
}

fn target() -> PathBuf {
    super::isotope::target(super::isotope::PLUMBER)
}

fn read_config(path: &Path) -> Result<String, String> {
    let file = match OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(String::new()),
        Err(error) => return Err(format!("failed to read {}: {error}", path.display())),
    };
    if !file
        .metadata()
        .is_ok_and(|metadata| metadata.file_type().is_file() && metadata.len() <= MAX_CONFIG_BYTES)
    {
        return Err(format!("refusing unsafe Plumber config {}", path.display()));
    }
    let mut contents = String::new();
    file.take(MAX_CONFIG_BYTES + 1)
        .read_to_string(&mut contents)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    if contents.len() as u64 > MAX_CONFIG_BYTES {
        return Err("Plumber config exceeds 1 MiB".into());
    }
    Ok(contents)
}

fn write_config(path: &Path, contents: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Plumber config has no parent".to_string())?;
    secure_directory(parent)?;
    let staging = parent.join(format!(
        ".plumber.json.av-{}.tmp",
        super::isotope::now_nanos()
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&staging)
            .map_err(|error| format!("failed to create {}: {error}", staging.display()))?;
        file.write_all(contents.as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("failed to write {}: {error}", staging.display()))?;
        fs::rename(&staging, path)
            .map_err(|error| format!("failed to replace {}: {error}", path.display()))?;
        OpenOptions::new()
            .read(true)
            .open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format!("failed to sync {}: {error}", parent.display()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(staging);
    }
    result
}

fn secure_directory(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        secure_directory(parent)?;
    }
    match fs::symlink_metadata(path) {
        Ok(metadata)
            if metadata.file_type().is_dir()
                && metadata.uid() == super::effective_uid()
                && metadata.permissions().mode() & 0o022 == 0 =>
        {
            Ok(())
        }
        Ok(_) => Err(format!(
            "refusing unsafe Plumber directory {}",
            path.display()
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::create_dir(path)
                .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
            fs::set_permissions(path, fs::Permissions::from_mode(0o700))
                .map_err(|error| format!("failed to protect {}: {error}", path.display()))
        }
        Err(error) => Err(format!("failed to inspect {}: {error}", path.display())),
    }
}

fn verify_command_resolution() -> Result<(), String> {
    let output = Command::new("/usr/bin/which")
        .arg("plumber")
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("failed to resolve Plumber: {error}"))?;
    let resolved = String::from_utf8(output.stdout)
        .ok()
        .filter(|_| output.status.success())
        .map(|path| PathBuf::from(path.trim()))
        .and_then(|path| path.canonicalize().ok());
    if resolved.is_some() && resolved == target().canonicalize().ok() {
        Ok(())
    } else {
        Err(format!(
            "your PATH does not resolve `plumber` to {}",
            target().display()
        ))
    }
}

pub(crate) fn verify_target(path: &Path) -> Result<(), String> {
    if test_config_path().is_some() {
        return path
            .exists()
            .then_some(())
            .ok_or_else(|| format!("test Plumber Target is missing: {}", path.display()));
    }
    let requirement = format!(
        "=identifier \"plumber\" and anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] exists and certificate leaf[field.1.2.840.113635.100.6.1.13] exists and certificate leaf[subject.OU] = \"{TEAM_IDENTIFIER}\""
    );
    let status = Command::new("/usr/bin/codesign")
        .args(["--verify", "--strict", "-R", &requirement])
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| format!("failed to verify {}: {error}", path.display()))?;
    if !status.success() {
        return Err(format!(
            "Plumber Target signature is invalid: {}",
            path.display()
        ));
    }
    let output = Command::new("/usr/bin/codesign")
        .args(["-d", "-vvv"])
        .arg(path)
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .output()
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
    let details = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if !output.status.success()
        || !details.contains("flags=0x10000(runtime)")
        || !details.contains(&format!("TeamIdentifier={TEAM_IDENTIFIER}"))
        || !details.contains("Timestamp=")
    {
        return Err(
            "Plumber Target lacks the required Developer ID Hardened Runtime identity".into(),
        );
    }
    let entitlements = Command::new("/usr/bin/codesign")
        .args(["-d", "--entitlements", ":-"])
        .arg(path)
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .output()
        .map_err(|error| format!("failed to inspect Plumber entitlements: {error}"))?;
    if !entitlements.status.success() || !entitlements.stdout.is_empty() {
        return Err("Plumber Target has unexpected code-signing entitlements".into());
    }
    Ok(())
}

fn confirm(stdout: &mut dyn Write, yes: bool) -> Result<bool, String> {
    if yes {
        writeln!(stdout, "◇ Continue? yes (--yes)").ok();
        return Ok(true);
    }
    write!(stdout, "◇ Continue? [y/N] ").ok();
    stdout
        .flush()
        .map_err(|error| format!("failed to flush prompt: {error}"))?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|error| format!("failed to read confirmation: {error}"))?;
    if !io::stdin().is_terminal() {
        writeln!(stdout).ok();
    }
    Ok(matches!(
        input.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_the_complete_config_and_rejects_non_objects() {
        let input =
            r#"{"token":"streamdal-token","connections":{"kafka":{"sasl_password":"password"}}}"#;
        let (sanitized, credential) = sanitize_config(input, true).unwrap();
        assert_eq!(sanitized, MARKER);
        let credential = credential.unwrap();
        assert!(credential.contains("streamdal-token"));
        assert!(credential.contains("password"));
        assert_eq!(
            sanitize_config(MARKER, true).unwrap(),
            (MARKER.into(), None)
        );
        assert!(sanitize_config("[]", true).is_err());
        assert!(sanitize_config("", true).is_err());
        assert_eq!(sanitize_config("", false).unwrap(), (String::new(), None));
    }
}
