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
    super::PrivilegeMode::Mixed.require_user("uaa-cli", testing)?;
    if !testing {
        crate::secrets::ensure_uaa_helper_ready()?;
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
        return Err("UAA CLI config contains conflicting credential bundles".into());
    }
    let target = target();
    let plan = super::isotope::plan(super::isotope::UAA)?;

    writeln!(stdout, "╭─ harden uaa-cli").ok();
    writeln!(stdout, "│").ok();
    plan.write(stdout, super::isotope::UAA);
    writeln!(
        stdout,
        "├─ migrate UAA OAuth contexts without printing them"
    )
    .ok();
    writeln!(stdout, "├─ keep only UAA metadata and @av markers on disk").ok();
    writeln!(stdout, "│").ok();
    if !confirm(stdout, yes)? {
        writeln!(stdout, "╰─ cancelled").ok();
        return Ok(());
    }
    plan.apply(super::isotope::UAA)?;
    verify_target(&target)?;
    if !testing {
        verify_command_resolution()?;
    }
    if let Some(credential) = credentials.first() {
        crate::secrets::store_secret_if_absent_or_equal(
            crate::cli::uaa_credential::SECRET_NAME,
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
    writeln!(stdout, "╰─ hardened uaa-cli").ok();
    super::write_secret_gate_notice(stdout, "uaa-cli");
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
    let isotope = super::isotope::detect(super::isotope::UAA)
        .commands
        .into_iter()
        .next()
        .and_then(|command| command.isotope);
    let command = HardenerCommand {
        name: "uaa-cli".into(),
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
            kind: "uaa_cli_target_invalid",
            message: verify_target(&target).unwrap_err(),
            remediation: "Rerun `av harden uaa-cli` to install the signed uaa-cli Isotope.".into(),
            path: Some(target.display().to_string()),
        });
    }
    if target_valid && !command_resolves && test_config_path().is_none() {
        detection.diagnostics.push(HardenerDiagnostic {
            kind: "uaa_cli_command_shadowed",
            message: verify_command_resolution().unwrap_err(),
            remediation: "Rerun `av harden uaa-cli` after correcting PATH.".into(),
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
            kind: "uaa_cli_plaintext_or_unsupported_session",
            message: "UAA CLI auth state is not in the supported Hardened State.".into(),
            remediation: "Rerun `av harden uaa-cli`; unsupported fields must be resolved manually."
                .into(),
            path: Some(path.display().to_string()),
        });
    }
    detection
}

pub(crate) fn secret_gate() -> SecretGateDescriptor {
    SecretGateDescriptor {
        id: "uaa-cli",
        key_patterns: vec![crate::cli::uaa_credential::SECRET_NAME.into()],
        routes: vec![SecretGateRoute {
            operation: "uaa-get",
            script_path: None,
            target_path: target().display().to_string(),
            caller_identifiers: vec!["com.automicvault.av"],
            key_patterns: vec![crate::cli::uaa_credential::SECRET_NAME.into()],
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
        .map_err(|error| format!("invalid UAA CLI config JSON: {error}"))?;
    let root = value
        .as_object_mut()
        .ok_or_else(|| "UAA CLI config must be a JSON object".to_string())?;
    if !root.keys().all(|key| {
        matches!(
            key.as_str(),
            "Verbose" | "ZoneSubdomain" | "Targets" | "ActiveTargetName"
        )
    }) {
        return Err("UAA CLI config contains unsupported top-level fields".into());
    }
    for key in ["ZoneSubdomain", "ActiveTargetName"] {
        if root.get(key).is_some_and(|value| !value.is_string()) {
            return Err(format!("UAA CLI `{key}` must be a string"));
        }
    }
    if root.get("Verbose").is_some_and(|value| !value.is_boolean()) {
        return Err("UAA CLI `Verbose` must be a boolean".into());
    }
    let Some(targets) = root.get_mut("Targets") else {
        return Ok((contents.to_string(), None));
    };
    let targets = targets
        .as_object_mut()
        .ok_or_else(|| "UAA CLI `Targets` must be an object".to_string())?;
    if targets.len() > 128 {
        return Err("UAA CLI config has too many targets".into());
    }
    let mut bundle = serde_json::Map::new();
    let mut saw_marker = false;
    let mut saw_plaintext = false;
    for (target_name, target) in targets {
        validate_key("target", target_name)?;
        let target = target
            .as_object_mut()
            .ok_or_else(|| "UAA CLI target must be an object".to_string())?;
        if !target.keys().all(|key| {
            matches!(
                key.as_str(),
                "BaseUrl" | "SkipSSLValidation" | "Contexts" | "ActiveContextName"
            )
        }) {
            return Err("UAA CLI target contains unsupported fields".into());
        }
        for key in ["BaseUrl", "ActiveContextName"] {
            if target.get(key).is_some_and(|value| !value.is_string()) {
                return Err(format!("UAA CLI target `{key}` must be a string"));
            }
        }
        if target
            .get("SkipSSLValidation")
            .is_some_and(|value| !value.is_boolean())
        {
            return Err("UAA CLI `SkipSSLValidation` must be a boolean".into());
        }
        let Some(contexts) = target.get_mut("Contexts") else {
            continue;
        };
        let contexts = contexts
            .as_object_mut()
            .ok_or_else(|| "UAA CLI `Contexts` must be an object".to_string())?;
        if contexts.len() > 256 {
            return Err("UAA CLI target has too many contexts".into());
        }
        let mut target_bundle = serde_json::Map::new();
        for (context_name, context) in contexts {
            validate_key("context", context_name)?;
            let context = context
                .as_object_mut()
                .ok_or_else(|| "UAA CLI context must be an object".to_string())?;
            if !context.keys().all(|key| {
                matches!(
                    key.as_str(),
                    "client_id" | "grant_type" | "username" | "Token"
                )
            }) {
                return Err("UAA CLI context contains unsupported fields".into());
            }
            for key in ["client_id", "grant_type", "username"] {
                if context.get(key).is_some_and(|value| !value.is_string()) {
                    return Err(format!("UAA CLI context `{key}` must be a string"));
                }
            }
            let Some(token) = context.get_mut("Token") else {
                continue;
            };
            let token = token
                .as_object_mut()
                .ok_or_else(|| "UAA CLI `Token` must be an object".to_string())?;
            if !token.keys().all(|key| {
                matches!(
                    key.as_str(),
                    "access_token" | "token_type" | "refresh_token" | "expiry" | "expires_in"
                )
            }) {
                return Err("UAA CLI token contains unsupported fields".into());
            }
            for key in ["access_token", "refresh_token"] {
                if token.get(key).is_some_and(|value| !value.is_string()) {
                    return Err(format!("UAA CLI token `{key}` must be a string"));
                }
            }
            if token
                .get("token_type")
                .is_some_and(|value| !value.is_string())
                || token.get("expiry").is_some_and(|value| !value.is_string())
                || token
                    .get("expires_in")
                    .is_some_and(|value| !value.is_number())
            {
                return Err("UAA CLI token metadata has an unsupported type".into());
            }
            let access = token
                .get("access_token")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let refresh = token
                .get("refresh_token")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let access_marker = access == MARKER;
            let refresh_marker = refresh == MARKER;
            saw_marker |= access_marker || refresh_marker;
            saw_plaintext |=
                !access.is_empty() && !access_marker || !refresh.is_empty() && !refresh_marker;
            if access_marker || refresh_marker {
                continue;
            }
            if access.is_empty() && refresh.is_empty() {
                continue;
            }
            let mut stored = serde_json::Map::new();
            if !access.is_empty() {
                stored.insert("access_token".into(), Value::String(access));
                token.insert("access_token".into(), Value::String(MARKER.into()));
            }
            if !refresh.is_empty() {
                stored.insert("refresh_token".into(), Value::String(refresh));
                token.insert("refresh_token".into(), Value::String(MARKER.into()));
            }
            target_bundle.insert(context_name.clone(), Value::Object(stored));
        }
        if !target_bundle.is_empty() {
            bundle.insert(target_name.clone(), Value::Object(target_bundle));
        }
    }
    if saw_marker && saw_plaintext {
        return Err("UAA CLI credential state is only partially migrated".into());
    }
    if saw_marker {
        return Ok((contents.to_string(), None));
    }
    if bundle.is_empty() {
        return Ok((contents.to_string(), None));
    }
    let raw = json!({"targets": bundle}).to_string();
    let credential = Some(crate::cli::uaa_credential::parse_credentials(&raw)?);
    let mut sanitized = serde_json::to_string_pretty(&value)
        .map_err(|error| format!("failed to encode UAA CLI metadata: {error}"))?;
    sanitized.push('\n');
    Ok((sanitized, credential))
}

fn validate_key(kind: &str, value: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > 4096 || value.bytes().any(|byte| byte == 0) {
        return Err(format!("invalid UAA {kind} credential key"));
    }
    Ok(())
}

fn config_paths() -> Result<Vec<PathBuf>, String> {
    if let Some(path) = test_config_path() {
        return Ok(vec![path]);
    }
    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
    if let Some(root) = std::env::var_os("UAA_HOME") {
        return Ok(vec![PathBuf::from(root).join("config.json")]);
    }
    Ok(vec![PathBuf::from(home).join(".uaa/config.json")])
}

fn test_config_path() -> Option<PathBuf> {
    crate::test_env_var("AUTOMIC_VAULT_TEST_UAA_CLI_CONFIG").map(PathBuf::from)
}

fn target() -> PathBuf {
    super::isotope::target(super::isotope::UAA)
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
        return Err(format!("refusing unsafe uaa-cli config {}", path.display()));
    }
    let mut contents = String::new();
    file.take(MAX_CONFIG_BYTES + 1)
        .read_to_string(&mut contents)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    if contents.len() as u64 > MAX_CONFIG_BYTES {
        return Err("uaa-cli auth session exceeds 1 MiB".into());
    }
    Ok(contents)
}

fn write_config(path: &Path, contents: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "uaa-cli config has no parent".to_string())?;
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
            "refusing unsafe uaa-cli directory {}",
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
        .arg("uaa")
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("failed to resolve UAA CLI: {error}"))?;
    let resolved = String::from_utf8(output.stdout)
        .ok()
        .filter(|_| output.status.success())
        .map(|path| PathBuf::from(path.trim()))
        .and_then(|path| path.canonicalize().ok());
    if resolved.is_some() && resolved == target().canonicalize().ok() {
        Ok(())
    } else {
        Err(format!(
            "your PATH does not resolve `uaa` to {}",
            target().display()
        ))
    }
}

pub(crate) fn verify_target(path: &Path) -> Result<(), String> {
    if test_config_path().is_some() {
        return path
            .exists()
            .then_some(())
            .ok_or_else(|| format!("test uaa-cli Target is missing: {}", path.display()));
    }
    let requirement = format!(
        "=identifier \"uaa\" and anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] exists and certificate leaf[field.1.2.840.113635.100.6.1.13] exists and certificate leaf[subject.OU] = \"{TEAM_IDENTIFIER}\""
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
            "uaa-cli Target signature is invalid: {}",
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
            "uaa-cli Target lacks the required Developer ID Hardened Runtime identity".into(),
        );
    }
    let entitlements = Command::new("/usr/bin/codesign")
        .args(["-d", "--entitlements", ":-"])
        .arg(path)
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .output()
        .map_err(|error| format!("failed to inspect uaa-cli entitlements: {error}"))?;
    if !entitlements.status.success() || !entitlements.stdout.is_empty() {
        return Err("uaa-cli Target has unexpected code-signing entitlements".into());
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
    fn migrates_oauth_contexts_and_rejects_partial_or_unknown_state() {
        let input = r#"{"Verbose":false,"ZoneSubdomain":"","ActiveTargetName":"url:https://uaa.example","Targets":{"url:https://uaa.example":{"BaseUrl":"https://uaa.example","SkipSSLValidation":false,"ActiveContextName":"client:admin user: grant_type:client_credentials","Contexts":{"client:admin user: grant_type:client_credentials":{"client_id":"admin","grant_type":"client_credentials","username":"","Token":{"access_token":"access","token_type":"bearer","refresh_token":"refresh","expiry":"2026-08-24T12:00:00Z"}}}}}}"#;
        let (sanitized, credential) = sanitize_config(input).unwrap();
        let credential = credential.unwrap();
        assert!(credential.contains("access"));
        assert!(credential.contains("refresh"));
        assert_eq!(sanitized.matches(MARKER).count(), 2);
        assert!(
            sanitize_config(&input.replace("\"Verbose\"", "\"Future\":1,\"Verbose\"")).is_err()
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
