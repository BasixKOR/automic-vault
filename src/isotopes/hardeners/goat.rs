use std::fs::{self, OpenOptions};
use std::io::{self, IsTerminal, Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

use super::{
    HardenerCommand, HardenerDetection, HardenerDiagnostic, RequiredExecutable,
    SecretGateDescriptor, SecretGateRoute,
};

const AV_PATH: &str = "/usr/local/bin/av";
const MARKER: &str = "@av";
const MAX_CONFIG_BYTES: u64 = 1024 * 1024;
const TEAM_IDENTIFIER: &str = "ZU76A67LGU";

struct Credential {
    did: String,
    pds: String,
    value: String,
}

pub(crate) fn run(stdout: &mut dyn Write, yes: bool) -> Result<(), String> {
    let testing = test_config_path().is_some();
    super::PrivilegeMode::Mixed.require_user("goat", testing)?;
    if !testing {
        crate::secrets::ensure_goat_helper_ready()?;
    }
    let path = config_path()?;
    let original = read_config(&path)?;
    let (sanitized, credential) = sanitize_config(&original)?;
    let target = target();
    let plan = super::isotope::plan(super::isotope::GOAT)?;
    let brew_conflict = !testing && homebrew_formula_installed();

    writeln!(stdout, "╭─ harden goat").ok();
    writeln!(stdout, "│").ok();
    plan.write(stdout, super::isotope::GOAT);
    if brew_conflict {
        writeln!(stdout, "├─ unlink the Homebrew goat formula").ok();
    }
    writeln!(
        stdout,
        "├─ migrate the goat auth session without printing it"
    )
    .ok();
    writeln!(stdout, "├─ keep only DID and PDS metadata on disk").ok();
    writeln!(stdout, "│").ok();
    if !confirm(stdout, yes)? {
        writeln!(stdout, "╰─ cancelled").ok();
        return Ok(());
    }
    plan.apply(super::isotope::GOAT)?;
    verify_target(&target)?;
    if brew_conflict {
        unlink_homebrew()?;
    }
    if !testing {
        verify_command_resolution()?;
    }
    if let Some(credential) = credential {
        crate::secrets::store_secret_if_absent_or_equal(
            &crate::cli::goat_credential::secret_name(&credential.did, &credential.pds),
            &credential.value,
        )?;
    }
    if original != sanitized || !path.exists() && !sanitized.is_empty() {
        write_config(&path, &sanitized)?;
    }
    writeln!(stdout, "╰─ hardened goat").ok();
    super::write_secret_gate_notice(stdout, "goat");
    Ok(())
}

pub(crate) fn detect() -> HardenerDetection {
    let target = target();
    let target_valid = verify_target(&target).is_ok();
    let command_resolves = test_config_path().is_some() || verify_command_resolution().is_ok();
    let config = config_path().ok();
    let config_valid = config.as_deref().is_some_and(|path| {
        read_config(path).is_ok_and(|contents| {
            sanitize_config(&contents)
                .is_ok_and(|(sanitized, credential)| credential.is_none() && sanitized == contents)
        })
    });
    let hardened = target_valid && command_resolves && config_valid;
    let isotope = super::isotope::detect(super::isotope::GOAT)
        .commands
        .into_iter()
        .next()
        .and_then(|command| command.isotope);
    let command = HardenerCommand {
        name: "goat".into(),
        hardened,
        stub_valid: true,
        stub_path: None,
        target_path: target.display().to_string(),
        required_paths: if test_config_path().is_some() {
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
    detection.applicable = config.as_deref().is_some_and(Path::exists) || target.exists();
    if target.exists() && !target_valid && test_config_path().is_none() {
        detection.diagnostics.push(HardenerDiagnostic {
            kind: "goat_target_invalid",
            message: verify_target(&target).unwrap_err(),
            remediation: "Rerun `av harden goat` to install the signed goat Isotope.".into(),
            path: Some(target.display().to_string()),
        });
    }
    if target_valid && !command_resolves && test_config_path().is_none() {
        detection.diagnostics.push(HardenerDiagnostic {
            kind: "goat_command_shadowed",
            message: verify_command_resolution().unwrap_err(),
            remediation: "Rerun `av harden goat` after correcting PATH.".into(),
            path: Some(target.display().to_string()),
        });
    }
    if let Some(path) = config
        && path.exists()
        && !config_valid
    {
        detection.diagnostics.push(HardenerDiagnostic {
            kind: "goat_plaintext_or_unsupported_session",
            message: "goat auth state is not in the supported Hardened State.".into(),
            remediation: "Rerun `av harden goat`; unsupported fields must be resolved manually."
                .into(),
            path: Some(path.display().to_string()),
        });
    }
    detection
}

pub(crate) fn secret_gate() -> SecretGateDescriptor {
    SecretGateDescriptor {
        id: "goat",
        key_patterns: vec!["GOAT_AUTH_SESSION_*".into()],
        routes: vec![SecretGateRoute {
            operation: "goat-get",
            script_path: None,
            target_path: target().display().to_string(),
            caller_identifiers: vec!["com.automicvault.av"],
            key_patterns: vec!["GOAT_AUTH_SESSION_*".into()],
            replace_existing_env: false,
            allow_missing_keys: false,
        }],
    }
}

fn sanitize_config(contents: &str) -> Result<(String, Option<Credential>), String> {
    if contents.is_empty() {
        return Ok((String::new(), None));
    }
    let mut value: Value = serde_json::from_str(contents)
        .map_err(|error| format!("invalid goat auth session JSON: {error}"))?;
    let object = value
        .as_object_mut()
        .filter(|object| {
            object.len() == 5
                && ["did", "password", "access_token", "session_token", "pds"]
                    .iter()
                    .all(|key| object.get(*key).is_some_and(Value::is_string))
        })
        .ok_or_else(|| {
            "goat auth session must contain exactly the supported string fields".to_string()
        })?;
    let did = crate::cli::goat_credential::normalize_did(object["did"].as_str().unwrap())?;
    let pds = crate::cli::oxide_credential::normalize_host(object["pds"].as_str().unwrap())?;
    let marker_count = ["password", "access_token", "session_token"]
        .iter()
        .filter(|key| object[**key].as_str() == Some(MARKER))
        .count();
    if marker_count != 0 && marker_count != 3 {
        return Err("goat auth session is only partially migrated".into());
    }
    let credential = if marker_count == 3 {
        None
    } else {
        let raw = json!({
            "password": object["password"],
            "access_token": object["access_token"],
            "session_token": object["session_token"],
        })
        .to_string();
        Some(Credential {
            did: did.clone(),
            pds: pds.clone(),
            value: crate::cli::goat_credential::parse_secrets(&raw)?,
        })
    };
    object.insert("did".into(), Value::String(did));
    object.insert("pds".into(), Value::String(pds));
    for key in ["password", "access_token", "session_token"] {
        object.insert(key.into(), Value::String(MARKER.into()));
    }
    let mut sanitized = serde_json::to_string_pretty(&value)
        .map_err(|error| format!("failed to encode goat auth metadata: {error}"))?;
    sanitized.push('\n');
    Ok((sanitized, credential))
}

fn config_path() -> Result<PathBuf, String> {
    if let Some(path) = test_config_path() {
        return Ok(path);
    }
    if let Some(state) = std::env::var_os("XDG_STATE_HOME").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(state).join("goat/auth-session.json"));
    }
    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
    Ok(PathBuf::from(home).join(".local/state/goat/auth-session.json"))
}

fn test_config_path() -> Option<PathBuf> {
    crate::test_env_var("AUTOMIC_VAULT_TEST_GOAT_CONFIG").map(PathBuf::from)
}

fn target() -> PathBuf {
    super::isotope::target(super::isotope::GOAT)
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
        return Err(format!("refusing unsafe goat config {}", path.display()));
    }
    let mut contents = String::new();
    file.take(MAX_CONFIG_BYTES + 1)
        .read_to_string(&mut contents)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    if contents.len() as u64 > MAX_CONFIG_BYTES {
        return Err("goat auth session exceeds 1 MiB".into());
    }
    Ok(contents)
}

fn write_config(path: &Path, contents: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "goat config has no parent".to_string())?;
    secure_directory(parent)?;
    let staging = parent.join(format!(
        ".auth-session.json.av-{}.tmp",
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
        Ok(_) => Err(format!("refusing unsafe goat directory {}", path.display())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::create_dir(path)
                .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
            fs::set_permissions(path, fs::Permissions::from_mode(0o700))
                .map_err(|error| format!("failed to protect {}: {error}", path.display()))
        }
        Err(error) => Err(format!("failed to inspect {}: {error}", path.display())),
    }
}

fn homebrew_formula_installed() -> bool {
    ["/opt/homebrew", "/usr/local"].into_iter().any(|prefix| {
        let linked = Path::new(prefix).join("bin/goat");
        let formula = Path::new(prefix).join("opt/goat/bin/goat");
        linked.canonicalize().ok().is_some_and(|linked| {
            formula
                .canonicalize()
                .ok()
                .is_some_and(|formula| linked == formula)
        })
    })
}

fn unlink_homebrew() -> Result<(), String> {
    let brew = ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"]
        .into_iter()
        .map(PathBuf::from)
        .find(|path| path.is_file())
        .ok_or_else(|| "Homebrew goat is installed but brew is unavailable".to_string())?;
    let status = Command::new(&brew)
        .args(["unlink", "goat"])
        .status()
        .map_err(|error| format!("failed to run {}: {error}", brew.display()))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("failed to unlink Homebrew goat: {status}"))
}

fn verify_command_resolution() -> Result<(), String> {
    let output = Command::new("/usr/bin/which")
        .arg("goat")
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("failed to resolve goat: {error}"))?;
    let resolved = String::from_utf8(output.stdout)
        .ok()
        .filter(|_| output.status.success())
        .map(|path| PathBuf::from(path.trim()))
        .and_then(|path| path.canonicalize().ok());
    if resolved.is_some() && resolved == target().canonicalize().ok() {
        Ok(())
    } else {
        Err(format!(
            "your PATH does not resolve `goat` to {}",
            target().display()
        ))
    }
}

pub(crate) fn verify_target(path: &Path) -> Result<(), String> {
    if test_config_path().is_some() {
        return path
            .exists()
            .then_some(())
            .ok_or_else(|| format!("test goat Target is missing: {}", path.display()));
    }
    let requirement = format!(
        "=identifier \"goat\" and anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] exists and certificate leaf[field.1.2.840.113635.100.6.1.13] exists and certificate leaf[subject.OU] = \"{TEAM_IDENTIFIER}\""
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
            "goat Target signature is invalid: {}",
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
        return Err("goat Target lacks the required Developer ID Hardened Runtime identity".into());
    }
    let entitlements = Command::new("/usr/bin/codesign")
        .args(["-d", "--entitlements", ":-"])
        .arg(path)
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .output()
        .map_err(|error| format!("failed to inspect goat entitlements: {error}"))?;
    if !entitlements.status.success() || !entitlements.stdout.is_empty() {
        return Err("goat Target has unexpected code-signing entitlements".into());
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
    fn migrates_complete_session_and_rejects_unknown_fields() {
        let input = r#"{"did":"did:plc:abc","password":"pass","access_token":"access","session_token":"refresh","pds":"https://PDS.example/"}"#;
        let (sanitized, credential) = sanitize_config(input).unwrap();
        let credential = credential.unwrap();
        assert_eq!(credential.did, "did:plc:abc");
        assert_eq!(credential.pds, "https://pds.example");
        assert!(sanitized.contains("\"password\": \"@av\""));
        assert!(sanitize_config(&input.replace("\"pds\"", "\"future\":1,\"pds\"")).is_err());
        assert!(sanitize_config(&input.replace("\"pass\"", "\"@av\"")).is_err());
    }
}
