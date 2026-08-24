use std::fs::{self, OpenOptions};
use std::io::{self, IsTerminal, Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use toml::Value;

use super::{
    HardenerCommand, HardenerDetection, HardenerDiagnostic, RequiredExecutable,
    SecretGateDescriptor, SecretGateRoute,
};

const AV_PATH: &str = "/usr/local/bin/av";
const CREDENTIAL_MARKER: &str = "@av";
const MAX_CONFIG_BYTES: u64 = 1024 * 1024;
const PRIVILEGE_MODE: super::PrivilegeMode = super::PrivilegeMode::Mixed;
const TEAM_IDENTIFIER: &str = "ZU76A67LGU";

#[derive(Debug, PartialEq, Eq)]
struct Credential {
    profile: String,
    host: String,
    token: String,
}

pub(crate) fn run(stdout: &mut dyn Write, yes: bool) -> Result<(), String> {
    let testing = test_config_path().is_some();
    PRIVILEGE_MODE.require_user("oxide-cli", testing)?;
    if !testing {
        crate::secrets::ensure_oxide_helper_ready()?;
    }
    if std::env::var_os("OXIDE_TOKEN").is_some() {
        return Err("unset OXIDE_TOKEN before hardening Oxide CLI".into());
    }
    let path = config_path()?;
    let original = read_config(&path)?;
    let (sanitized, credentials, managed_secret_names) = sanitize_config(&original)?;
    let existing_secret_names = crate::secrets::list_secret_names()?;
    if let Some(name) = managed_secret_names
        .iter()
        .find(|name| !existing_secret_names.contains(name))
    {
        return Err(format!(
            "Oxide credential marker has no matching Secret Value: {name}"
        ));
    }
    let target = target();
    let plan = super::isotope::plan(super::isotope::OXIDE)?;
    let brew_conflict = !testing && homebrew_formula_installed();

    writeln!(stdout, "╭─ harden oxide-cli").ok();
    writeln!(stdout, "│").ok();
    plan.write(stdout, super::isotope::OXIDE);
    if brew_conflict {
        writeln!(stdout, "├─ unlink the Homebrew oxide-cli formula").ok();
    }
    writeln!(
        stdout,
        "├─ migrate {} profile token{} without printing them",
        credentials.len(),
        if credentials.len() == 1 { "" } else { "s" }
    )
    .ok();
    writeln!(stdout, "├─ keep only Oxide profile metadata on disk").ok();
    writeln!(stdout, "│").ok();
    if !confirm(stdout, yes)? {
        writeln!(stdout, "╰─ cancelled").ok();
        return Ok(());
    }

    plan.apply(super::isotope::OXIDE)?;
    verify_target(&target)?;
    if brew_conflict {
        unlink_homebrew()?;
    }
    if !testing {
        verify_command_resolution()?;
    }
    for credential in &credentials {
        crate::secrets::store_secret_if_absent_or_equal(
            &crate::cli::oxide_credential::secret_name(&credential.profile, &credential.host),
            &credential.token,
        )?;
    }
    if original != sanitized || !path.exists() && !sanitized.is_empty() {
        write_config(&path, &sanitized)?;
    }
    writeln!(stdout, "╰─ hardened oxide-cli").ok();
    super::write_secret_gate_notice(stdout, "oxide-cli");
    Ok(())
}

pub(crate) fn detect() -> HardenerDetection {
    let target = target();
    let target_valid = verify_target(&target).is_ok();
    let command_resolves = test_config_path().is_some() || verify_command_resolution().is_ok();
    let config = config_path().ok();
    let config_valid = config.as_deref().is_some_and(|path| {
        read_config(path).is_ok_and(|contents| {
            sanitize_config(&contents).is_ok_and(|(sanitized, credentials, managed)| {
                credentials.is_empty()
                    && sanitized == contents
                    && crate::secrets::list_secret_names()
                        .is_ok_and(|names| managed.iter().all(|name| names.contains(name)))
            })
        })
    });
    let hardened = target_valid && command_resolves && config_valid;
    let isotope = super::isotope::detect(super::isotope::OXIDE)
        .commands
        .into_iter()
        .next()
        .and_then(|command| command.isotope);
    let command = HardenerCommand {
        name: "oxide".into(),
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
            kind: "oxide_target_invalid",
            message: verify_target(&target).unwrap_err(),
            remediation: "Rerun `av harden oxide-cli` to install the signed Oxide Isotope.".into(),
            path: Some(target.display().to_string()),
        });
    }
    if target_valid && !command_resolves && test_config_path().is_none() {
        detection.diagnostics.push(HardenerDiagnostic {
            kind: "oxide_command_shadowed",
            message: verify_command_resolution().unwrap_err(),
            remediation: "Rerun `av harden oxide-cli` after correcting PATH.".into(),
            path: Some(target.display().to_string()),
        });
    }
    if let Some(path) = config
        && path.exists()
        && !config_valid
    {
        detection.diagnostics.push(HardenerDiagnostic {
            kind: "oxide_plaintext_or_unsupported_config",
            message: "Oxide credential configuration is not in the supported Hardened State."
                .into(),
            remediation:
                "Rerun `av harden oxide-cli`; unsupported fields must be resolved manually.".into(),
            path: Some(path.display().to_string()),
        });
    }
    detection
}

pub(crate) fn secret_gate() -> SecretGateDescriptor {
    SecretGateDescriptor {
        id: "oxide-cli",
        key_patterns: vec!["OXIDE_PROFILE_TOKEN_*".into()],
        routes: vec![SecretGateRoute {
            operation: "oxide-get",
            script_path: None,
            target_path: target().display().to_string(),
            caller_identifiers: vec!["com.automicvault.av"],
            key_patterns: vec!["OXIDE_PROFILE_TOKEN_*".into()],
            replace_existing_env: false,
            allow_missing_keys: false,
        }],
    }
}

fn sanitize_config(contents: &str) -> Result<(String, Vec<Credential>, Vec<String>), String> {
    if contents.is_empty() {
        return Ok((String::new(), Vec::new(), Vec::new()));
    }
    let mut document = toml::from_str::<Value>(contents)
        .map_err(|error| format!("invalid Oxide credentials TOML: {error}"))?;
    let root = document
        .as_table_mut()
        .ok_or_else(|| "Oxide credentials must be a TOML table".to_string())?;
    if root.keys().any(|key| key != "profile") {
        return Err("Oxide credentials contain unsupported top-level fields".into());
    }
    let profiles = root
        .get_mut("profile")
        .and_then(Value::as_table_mut)
        .ok_or_else(|| "Oxide credentials must contain a `profile` table".to_string())?;
    let mut credentials = Vec::new();
    let mut managed_secret_names = Vec::new();
    for (profile, value) in profiles {
        let profile = crate::cli::oxide_credential::normalize_profile(profile)?;
        let table = value
            .as_table_mut()
            .ok_or_else(|| format!("Oxide profile {profile:?} must be a table"))?;
        let allowed = ["host", "token", "token_id", "user", "time_expires"];
        if table.keys().any(|key| !allowed.contains(&key.as_str())) {
            return Err(format!(
                "Oxide profile {profile:?} contains unsupported fields"
            ));
        }
        for required in ["host", "token", "user"] {
            if !table.get(required).is_some_and(Value::is_str) {
                return Err(format!(
                    "Oxide profile {profile:?} requires string field {required:?}"
                ));
            }
        }
        for optional in ["token_id", "time_expires"] {
            if table.get(optional).is_some_and(|value| !value.is_str()) {
                return Err(format!(
                    "Oxide profile {profile:?} field {optional:?} must be a string"
                ));
            }
        }
        let host = crate::cli::oxide_credential::normalize_host(
            table["host"].as_str().expect("validated host"),
        )?;
        let token = table["token"].as_str().expect("validated token");
        if token == CREDENTIAL_MARKER {
            managed_secret_names.push(crate::cli::oxide_credential::secret_name(&profile, &host));
        } else {
            credentials.push(Credential {
                profile: profile.clone(),
                host: host.clone(),
                token: crate::cli::oxide_credential::parse_token(token)?,
            });
        }
        table.insert("host".into(), Value::String(host));
        table.insert("token".into(), Value::String(CREDENTIAL_MARKER.into()));
    }
    let sanitized = toml::to_string_pretty(&document)
        .map_err(|error| format!("failed to serialize Oxide credentials: {error}"))?;
    Ok((sanitized, credentials, managed_secret_names))
}

fn config_path() -> Result<PathBuf, String> {
    if let Some(path) = test_config_path() {
        return Ok(path);
    }
    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
    Ok(PathBuf::from(home).join(".config/oxide/credentials.toml"))
}

fn test_config_path() -> Option<PathBuf> {
    crate::test_env_var("AUTOMIC_VAULT_TEST_OXIDE_CONFIG").map(PathBuf::from)
}

fn target() -> PathBuf {
    super::isotope::target(super::isotope::OXIDE)
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
        return Err(format!("refusing unsafe Oxide config {}", path.display()));
    }
    let mut contents = String::new();
    file.take(MAX_CONFIG_BYTES + 1)
        .read_to_string(&mut contents)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    if contents.len() as u64 > MAX_CONFIG_BYTES {
        return Err("Oxide credentials exceed 1 MiB".into());
    }
    Ok(contents)
}

fn write_config(path: &Path, contents: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("Oxide config has no parent: {}", path.display()))?;
    secure_directory(parent)?;
    let staging = parent.join(format!(
        ".credentials.toml.av-{}.tmp",
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
            "refusing unsafe Oxide directory {}",
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
        let linked = Path::new(prefix).join("bin/oxide");
        let formula = Path::new(prefix).join("opt/oxide-cli/bin/oxide");
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
        .ok_or_else(|| "Homebrew oxide-cli is installed but brew is unavailable".to_string())?;
    let status = Command::new(&brew)
        .args(["unlink", "oxide-cli"])
        .status()
        .map_err(|error| format!("failed to run {}: {error}", brew.display()))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("failed to unlink Homebrew oxide-cli: {status}"))
}

fn verify_command_resolution() -> Result<(), String> {
    let output = Command::new("/usr/bin/which")
        .arg("oxide")
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("failed to resolve oxide: {error}"))?;
    let resolved = String::from_utf8(output.stdout)
        .ok()
        .filter(|_| output.status.success())
        .map(|path| PathBuf::from(path.trim()))
        .and_then(|path| path.canonicalize().ok());
    let expected = target().canonicalize().ok();
    if resolved.is_some() && resolved == expected {
        Ok(())
    } else {
        Err(format!(
            "your PATH does not resolve `oxide` to {}; remove version-manager shims or adjust PATH",
            target().display()
        ))
    }
}

pub(crate) fn verify_target(path: &Path) -> Result<(), String> {
    if test_config_path().is_some() {
        return path
            .exists()
            .then_some(())
            .ok_or_else(|| format!("test Oxide Target is missing: {}", path.display()));
    }
    let requirement = format!(
        "=identifier \"oxide\" and anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] exists and certificate leaf[field.1.2.840.113635.100.6.1.13] exists and certificate leaf[subject.OU] = \"{TEAM_IDENTIFIER}\""
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
            "Oxide Target signature is invalid: {}",
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
            "Oxide Target lacks the required Developer ID Hardened Runtime identity".into(),
        );
    }
    let entitlements = Command::new("/usr/bin/codesign")
        .args(["-d", "--entitlements", ":-"])
        .arg(path)
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .output()
        .map_err(|error| format!("failed to inspect {} entitlements: {error}", path.display()))?;
    if !entitlements.status.success() || !entitlements.stdout.is_empty() {
        return Err("Oxide Target has unexpected code-signing entitlements".into());
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
    fn migration_preserves_metadata_and_rejects_unknown_fields() {
        let input = r#"[profile.prod]
host = "https://OXIDE.example/"
token = "secret"
token_id = "id"
user = "user"
time_expires = "tomorrow"
"#;
        let (sanitized, credentials, managed) = sanitize_config(input).unwrap();
        assert_eq!(
            credentials,
            [Credential {
                profile: "prod".into(),
                host: "https://oxide.example".into(),
                token: "secret".into(),
            }]
        );
        assert!(sanitized.contains("token = \"@av\""));
        assert!(sanitized.contains("token_id = \"id\""));
        assert!(managed.is_empty());
        assert_eq!(
            sanitize_config(&input.replace("secret", "@av")).unwrap().2,
            [crate::cli::oxide_credential::secret_name(
                "prod",
                "https://oxide.example"
            )]
        );
        assert!(sanitize_config(&input.replace("user =", "future = \"x\"\nuser =")).is_err());
    }

    #[test]
    fn hardener_migrates_without_recreating_plaintext() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let root = std::env::temp_dir().join(format!("av-oxide-hardener-{}", std::process::id()));
        let config = root.join("credentials.toml");
        let target = root.join("oxide");
        let keychain = root.join("keychain");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            &config,
            "[profile.prod]\nhost = \"https://oxide.example\"\ntoken = \"secret\"\nuser = \"user\"\n",
        )
        .unwrap();
        fs::write(&target, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).unwrap();
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_OXIDE_CONFIG", &config);
            std::env::set_var("AUTOMIC_VAULT_TEST_OXIDE_TARGET", &target);
            std::env::set_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR", &keychain);
        }
        run(&mut Vec::new(), true).unwrap();
        let hardened = fs::read_to_string(&config).unwrap();
        assert!(hardened.contains("token = \"@av\""));
        assert!(!hardened.contains("secret"));
        assert_eq!(
            fs::read_to_string(keychain.join(crate::cli::oxide_credential::secret_name(
                "prod",
                "https://oxide.example"
            )))
            .unwrap(),
            "secret"
        );
        assert!(detect().hardened);
        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_OXIDE_CONFIG");
            std::env::remove_var("AUTOMIC_VAULT_TEST_OXIDE_TARGET");
            std::env::remove_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR");
        }
        let _ = fs::remove_dir_all(root);
    }
}
