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
    environment: String,
    host: String,
    value: String,
}

struct ConfigState {
    path: PathBuf,
    existed: bool,
    original: String,
    sanitized: String,
    credential: Option<Credential>,
}

pub(crate) fn run(stdout: &mut dyn Write, yes: bool) -> Result<(), String> {
    let testing = test_config_path().is_some();
    super::PrivilegeMode::Mixed.require_user("railway", testing)?;
    if !testing {
        crate::secrets::ensure_railway_helper_ready()?;
    }
    let configs = config_paths()?
        .into_iter()
        .map(|(environment, host, path)| {
            let existed = path.exists();
            let original = read_config(&path)?;
            let (sanitized, credential) = sanitize_config(&original, environment, host)?;
            Ok(ConfigState {
                path,
                existed,
                original,
                sanitized,
                credential,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let target = target();
    let plan = super::isotope::plan(super::isotope::RAILWAY)?;

    writeln!(stdout, "╭─ harden railway").ok();
    writeln!(stdout, "│").ok();
    plan.write(stdout, super::isotope::RAILWAY);
    writeln!(stdout, "├─ migrate Railway auth state without printing it").ok();
    writeln!(stdout, "├─ keep only user metadata and @av markers on disk").ok();
    writeln!(stdout, "│").ok();
    if !confirm(stdout, yes)? {
        writeln!(stdout, "╰─ cancelled").ok();
        return Ok(());
    }
    let rewrites = configs
        .iter()
        .filter(|config| {
            config.original != config.sanitized || !config.existed && !config.sanitized.is_empty()
        })
        .map(|config| super::ConfigRewrite {
            path: &config.path,
            existed: config.existed,
            original: &config.original,
            replacement: &config.sanitized,
        })
        .collect::<Vec<_>>();
    for rewrite in &rewrites {
        secure_directory(
            rewrite
                .path
                .parent()
                .ok_or_else(|| "railway config has no parent".to_string())?,
        )?;
    }
    for credential in configs
        .iter()
        .filter_map(|config| config.credential.as_ref())
    {
        crate::secrets::store_secret_if_absent_or_equal(
            &crate::cli::railway_credential::secret_name(&credential.environment, &credential.host),
            &credential.value,
        )?;
    }
    super::rewrite_configs_with_rollback(&rewrites, write_config, remove_config)?;
    if let Err(error) = plan.apply(super::isotope::RAILWAY) {
        let references = rewrites.iter().collect::<Vec<_>>();
        return match super::restore_config_rewrites(&references, write_config, remove_config) {
            Ok(()) => Err(format!(
                "Railway Target installation failed and configs were restored: {error}"
            )),
            Err(rollback) => Err(format!(
                "Railway Target installation failed ({error}); config restoration also failed: {rollback}"
            )),
        };
    }
    verify_target(&target)?;
    if !testing {
        verify_command_resolution()?;
    }
    writeln!(stdout, "╰─ hardened railway").ok();
    super::write_secret_gate_notice(stdout, "railway");
    Ok(())
}

pub(crate) fn detect() -> HardenerDetection {
    let target = target();
    let target_valid = verify_target(&target).is_ok();
    let command_resolves = test_config_path().is_some() || verify_command_resolution().is_ok();
    let configs = config_paths().unwrap_or_default();
    let config_valid = !configs.is_empty()
        && configs.iter().all(|(environment, host, path)| {
            read_config(path).is_ok_and(|contents| {
                sanitize_config(&contents, environment, host).is_ok_and(
                    |(sanitized, credential)| credential.is_none() && sanitized == contents,
                )
            })
        });
    let hardened = target_valid && command_resolves && config_valid;
    let isotope = super::isotope::detect(super::isotope::RAILWAY)
        .commands
        .into_iter()
        .next()
        .and_then(|command| command.isotope);
    let command = HardenerCommand {
        name: "railway".into(),
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
    detection.applicable = configs.iter().any(|(_, _, path)| path.exists()) || target.exists();
    if target.exists() && !target_valid && test_config_path().is_none() {
        detection.diagnostics.push(HardenerDiagnostic {
            kind: "railway_target_invalid",
            message: verify_target(&target).unwrap_err(),
            remediation: "Rerun `av harden railway` to install the signed railway Isotope.".into(),
            path: Some(target.display().to_string()),
        });
    }
    if target_valid && !command_resolves && test_config_path().is_none() {
        detection.diagnostics.push(HardenerDiagnostic {
            kind: "railway_command_shadowed",
            message: verify_command_resolution().unwrap_err(),
            remediation: "Rerun `av harden railway` after correcting PATH.".into(),
            path: Some(target.display().to_string()),
        });
    }
    if let Some((_, _, path)) = configs.iter().find(|(environment, host, path)| {
        path.exists()
            && match read_config(path) {
                Ok(contents) => match sanitize_config(&contents, environment, host) {
                    Ok((sanitized, credential)) => credential.is_some() || sanitized != contents,
                    Err(_) => true,
                },
                Err(_) => true,
            }
    }) {
        detection.diagnostics.push(HardenerDiagnostic {
            kind: "railway_plaintext_or_unsupported_session",
            message: "railway auth state is not in the supported Hardened State.".into(),
            remediation: "Rerun `av harden railway`; unsupported fields must be resolved manually."
                .into(),
            path: Some(path.display().to_string()),
        });
    }
    detection
}

pub(crate) fn secret_gate() -> SecretGateDescriptor {
    SecretGateDescriptor {
        id: "railway",
        key_patterns: vec!["RAILWAY_AUTH_*".into()],
        routes: vec![SecretGateRoute {
            operation: "railway-get",
            script_path: None,
            target_path: target().display().to_string(),
            caller_identifiers: vec!["com.automicvault.av"],
            key_patterns: vec!["RAILWAY_AUTH_*".into()],
            replace_existing_env: false,
            allow_missing_keys: false,
        }],
    }
}

fn sanitize_config(
    contents: &str,
    environment: &str,
    host: &str,
) -> Result<(String, Option<Credential>), String> {
    crate::cli::railway_credential::validate_scope(environment, host)?;
    if contents.is_empty() {
        return Ok((String::new(), None));
    }
    let mut value: Value = serde_json::from_str(contents)
        .map_err(|error| format!("invalid Railway config JSON: {error}"))?;
    let root = value
        .as_object_mut()
        .ok_or_else(|| "Railway config must be a JSON object".to_string())?;
    let user = root
        .entry("user")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| "Railway `user` config must be an object".to_string())?;
    if !user.keys().all(|key| {
        matches!(
            key.as_str(),
            "id" | "token" | "accessToken" | "refreshToken" | "tokenExpiresAt"
        )
    }) || user
        .get("id")
        .is_some_and(|value| !value.is_null() && !value.is_string())
        || user
            .get("tokenExpiresAt")
            .is_some_and(|value| !value.is_null() && !value.is_i64())
        || ["token", "accessToken", "refreshToken"].iter().any(|key| {
            user.get(*key)
                .is_some_and(|value| !value.is_null() && !value.is_string())
        })
    {
        return Err("Railway `user` config contains unsupported fields or types".into());
    }
    let secret_fields = ["token", "accessToken", "refreshToken"];
    let present = secret_fields
        .iter()
        .filter(|key| user.get(**key).is_some_and(Value::is_string))
        .copied()
        .collect::<Vec<_>>();
    let marker_count = present
        .iter()
        .filter(|key| user[**key].as_str() == Some(MARKER))
        .count();
    if marker_count != 0 && marker_count != present.len() {
        return Err("Railway auth state is only partially migrated".into());
    }
    let raw = json!({
        "token": user.get("token").and_then(Value::as_str),
        "accessToken": user.get("accessToken").and_then(Value::as_str),
        "refreshToken": user.get("refreshToken").and_then(Value::as_str),
    })
    .to_string();
    let canonical = (!present.is_empty())
        .then(|| crate::cli::railway_credential::parse_credentials(&raw))
        .transpose()?;
    let credential = if present.is_empty() || marker_count == present.len() {
        None
    } else {
        Some(Credential {
            environment: environment.into(),
            host: host.into(),
            value: canonical.unwrap(),
        })
    };
    for key in secret_fields {
        if present.contains(&key) {
            user.insert(key.into(), Value::String(MARKER.into()));
        } else {
            user.remove(key);
        }
    }
    let sanitized = serde_json::to_string_pretty(&value)
        .map_err(|error| format!("failed to encode Railway auth metadata: {error}"))?;
    Ok((sanitized, credential))
}

fn config_paths() -> Result<Vec<(&'static str, &'static str, PathBuf)>, String> {
    if let Some(path) = test_config_path() {
        return Ok(vec![("production", "railway.com", path)]);
    }
    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
    let root = PathBuf::from(home).join(".railway");
    Ok(vec![
        ("production", "railway.com", root.join("config.json")),
        (
            "staging",
            "railway-staging.com",
            root.join("config-staging.json"),
        ),
        ("dev", "railway-develop.com", root.join("config-dev.json")),
    ])
}

fn test_config_path() -> Option<PathBuf> {
    crate::test_env_var("AUTOMIC_VAULT_TEST_RAILWAY_CONFIG").map(PathBuf::from)
}

fn target() -> PathBuf {
    super::isotope::target(super::isotope::RAILWAY)
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
        return Err(format!("refusing unsafe railway config {}", path.display()));
    }
    let mut contents = String::new();
    file.take(MAX_CONFIG_BYTES + 1)
        .read_to_string(&mut contents)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    if contents.len() as u64 > MAX_CONFIG_BYTES {
        return Err("railway auth session exceeds 1 MiB".into());
    }
    Ok(contents)
}

fn write_config(path: &Path, contents: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "railway config has no parent".to_string())?;
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

fn remove_config(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("failed to remove {}: {error}", path.display())),
    }
    let parent = path
        .parent()
        .ok_or_else(|| "railway config has no parent".to_string())?;
    OpenOptions::new()
        .read(true)
        .open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("failed to sync {}: {error}", parent.display()))
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
            "refusing unsafe railway directory {}",
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
        .arg("railway")
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("failed to resolve railway: {error}"))?;
    let resolved = String::from_utf8(output.stdout)
        .ok()
        .filter(|_| output.status.success())
        .map(|path| PathBuf::from(path.trim()))
        .and_then(|path| path.canonicalize().ok());
    if resolved.is_some() && resolved == target().canonicalize().ok() {
        Ok(())
    } else {
        Err(format!(
            "your PATH does not resolve `railway` to {}",
            target().display()
        ))
    }
}

pub(crate) fn verify_target(path: &Path) -> Result<(), String> {
    if test_config_path().is_some() {
        return path
            .exists()
            .then_some(())
            .ok_or_else(|| format!("test railway Target is missing: {}", path.display()));
    }
    let requirement = format!(
        "=identifier \"railway\" and anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] exists and certificate leaf[field.1.2.840.113635.100.6.1.13] exists and certificate leaf[subject.OU] = \"{TEAM_IDENTIFIER}\""
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
            "railway Target signature is invalid: {}",
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
            "railway Target lacks the required Developer ID Hardened Runtime identity".into(),
        );
    }
    let entitlements = Command::new("/usr/bin/codesign")
        .args(["-d", "--entitlements", ":-"])
        .arg(path)
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .output()
        .map_err(|error| format!("failed to inspect railway entitlements: {error}"))?;
    if !entitlements.status.success() || !entitlements.stdout.is_empty() {
        return Err("railway Target has unexpected code-signing entitlements".into());
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
    fn migrates_oauth_session_and_rejects_unsupported_user_state() {
        let input = r#"{"projects":{},"user":{"id":"u","accessToken":"access","refreshToken":"refresh","tokenExpiresAt":123}}"#;
        let (sanitized, credential) = sanitize_config(input, "production", "railway.com").unwrap();
        let credential = credential.unwrap();
        assert_eq!(credential.environment, "production");
        assert_eq!(credential.host, "railway.com");
        assert!(!credential.value.contains("@av"));
        assert_eq!(sanitized.matches("@av").count(), 2);
        assert!(
            sanitize_config(
                &input.replace("\"id\"", "\"future\":1,\"id\""),
                "production",
                "railway.com"
            )
            .is_err()
        );
        assert!(
            sanitize_config(
                &input.replace("\"access\"", "\"@av\""),
                "production",
                "railway.com"
            )
            .is_err()
        );
        assert!(
            sanitize_config(
                r#"{"user":{"refreshToken":"@av"}}"#,
                "production",
                "railway.com"
            )
            .is_err()
        );
    }
}
