use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{self, IsTerminal, Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use super::{
    HardenerCommand, HardenerDetection, HardenerDiagnostic, RequiredIdentity, SecretGateDescriptor,
    SecretGateRoute, StubRequirements,
};

const TERRAFORM_TARGET: &str = "/opt/av/terraform/current/terraform";
const OPENTOFU_TARGET: &str = "/opt/av/opentofu/current/tofu";
const MAX_CONFIG: u64 = 1024 * 1024;
const PRIVILEGE_MODE: super::PrivilegeMode = super::PrivilegeMode::UserOnly;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tool {
    Terraform,
    OpenTofu,
}

impl Tool {
    fn hardener(self) -> &'static str {
        match self {
            Self::Terraform => "terraform",
            Self::OpenTofu => "opentofu",
        }
    }

    fn command(self) -> &'static str {
        match self {
            Self::Terraform => "terraform",
            Self::OpenTofu => "tofu",
        }
    }

    fn target(self) -> PathBuf {
        let test = match self {
            Self::Terraform => "AUTOMIC_VAULT_TEST_TERRAFORM_TARGET",
            Self::OpenTofu => "AUTOMIC_VAULT_TEST_OPENTOFU_TARGET",
        };
        crate::test_env_var(test)
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(match self {
                    Self::Terraform => TERRAFORM_TARGET,
                    Self::OpenTofu => OPENTOFU_TARGET,
                })
            })
    }
}

pub(crate) fn run(tool: Tool, stdout: &mut dyn Write, yes: bool) -> Result<(), String> {
    let testing = test_config_path().is_some();
    PRIVILEGE_MODE.require_user(tool.hardener(), testing)?;
    if !testing {
        crate::secrets::ensure_terraform_helper_ready()?;
        verify_target(tool, &tool.target())?;
    }
    refuse_competing_config()?;
    let path = config_path()?;
    let mut config = read_config(&path)?;
    let credentials = validate_config(&config)?;

    writeln!(stdout, "╭─ harden {}", tool.hardener()).ok();
    writeln!(stdout, "│").ok();
    writeln!(stdout, "├─ keep the verified {} Target", tool.command()).ok();
    writeln!(stdout, "├─ install {}", helper_path().display()).ok();
    writeln!(
        stdout,
        "├─ migrate {} host credential{} without printing them",
        credentials.len(),
        if credentials.len() == 1 { "" } else { "s" }
    )
    .ok();
    writeln!(stdout, "├─ configure the Automic Vault credential helper").ok();
    writeln!(stdout, "│").ok();
    if !confirm(stdout, yes)? {
        writeln!(stdout, "╰─ cancelled").ok();
        return Ok(());
    }

    install_helper()?;
    for (hostname, token) in &credentials {
        crate::secrets::store_secret_if_absent_or_equal(
            &crate::cli::terraform_credential::secret_name(hostname),
            &json!({"token": token}).to_string(),
        )?;
    }
    let object = config.as_object_mut().expect("validated config");
    object.remove("credentials");
    object.insert("credentials_helper".into(), json!({"av": {"args": []}}));
    write_config(&path, &config)?;
    writeln!(stdout, "╰─ hardened {}", tool.hardener()).ok();
    super::write_secret_gate_notice(stdout, tool.hardener());
    Ok(())
}

pub(crate) fn detect(tool: Tool) -> HardenerDetection {
    let helper = helper_path();
    let config = config_path().ok();
    let config_valid = config
        .as_deref()
        .and_then(|path| read_config(path).ok())
        .is_some_and(|value| {
            validate_config(&value).is_ok_and(|credentials| credentials.is_empty())
                && helper_configured(&value)
        });
    let target = tool.target();
    let target_valid =
        test_config_path().is_some() && target.exists() || verify_target(tool, &target).is_ok();
    let stub_valid = helper_valid(&helper);
    let hardened = config_valid && target_valid && stub_valid;
    let command = HardenerCommand {
        name: tool.command().into(),
        hardened,
        stub_valid,
        stub_path: Some(helper.display().to_string()),
        target_path: target.display().to_string(),
        required_paths: Vec::new(),
        stub_requirements: Some(stub_requirements(&helper)),
        injected_keys: Vec::new(),
        assignment_keys: Vec::new(),
        isotope: None,
    };
    let mut detection = HardenerDetection::commands(hardened, vec![command]);
    detection.applicable = config.as_deref().is_some_and(Path::exists) || target.exists();
    if target.exists() && !target_valid && test_config_path().is_none() {
        detection.diagnostics.push(HardenerDiagnostic {
            kind: "terraform_target_invalid",
            message: verify_target(tool, &target).unwrap_err(),
            remediation: format!(
                "Rerun `av harden {}` to install a verified Target.",
                tool.hardener()
            ),
            path: Some(target.display().to_string()),
        });
    }
    detection
}

pub(crate) fn secret_gate(tool: Tool) -> SecretGateDescriptor {
    SecretGateDescriptor {
        id: tool.hardener(),
        key_patterns: vec!["TERRAFORM_HOST_CREDENTIAL_*".into()],
        routes: vec![SecretGateRoute {
            operation: "terraform-get",
            script_path: None,
            target_path: tool.target().display().to_string(),
            caller_identifiers: vec!["com.automicvault.av"],
            key_patterns: vec!["TERRAFORM_HOST_CREDENTIAL_*".into()],
            replace_existing_env: false,
            allow_missing_keys: false,
        }],
    }
}

fn validate_config(config: &Value) -> Result<BTreeMap<String, String>, String> {
    let object = config
        .as_object()
        .ok_or_else(|| "Terraform CLI config must be a JSON object".to_string())?;
    if object
        .get("credentials_helper")
        .is_some_and(|value| value != &json!({"av": {"args": []}}))
    {
        return Err("a non-Automic Terraform credentials helper is already configured".into());
    }
    let mut credentials = BTreeMap::new();
    let Some(hosts) = object.get("credentials") else {
        return Ok(credentials);
    };
    let hosts = hosts
        .as_object()
        .ok_or_else(|| "Terraform `credentials` must be an object".to_string())?;
    for (hostname, credential) in hosts {
        let normalized = crate::cli::terraform_credential::normalize_hostname(hostname)?;
        if &normalized != hostname {
            return Err(format!(
                "Terraform credential hostname is not canonical: {hostname}"
            ));
        }
        let token = crate::cli::terraform_credential::parse_token(&credential.to_string())?;
        if credentials.insert(normalized, token).is_some() {
            return Err("duplicate Terraform credential hostname".into());
        }
    }
    Ok(credentials)
}

fn helper_configured(config: &Value) -> bool {
    config.get("credentials_helper") == Some(&json!({"av": {"args": []}}))
        && config.get("credentials").is_none()
}

fn config_path() -> Result<PathBuf, String> {
    if let Some(path) = test_config_path() {
        return Ok(path);
    }
    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
    Ok(PathBuf::from(home).join(".terraform.d/credentials.tfrc.json"))
}

fn test_config_path() -> Option<PathBuf> {
    crate::test_env_var("AUTOMIC_VAULT_TEST_TERRAFORM_CONFIG").map(PathBuf::from)
}

fn refuse_competing_config() -> Result<(), String> {
    if test_config_path().is_some() {
        return Ok(());
    }
    if std::env::var_os("TF_CLI_CONFIG_FILE").is_some()
        || std::env::var_os("TERRAFORM_CONFIG").is_some()
        || std::env::vars_os().any(|(name, _)| name.to_string_lossy().starts_with("TF_TOKEN_"))
    {
        return Err(
            "unset TF_CLI_CONFIG_FILE, TERRAFORM_CONFIG, and TF_TOKEN_* before hardening".into(),
        );
    }
    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
    let path = PathBuf::from(home).join(".terraformrc");
    match fs::read_to_string(&path) {
        Ok(contents) if contents.trim().is_empty() => Ok(()),
        Ok(_) => Err(format!(
            "move non-secret settings out of {} and remove it before hardening; Automic Vault will not guess whether HCL contains competing credentials",
            path.display()
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("failed to inspect {}: {error}", path.display())),
    }
}

fn read_config(path: &Path) -> Result<Value, String> {
    let file = match OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(json!({})),
        Err(error) => return Err(format!("failed to open {}: {error}", path.display())),
    };
    let metadata = file
        .metadata()
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
    if !metadata.is_file() || metadata.len() > MAX_CONFIG {
        return Err(format!(
            "refusing unsafe Terraform config: {}",
            path.display()
        ));
    }
    let mut contents = String::new();
    file.take(MAX_CONFIG + 1)
        .read_to_string(&mut contents)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    if contents.len() as u64 > MAX_CONFIG {
        return Err("Terraform config exceeds 1 MiB".into());
    }
    serde_json::from_str(&contents)
        .map_err(|error| format!("invalid Terraform config {}: {error}", path.display()))
}

fn write_config(path: &Path, value: &Value) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("Terraform config has no parent: {}", path.display()))?;
    secure_directory(parent, 0o700)?;
    let staging = parent.join(format!(
        ".credentials.tfrc.json.av.{}.{}",
        std::process::id(),
        now_nanos()
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&staging)
            .map_err(|error| format!("failed to create {}: {error}", staging.display()))?;
        serde_json::to_writer_pretty(&mut file, value)
            .map_err(|error| format!("failed to encode Terraform config: {error}"))?;
        file.write_all(b"\n")
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("failed to write {}: {error}", staging.display()))?;
        fs::rename(&staging, path)
            .map_err(|error| format!("failed to replace {}: {error}", path.display()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(staging);
    }
    result
}

fn install_helper() -> Result<(), String> {
    let path = helper_path();
    let parent = path
        .parent()
        .ok_or_else(|| format!("Terraform helper has no parent: {}", path.display()))?;
    secure_directory(parent, 0o755)?;
    let staging = parent.join(format!(".terraform-credentials-av.{}.tmp", now_nanos()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o700)
            .open(&staging)
            .map_err(|error| format!("failed to create {}: {error}", staging.display()))?;
        file.write_all(crate::cli::terraform_credential::helper_stub().as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("failed to write {}: {error}", staging.display()))?;
        file.set_permissions(fs::Permissions::from_mode(0o755))
            .map_err(|error| format!("failed to chmod {}: {error}", staging.display()))?;
        fs::rename(&staging, path)
            .map_err(|error| format!("failed to install Terraform helper: {error}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(staging);
    }
    result
}

fn secure_directory(path: &Path, mode: u32) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        secure_directory(parent, 0o700)?;
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
            "refusing unsafe Terraform directory {}",
            path.display()
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::create_dir(path)
                .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
            fs::set_permissions(path, fs::Permissions::from_mode(mode))
                .map_err(|error| format!("failed to protect {}: {error}", path.display()))
        }
        Err(error) => Err(format!("failed to inspect {}: {error}", path.display())),
    }
}

fn helper_path() -> PathBuf {
    crate::cli::terraform_credential::helper_path()
}

fn helper_valid(path: &Path) -> bool {
    let metadata = fs::symlink_metadata(path).ok();
    crate::cli::terraform_credential::helper_stub_valid(path)
        && metadata.is_some_and(|metadata| {
            metadata.uid() == super::effective_uid()
                && metadata.permissions().mode() & 0o777 == 0o755
        })
}

fn stub_requirements(path: &Path) -> StubRequirements {
    let ids = path
        .parent()
        .and_then(|parent| parent.metadata().ok())
        .map(|metadata| (metadata.uid(), metadata.gid()));
    StubRequirements {
        mode: 0o755,
        owner: RequiredIdentity {
            name: "current user",
            id: Some(ids.map_or(super::effective_uid(), |ids| ids.0)),
        },
        group: RequiredIdentity {
            name: "current group",
            id: ids.map(|ids| ids.1),
        },
    }
}

fn verify_target(tool: Tool, path: &Path) -> Result<(), String> {
    if test_config_path().is_some() {
        return path.exists().then_some(()).ok_or_else(|| {
            format!(
                "test {} Target is missing: {}",
                tool.command(),
                path.display()
            )
        });
    }
    let (identifier, team) = match tool {
        Tool::Terraform => ("terraform", "D38WU7D763"),
        Tool::OpenTofu => ("tofu", "ZU76A67LGU"),
    };
    let requirement = format!(
        "=identifier \"{identifier}\" and anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] exists and certificate leaf[field.1.2.840.113635.100.6.1.13] exists and certificate leaf[subject.OU] = \"{team}\""
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
            "{} Target signature is invalid: {}",
            tool.command(),
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
        || !details.contains(&format!("TeamIdentifier={team}"))
        || !details.contains("Timestamp=")
    {
        return Err(format!(
            "{} Target lacks the required Developer ID Hardened Runtime identity",
            tool.command()
        ));
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

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_rejects_partial_credentials_and_installs_exact_helper_config() {
        let mut config = json!({
            "credentials": {
                "app.terraform.io": {"token": "secret"}
            },
            "plugin_cache_dir": "/tmp/plugins"
        });
        let credentials = validate_config(&config).unwrap();
        assert_eq!(credentials["app.terraform.io"], "secret");
        config.as_object_mut().unwrap().remove("credentials");
        config
            .as_object_mut()
            .unwrap()
            .insert("credentials_helper".into(), json!({"av": {"args": []}}));
        assert!(helper_configured(&config));
        assert_eq!(config["plugin_cache_dir"], "/tmp/plugins");
        assert!(
            validate_config(&json!({
                "credentials": {"app.terraform.io": {"token": "secret", "future": true}}
            }))
            .is_err()
        );
    }
}
