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

struct ConfigState {
    path: PathBuf,
    original: String,
    sanitized: String,
    credential: Option<String>,
}

pub(crate) fn run(stdout: &mut dyn Write, yes: bool) -> Result<(), String> {
    let testing = test_config_path().is_some();
    super::PrivilegeMode::Mixed.require_user("ordercli", testing)?;
    if !testing {
        crate::secrets::ensure_ordercli_helper_ready()?;
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
    let credentials = configs
        .iter()
        .filter_map(|config| config.credential.as_deref())
        .collect::<std::collections::BTreeSet<_>>();
    if credentials.len() > 1 {
        return Err("ordercli config files contain conflicting credential bundles".into());
    }
    let target = target();
    let plan = super::isotope::plan(super::isotope::ORDERCLI)?;
    let brew_conflict = !testing && homebrew_formula_installed();

    writeln!(stdout, "╭─ harden ordercli").ok();
    writeln!(stdout, "│").ok();
    plan.write(stdout, super::isotope::ORDERCLI);
    if brew_conflict {
        writeln!(stdout, "├─ unlink the Homebrew ordercli formula").ok();
    }
    writeln!(
        stdout,
        "├─ migrate the ordercli auth session without printing it"
    )
    .ok();
    writeln!(
        stdout,
        "├─ keep only provider metadata and @av markers on disk"
    )
    .ok();
    writeln!(stdout, "│").ok();
    if !confirm(stdout, yes)? {
        writeln!(stdout, "╰─ cancelled").ok();
        return Ok(());
    }
    plan.apply(super::isotope::ORDERCLI)?;
    verify_target(&target)?;
    if brew_conflict {
        unlink_homebrew()?;
    }
    if !testing {
        verify_command_resolution()?;
    }
    if let Some(credential) = credentials.first() {
        crate::secrets::store_secret_if_absent_or_equal(
            crate::cli::ordercli_credential::SECRET_NAME,
            credential,
        )?;
    }
    for config in configs {
        if config.original != config.sanitized
            || !config.path.exists() && !config.sanitized.is_empty()
        {
            write_config(&config.path, &config.sanitized)?;
        }
    }
    writeln!(stdout, "╰─ hardened ordercli").ok();
    super::write_secret_gate_notice(stdout, "ordercli");
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
    let isotope = super::isotope::detect(super::isotope::ORDERCLI)
        .commands
        .into_iter()
        .next()
        .and_then(|command| command.isotope);
    let command = HardenerCommand {
        name: "ordercli".into(),
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
            kind: "ordercli_target_invalid",
            message: verify_target(&target).unwrap_err(),
            remediation: "Rerun `av harden ordercli` to install the signed ordercli Isotope."
                .into(),
            path: Some(target.display().to_string()),
        });
    }
    if target_valid && !command_resolves && test_config_path().is_none() {
        detection.diagnostics.push(HardenerDiagnostic {
            kind: "ordercli_command_shadowed",
            message: verify_command_resolution().unwrap_err(),
            remediation: "Rerun `av harden ordercli` after correcting PATH.".into(),
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
            kind: "ordercli_plaintext_or_unsupported_session",
            message: "ordercli auth state is not in the supported Hardened State.".into(),
            remediation:
                "Rerun `av harden ordercli`; unsupported fields must be resolved manually.".into(),
            path: Some(path.display().to_string()),
        });
    }
    detection
}

pub(crate) fn secret_gate() -> SecretGateDescriptor {
    SecretGateDescriptor {
        id: "ordercli",
        key_patterns: vec![crate::cli::ordercli_credential::SECRET_NAME.into()],
        routes: vec![SecretGateRoute {
            operation: "ordercli-get",
            script_path: None,
            target_path: target().display().to_string(),
            caller_identifiers: vec!["com.automicvault.av"],
            key_patterns: vec![crate::cli::ordercli_credential::SECRET_NAME.into()],
            replace_existing_env: false,
            allow_missing_keys: false,
        }],
    }
}

fn sanitize_config(contents: &str) -> Result<(String, Option<String>), String> {
    if contents.is_empty() {
        return Ok((String::new(), None));
    }
    let mut value: Value = serde_json::from_str(contents)
        .map_err(|error| format!("invalid ordercli auth session JSON: {error}"))?;
    let root = value
        .as_object_mut()
        .ok_or_else(|| "ordercli config must be a JSON object".to_string())?;
    let foodora = if root.contains_key("providers") {
        let providers = root
            .get_mut("providers")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| "ordercli `providers` must be an object".to_string())?;
        let Some(foodora) = providers.get_mut("foodora") else {
            return Ok((contents.to_string(), None));
        };
        foodora
            .as_object_mut()
            .ok_or_else(|| "ordercli Foodora config must be an object".to_string())?
    } else {
        root
    };
    const ALLOWED: [&str; 15] = [
        "base_url",
        "global_entity_id",
        "target_country_iso",
        "device_id",
        "access_token",
        "refresh_token",
        "expires_at",
        "client_secret",
        "oauth_client_id",
        "http_user_agent",
        "cookies_by_host",
        "pending_mfa_token",
        "pending_mfa_channel",
        "pending_mfa_email",
        "pending_mfa_created_at",
    ];
    if !foodora.keys().all(|key| ALLOWED.contains(&key.as_str())) {
        return Err("ordercli Foodora config contains unsupported fields".into());
    }
    let string_secret = |key: &str| -> Result<&str, String> {
        foodora
            .get(key)
            .map(|value| {
                value
                    .as_str()
                    .ok_or_else(|| format!("ordercli `{key}` must be a string"))
            })
            .transpose()
            .map(|value| value.unwrap_or(""))
    };
    let access = string_secret("access_token")?;
    let refresh = string_secret("refresh_token")?;
    let client = string_secret("client_secret")?;
    let mfa = string_secret("pending_mfa_token")?;
    let cookies = foodora.get("cookies_by_host");
    if cookies.is_some_and(|value| !value.is_object()) {
        return Err("ordercli `cookies_by_host` must be an object".into());
    }
    if cookies.and_then(Value::as_object).is_some_and(|cookies| {
        cookies
            .iter()
            .any(|(host, cookie)| host.is_empty() || !cookie.is_string())
    }) {
        return Err("ordercli cookies contain unsupported fields".into());
    }
    let cookie_marker = cookies.and_then(Value::as_object).is_some_and(|cookies| {
        cookies.len() == 1 && cookies.get(MARKER).and_then(Value::as_str) == Some(MARKER)
    });
    let any_marker = [access, refresh, client, mfa].contains(&MARKER) || cookie_marker;
    if any_marker
        && ([access, refresh, client, mfa]
            .iter()
            .any(|value| *value != MARKER)
            || !cookie_marker)
    {
        return Err("ordercli credential state is only partially migrated".into());
    }
    let has_plaintext = [access, refresh, client, mfa]
        .iter()
        .any(|value| !value.is_empty())
        || cookies
            .and_then(Value::as_object)
            .is_some_and(|cookies| !cookies.is_empty());
    if !has_plaintext {
        return Ok((contents.to_string(), None));
    }
    let credential = if any_marker {
        None
    } else {
        let raw = json!({
            "access_token": access,
            "refresh_token": refresh,
            "client_secret": client,
            "pending_mfa_token": mfa,
            "cookies_by_host": cookies.cloned(),
        })
        .to_string();
        Some(crate::cli::ordercli_credential::parse_credentials(&raw)?)
    };
    for key in [
        "access_token",
        "refresh_token",
        "client_secret",
        "pending_mfa_token",
    ] {
        foodora.insert(key.into(), Value::String(MARKER.into()));
    }
    foodora.insert("cookies_by_host".into(), json!({"@av": "@av"}));
    let mut sanitized = serde_json::to_string_pretty(&value)
        .map_err(|error| format!("failed to encode ordercli auth metadata: {error}"))?;
    sanitized.push('\n');
    Ok((sanitized, credential))
}

fn config_paths() -> Result<Vec<PathBuf>, String> {
    if let Some(path) = test_config_path() {
        return Ok(vec![path]);
    }
    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
    let root = PathBuf::from(home).join("Library/Application Support");
    Ok(["ordercli", "foodcli", "foodoracli"]
        .into_iter()
        .map(|name| root.join(name).join("config.json"))
        .collect())
}

fn test_config_path() -> Option<PathBuf> {
    crate::test_env_var("AUTOMIC_VAULT_TEST_ORDERCLI_CONFIG").map(PathBuf::from)
}

fn target() -> PathBuf {
    super::isotope::target(super::isotope::ORDERCLI)
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
            "refusing unsafe ordercli config {}",
            path.display()
        ));
    }
    let mut contents = String::new();
    file.take(MAX_CONFIG_BYTES + 1)
        .read_to_string(&mut contents)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    if contents.len() as u64 > MAX_CONFIG_BYTES {
        return Err("ordercli auth session exceeds 1 MiB".into());
    }
    Ok(contents)
}

fn write_config(path: &Path, contents: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "ordercli config has no parent".to_string())?;
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
        Ok(_) => Err(format!(
            "refusing unsafe ordercli directory {}",
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
        let linked = Path::new(prefix).join("bin/ordercli");
        let formula = Path::new(prefix).join("opt/ordercli/bin/ordercli");
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
        .ok_or_else(|| "Homebrew ordercli is installed but brew is unavailable".to_string())?;
    let status = Command::new(&brew)
        .args(["unlink", "ordercli"])
        .status()
        .map_err(|error| format!("failed to run {}: {error}", brew.display()))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("failed to unlink Homebrew ordercli: {status}"))
}

fn verify_command_resolution() -> Result<(), String> {
    let output = Command::new("/usr/bin/which")
        .arg("ordercli")
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("failed to resolve ordercli: {error}"))?;
    let resolved = String::from_utf8(output.stdout)
        .ok()
        .filter(|_| output.status.success())
        .map(|path| PathBuf::from(path.trim()))
        .and_then(|path| path.canonicalize().ok());
    if resolved.is_some() && resolved == target().canonicalize().ok() {
        Ok(())
    } else {
        Err(format!(
            "your PATH does not resolve `ordercli` to {}",
            target().display()
        ))
    }
}

pub(crate) fn verify_target(path: &Path) -> Result<(), String> {
    if test_config_path().is_some() {
        return path
            .exists()
            .then_some(())
            .ok_or_else(|| format!("test ordercli Target is missing: {}", path.display()));
    }
    let requirement = format!(
        "=identifier \"ordercli\" and anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] exists and certificate leaf[field.1.2.840.113635.100.6.1.13] exists and certificate leaf[subject.OU] = \"{TEAM_IDENTIFIER}\""
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
            "ordercli Target signature is invalid: {}",
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
            "ordercli Target lacks the required Developer ID Hardened Runtime identity".into(),
        );
    }
    let entitlements = Command::new("/usr/bin/codesign")
        .args(["-d", "--entitlements", ":-"])
        .arg(path)
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .output()
        .map_err(|error| format!("failed to inspect ordercli entitlements: {error}"))?;
    if !entitlements.status.success() || !entitlements.stdout.is_empty() {
        return Err("ordercli Target has unexpected code-signing entitlements".into());
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
    fn migrates_foodora_bundle_and_rejects_partial_or_unknown_state() {
        let input = r#"{"version":1,"providers":{"foodora":{"base_url":"https://example.com","device_id":"device","access_token":"access","refresh_token":"refresh","client_secret":"client","cookies_by_host":{"example.com":"cookie"}}}}"#;
        let (sanitized, credential) = sanitize_config(input).unwrap();
        assert!(credential.unwrap().contains("cookie"));
        assert_eq!(sanitized.matches(MARKER).count(), 6);
        assert!(
            sanitize_config(&input.replace("\"device_id\"", "\"future\":1,\"device_id\"")).is_err()
        );
        assert!(
            sanitize_config(&sanitized.replace(
                "\"refresh_token\": \"@av\"",
                "\"refresh_token\": \"plaintext\""
            ))
            .is_err()
        );
    }
}
