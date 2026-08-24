use std::fs::{self, OpenOptions};
use std::io::{self, IsTerminal, Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::{
    HardenerCommand, HardenerDetection, HardenerDiagnostic, RequiredExecutable,
    SecretGateDescriptor, SecretGateRoute,
};

const AV_PATH: &str = "/usr/local/bin/av";
const MARKER: &str = "@av";
const MAX_CONFIG_BYTES: u64 = 1024 * 1024;
const TEAM_IDENTIFIER: &str = "ZU76A67LGU";

struct ConfigState {
    path: PathBuf,
    original: String,
    sanitized: String,
    credential: Option<Credential>,
}

struct Credential {
    bridge: String,
    value: String,
}

pub(crate) fn run(stdout: &mut dyn Write, yes: bool) -> Result<(), String> {
    let testing = test_config_path().is_some();
    super::PrivilegeMode::Mixed.require_user("openhue-cli", testing)?;
    if !testing {
        crate::secrets::ensure_openhue_helper_ready()?;
    }
    let configs = config_paths()?
        .into_iter()
        .map(|path| {
            let original = read_config(&path)?;
            let (sanitized, credential) = sanitize_config(&original)?;
            Ok(ConfigState {
                path,
                original,
                sanitized,
                credential,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let target = target();
    let plan = super::isotope::plan(super::isotope::OPENHUE)?;
    let brew_conflict = !testing && homebrew_formula_installed();

    writeln!(stdout, "╭─ harden openhue-cli").ok();
    writeln!(stdout, "│").ok();
    plan.write(stdout, super::isotope::OPENHUE);
    if brew_conflict {
        writeln!(stdout, "├─ unlink the Homebrew openhue-cli formula").ok();
    }
    writeln!(
        stdout,
        "├─ migrate the Hue application key without printing it"
    )
    .ok();
    writeln!(
        stdout,
        "├─ keep only bridge metadata and an @av marker on disk"
    )
    .ok();
    writeln!(stdout, "│").ok();
    if !confirm(stdout, yes)? {
        writeln!(stdout, "╰─ cancelled").ok();
        return Ok(());
    }
    plan.apply(super::isotope::OPENHUE)?;
    verify_target(&target)?;
    if brew_conflict {
        unlink_homebrew()?;
    }
    if !testing {
        verify_command_resolution()?;
    }
    if let Some(credential) = configs.iter().find_map(|config| config.credential.as_ref()) {
        crate::cli::openhue_credential::validate_bridge(&credential.bridge)?;
        crate::secrets::store_secret_if_absent_or_equal(
            crate::cli::openhue_credential::SECRET_NAME,
            &credential.value,
        )?;
    }
    for config in configs {
        if config.original != config.sanitized
            || !config.path.exists() && !config.sanitized.is_empty()
        {
            write_config(&config.path, &config.sanitized)?;
        }
    }
    writeln!(stdout, "╰─ hardened openhue-cli").ok();
    super::write_secret_gate_notice(stdout, "openhue-cli");
    Ok(())
}

pub(crate) fn detect() -> HardenerDetection {
    let target = target();
    let target_valid = verify_target(&target).is_ok();
    let command_resolves = test_config_path().is_some() || verify_command_resolution().is_ok();
    let configs = config_paths().unwrap_or_default();
    let config_valid = !configs.is_empty()
        && configs.iter().all(|path| {
            read_config(path).is_ok_and(|contents| {
                sanitize_config(&contents).is_ok_and(|(sanitized, credential)| {
                    credential.is_none() && sanitized == contents
                })
            })
        });
    let hardened = target_valid && command_resolves && config_valid;
    let isotope = super::isotope::detect(super::isotope::OPENHUE)
        .commands
        .into_iter()
        .next()
        .and_then(|command| command.isotope);
    let command = HardenerCommand {
        name: "openhue-cli".into(),
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
    detection.applicable = configs.iter().any(|path| path.exists()) || target.exists();
    if target.exists() && !target_valid && test_config_path().is_none() {
        detection.diagnostics.push(HardenerDiagnostic {
            kind: "openhue_cli_target_invalid",
            message: verify_target(&target).unwrap_err(),
            remediation: "Rerun `av harden openhue-cli` to install the signed openhue-cli Isotope."
                .into(),
            path: Some(target.display().to_string()),
        });
    }
    if target_valid && !command_resolves && test_config_path().is_none() {
        detection.diagnostics.push(HardenerDiagnostic {
            kind: "openhue_cli_command_shadowed",
            message: verify_command_resolution().unwrap_err(),
            remediation: "Rerun `av harden openhue-cli` after correcting PATH.".into(),
            path: Some(target.display().to_string()),
        });
    }
    if let Some(path) = configs.iter().find(|path| {
        path.exists()
            && match read_config(path) {
                Ok(contents) => match sanitize_config(&contents) {
                    Ok((sanitized, credential)) => credential.is_some() || sanitized != contents,
                    Err(_) => true,
                },
                Err(_) => true,
            }
    }) {
        detection.diagnostics.push(HardenerDiagnostic {
            kind: "openhue_cli_plaintext_or_unsupported_session",
            message: "OpenHue config is not in the supported Hardened State.".into(),
            remediation:
                "Rerun `av harden openhue-cli`; unsupported fields must be resolved manually."
                    .into(),
            path: Some(path.display().to_string()),
        });
    }
    detection
}

pub(crate) fn secret_gate() -> SecretGateDescriptor {
    SecretGateDescriptor {
        id: "openhue-cli",
        key_patterns: vec![crate::cli::openhue_credential::SECRET_NAME.into()],
        routes: vec![SecretGateRoute {
            operation: "openhue-get",
            script_path: None,
            target_path: target().display().to_string(),
            caller_identifiers: vec!["com.automicvault.av"],
            key_patterns: vec![crate::cli::openhue_credential::SECRET_NAME.into()],
            replace_existing_env: false,
            allow_missing_keys: false,
        }],
    }
}

fn sanitize_config(contents: &str) -> Result<(String, Option<Credential>), String> {
    if contents.is_empty() {
        return Ok((String::new(), None));
    }
    let mut bridge = None;
    let mut application_key = None;
    let mut sanitized = String::with_capacity(contents.len());
    for line in contents.split_inclusive('\n') {
        let body = line.strip_suffix('\n').unwrap_or(line);
        let trimmed = body.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            sanitized.push_str(line);
            continue;
        }
        let (name, raw) = trimmed
            .split_once(':')
            .ok_or_else(|| "OpenHue config contains unsupported YAML".to_string())?;
        let name = name.trim().to_ascii_lowercase();
        if !matches!(name.as_str(), "bridge" | "key" | "log_level") {
            return Err(format!(
                "OpenHue config contains unsupported field `{name}`"
            ));
        }
        let value = yaml_scalar(raw.trim())?;
        match name.as_str() {
            "bridge" if bridge.replace(value.to_string()).is_some() => {
                return Err("OpenHue config contains duplicate `bridge` fields".into());
            }
            "key" if application_key.replace(value.to_string()).is_some() => {
                return Err("OpenHue config contains duplicate `key` fields".into());
            }
            "key" if !value.is_empty() && value != MARKER => {
                let prefix_len = body.len() - body.trim_start().len();
                sanitized.push_str(&body[..prefix_len]);
                sanitized.push_str(name.as_str());
                sanitized.push_str(": '@av'");
                if line.ends_with('\n') {
                    sanitized.push('\n');
                }
                continue;
            }
            _ => {}
        }
        sanitized.push_str(line);
    }
    let Some(application_key) = application_key else {
        return Ok((contents.to_string(), None));
    };
    if application_key.is_empty() {
        return Ok((contents.to_string(), None));
    }
    let bridge = bridge.ok_or_else(|| "OpenHue config has a key but no bridge".to_string())?;
    crate::cli::openhue_credential::validate_bridge(&bridge)?;
    if application_key == MARKER {
        return Ok((contents.to_string(), None));
    }
    let value = crate::cli::openhue_credential::validate_key(&application_key)?;
    Ok((sanitized, Some(Credential { bridge, value })))
}

fn yaml_scalar(value: &str) -> Result<&str, String> {
    if value.is_empty() {
        return Ok("");
    }
    let bytes = value.as_bytes();
    if bytes.len() >= 2
        && ((bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\'')
            || (bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"'))
    {
        let value = &value[1..value.len() - 1];
        if value.contains(['\'', '"', '\\', '\n', '\r', '\0']) {
            return Err("OpenHue config contains an unsupported YAML scalar".into());
        }
        return Ok(value);
    }
    if value.contains(['#', '[', ']', '{', '}', ',', '\n', '\r', '\0']) {
        return Err("OpenHue config contains an unsupported YAML scalar".into());
    }
    Ok(value)
}

fn config_paths() -> Result<Vec<PathBuf>, String> {
    if let Some(path) = test_config_path() {
        return Ok(vec![path]);
    }
    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
    if let Some(root) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Ok(vec![PathBuf::from(root).join("openhue/config.yaml")]);
    }
    Ok(vec![PathBuf::from(home).join(".openhue/config.yaml")])
}

fn test_config_path() -> Option<PathBuf> {
    crate::test_env_var("AUTOMIC_VAULT_TEST_OPENHUE_CLI_CONFIG").map(PathBuf::from)
}

fn target() -> PathBuf {
    super::isotope::target(super::isotope::OPENHUE)
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
        return Err(format!(
            "refusing unsafe openhue-cli config {}",
            path.display()
        ));
    }
    let mut contents = String::new();
    file.take(MAX_CONFIG_BYTES + 1)
        .read_to_string(&mut contents)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    if contents.len() as u64 > MAX_CONFIG_BYTES {
        return Err("OpenHue config exceeds 1 MiB".into());
    }
    Ok(contents)
}

fn write_config(path: &Path, contents: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "openhue-cli config has no parent".to_string())?;
    secure_directory(parent)?;
    let staging = parent.join(format!(
        ".config.yaml.av-{}.tmp",
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
            "refusing unsafe openhue-cli directory {}",
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

fn homebrew_formula_installed() -> bool {
    ["/opt/homebrew", "/usr/local"].into_iter().any(|prefix| {
        let linked = Path::new(prefix).join("bin/openhue");
        let formula = Path::new(prefix).join("opt/openhue-cli/bin/openhue");
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
        .ok_or_else(|| "Homebrew openhue-cli is installed but brew is unavailable".to_string())?;
    let status = Command::new(&brew)
        .args(["unlink", "openhue-cli"])
        .status()
        .map_err(|error| format!("failed to run {}: {error}", brew.display()))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("failed to unlink Homebrew openhue-cli: {status}"))
}

fn verify_command_resolution() -> Result<(), String> {
    let output = Command::new("/usr/bin/which")
        .arg("openhue")
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("failed to resolve OpenHue CLI: {error}"))?;
    let resolved = String::from_utf8(output.stdout)
        .ok()
        .filter(|_| output.status.success())
        .map(|path| PathBuf::from(path.trim()))
        .and_then(|path| path.canonicalize().ok());
    if resolved.is_some() && resolved == target().canonicalize().ok() {
        Ok(())
    } else {
        Err(format!(
            "your PATH does not resolve `openhue` to {}",
            target().display()
        ))
    }
}

pub(crate) fn verify_target(path: &Path) -> Result<(), String> {
    if test_config_path().is_some() {
        return path
            .exists()
            .then_some(())
            .ok_or_else(|| format!("test openhue-cli Target is missing: {}", path.display()));
    }
    let requirement = format!(
        "=identifier \"openhue\" and anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] exists and certificate leaf[field.1.2.840.113635.100.6.1.13] exists and certificate leaf[subject.OU] = \"{TEAM_IDENTIFIER}\""
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
            "openhue-cli Target signature is invalid: {}",
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
            "openhue-cli Target lacks the required Developer ID Hardened Runtime identity".into(),
        );
    }
    let entitlements = Command::new("/usr/bin/codesign")
        .args(["-d", "--entitlements", ":-"])
        .arg(path)
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .output()
        .map_err(|error| format!("failed to inspect openhue-cli entitlements: {error}"))?;
    if !entitlements.status.success() || !entitlements.stdout.is_empty() {
        return Err("openhue-cli Target has unexpected code-signing entitlements".into());
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
    fn migrates_application_key_and_rejects_unsupported_yaml() {
        let input = "bridge: 192.0.2.10\nkey: application-key\nlog_level: info\n";
        let (sanitized, credential) = sanitize_config(input).unwrap();
        let credential = credential.unwrap();
        assert_eq!(credential.bridge, "192.0.2.10");
        assert_eq!(credential.value, "application-key");
        assert!(sanitized.contains("key: '@av'"));
        assert!(sanitize_config("bridge: 192.0.2.10\nfuture: value\nkey: secret\n").is_err());
        assert!(sanitize_config("bridge: first\nbridge: second\nkey: secret\n").is_err());
        assert_eq!(sanitize_config(&sanitized).unwrap().0, sanitized);
    }
}
