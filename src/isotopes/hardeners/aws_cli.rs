use std::fs;
use std::io::{self, IsTerminal, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use super::{
    HardenerDetection, RequiredExecutable, RequiredIdentity, SecretGateDescriptor, SecretGateRoute,
    StubRequirements,
};

const AWS_ACCESS_KEY_ID: &str = "AWS_ACCESS_KEY_ID";
const AWS_SECRET_ACCESS_KEY: &str = "AWS_SECRET_ACCESS_KEY";
const AWS_HARDEN_PROFILE: &str = "default";
const AWS_STUB: &str = include_str!("aws");
const AWS_STUB_PATH: &str = "/usr/local/bin/aws";

unsafe extern "C" {
    fn geteuid() -> u32;
}

pub(crate) fn run_aws(stdout: &mut dyn Write, yes: bool) -> Result<(), String> {
    let is_root = effective_uid() == 0;
    let has_test_keychain = crate::test_keychain_dir().is_some();
    let should_import_credentials = should_import_aws_credentials(is_root, has_test_keychain);
    let credentials_path = if should_import_credentials {
        Some(aws_credentials_path()?)
    } else {
        None
    };
    let credentials = if let Some(credentials_path) = &credentials_path {
        read_aws_credentials(credentials_path, AWS_HARDEN_PROFILE)?
    } else {
        None
    };

    writeln!(stdout, "╭─ harden aws").ok();
    writeln!(stdout, "│").ok();
    writeln!(
        stdout,
        "◆ This will use Automic Vault for temporary AWS credentials."
    )
    .ok();
    writeln!(stdout, "│").ok();
    if let Some(credentials_path) = &credentials_path {
        if credentials.is_some() {
            writeln!(
                stdout,
                "├─ import {AWS_HARDEN_PROFILE} keys from {} into the login keychain",
                credentials_path.display()
            )
            .ok();
            writeln!(
                stdout,
                "├─ delete {AWS_HARDEN_PROFILE} plaintext keys from {}",
                credentials_path.display()
            )
            .ok();
        } else {
            writeln!(
                stdout,
                "├─ no {AWS_HARDEN_PROFILE} plaintext keys found in {}",
                credentials_path.display()
            )
            .ok();
        }
    }

    if let Some(credentials) = credentials {
        writeln!(stdout, "│").ok();
        if !confirm(stdout, yes)? {
            writeln!(stdout, "╰─ cancelled").ok();
            return Ok(());
        }
        import_aws_credentials(&credentials)?;
        let credentials_path = credentials_path.as_ref().unwrap();
        delete_aws_credentials(credentials_path, AWS_HARDEN_PROFILE)?;
        writeln!(stdout, "├─ imported keys").ok();
        writeln!(stdout, "├─ deleted plaintext keys").ok();
    }

    if !is_root {
        writeln!(stdout, "╰─ finish with `sudo av harden aws`").ok();
        return Ok(());
    }

    writeln!(stdout, "├─ write {AWS_STUB_PATH}").ok();
    writeln!(stdout, "│").ok();
    if !confirm(stdout, yes)? {
        writeln!(stdout, "╰─ cancelled").ok();
        return Ok(());
    }
    install_aws_stub(&aws_stub_path())?;
    if should_import_credentials {
        writeln!(stdout, "╰─ wrote {AWS_STUB_PATH}").ok();
    } else {
        writeln!(stdout, "├─ wrote {AWS_STUB_PATH}").ok();
        writeln!(stdout, "╰─ next: run `av harden aws` to import credentials").ok();
    }
    super::write_secret_gate_notice(stdout, "aws");
    Ok(())
}

pub(crate) fn detect() -> HardenerDetection {
    let path = aws_stub_path();
    let mut detection = HardenerDetection::command(
        is_aws_stub(&path),
        "aws",
        Some(path.display().to_string()),
        "/opt/homebrew/bin/aws".to_string(),
    );
    if crate::test_env_var("AUTOMIC_VAULT_TEST_AWS_STUB_PATH").is_none() {
        detection.commands[0]
            .required_paths
            .push(RequiredExecutable {
                name: "Automic Vault CLI",
                path: "/usr/local/bin/av".to_string(),
            });
    }
    detection.commands[0].stub_requirements = Some(root_stub_requirements(&path));
    detection.commands[0].injected_keys = vec![
        AWS_ACCESS_KEY_ID.to_string(),
        AWS_SECRET_ACCESS_KEY.to_string(),
    ];
    detection
}

fn root_stub_requirements(path: &Path) -> StubRequirements {
    let test_ids = crate::test_env_var("AUTOMIC_VAULT_TEST_AWS_STUB_PATH").and_then(|_| {
        path.parent()
            .and_then(|parent| parent.metadata().ok())
            .map(|metadata| (metadata.uid(), metadata.gid()))
    });
    let (uid, gid) = test_ids.unwrap_or((0, 0));
    StubRequirements {
        mode: 0o755,
        owner: RequiredIdentity {
            name: if test_ids.is_some() {
                "test user"
            } else {
                "root"
            },
            id: Some(uid),
        },
        group: RequiredIdentity {
            name: if test_ids.is_some() {
                "test group"
            } else {
                "wheel"
            },
            id: Some(gid),
        },
    }
}

pub(crate) fn secret_gate() -> SecretGateDescriptor {
    let keys = vec![
        AWS_ACCESS_KEY_ID.to_string(),
        AWS_SECRET_ACCESS_KEY.to_string(),
    ];
    SecretGateDescriptor {
        id: "aws",
        key_patterns: keys.clone(),
        routes: vec![SecretGateRoute {
            operation: "inject",
            script_path: None,
            target_path: "/opt/homebrew/bin/aws".to_string(),
            caller_identifiers: vec!["com.automicvault.av"],
            key_patterns: keys,
            replace_existing_env: false,
            allow_missing_keys: false,
        }],
    }
}

fn confirm(stdout: &mut dyn Write, yes: bool) -> Result<bool, String> {
    if yes {
        writeln!(stdout, "◇ Continue? yes (--yes)").ok();
        return Ok(true);
    }

    write!(stdout, "◇ Continue? [y/N] ").ok();
    stdout
        .flush()
        .map_err(|err| format!("failed to flush prompt: {err}"))?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|err| format!("failed to read confirmation: {err}"))?;
    if !io::stdin().is_terminal() {
        writeln!(stdout).ok();
    }
    Ok(matches!(
        input.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn should_import_aws_credentials(is_root: bool, has_test_keychain: bool) -> bool {
    !is_root || has_test_keychain
}

fn effective_uid() -> u32 {
    crate::test_env_string("AUTOMIC_VAULT_TEST_EUID")
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| unsafe { geteuid() })
}

fn is_aws_stub(path: &Path) -> bool {
    fs::read_to_string(path).is_ok_and(|contents| contents == AWS_STUB)
}

fn aws_stub_path() -> PathBuf {
    crate::test_env_var("AUTOMIC_VAULT_TEST_AWS_STUB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(AWS_STUB_PATH))
}

fn install_aws_stub(path: &Path) -> Result<(), String> {
    fs::write(path, AWS_STUB)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .map_err(|err| format!("failed to chmod {}: {err}", path.display()))
}

#[derive(Debug, PartialEq, Eq)]
struct AwsCredentials {
    access_key_id: String,
    secret_access_key: String,
}

fn aws_credentials_path() -> Result<PathBuf, String> {
    if let Some(path) =
        std::env::var_os("AWS_SHARED_CREDENTIALS_FILE").filter(|path| !path.is_empty())
    {
        return Ok(PathBuf::from(path));
    }
    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
    Ok(PathBuf::from(home).join(".aws/credentials"))
}

fn read_aws_credentials(path: &Path, profile: &str) -> Result<Option<AwsCredentials>, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    parse_aws_credentials(&contents, profile)
}

fn parse_aws_credentials(contents: &str, profile: &str) -> Result<Option<AwsCredentials>, String> {
    let mut in_profile = false;
    let mut access_key_id = None;
    let mut secret_access_key = None;

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if let Some(section) = section_name(trimmed) {
            in_profile = section == profile;
            continue;
        }
        if !in_profile {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "aws_access_key_id" if !value.is_empty() => access_key_id = Some(value.to_string()),
            "aws_secret_access_key" if !value.is_empty() => {
                secret_access_key = Some(value.to_string())
            }
            _ => {}
        }
    }

    match (access_key_id, secret_access_key) {
        (Some(access_key_id), Some(secret_access_key)) => Ok(Some(AwsCredentials {
            access_key_id,
            secret_access_key,
        })),
        (None, None) => Ok(None),
        _ => Err(format!(
            "AWS shared credentials file has incomplete AWS keys for profile {profile}"
        )),
    }
}

fn delete_aws_credentials(path: &Path, profile: &str) -> Result<(), String> {
    let contents = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let cleaned = remove_aws_credentials(&contents, profile);
    fs::write(path, cleaned).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn remove_aws_credentials(contents: &str, profile: &str) -> String {
    let mut in_profile = false;
    let mut out = String::new();
    for line in contents.split_inclusive('\n') {
        let trimmed = line.trim();
        if let Some(section) = section_name(trimmed) {
            in_profile = section == profile;
        }
        if in_profile
            && trimmed.split_once('=').is_some_and(|(key, _)| {
                matches!(key.trim(), "aws_access_key_id" | "aws_secret_access_key")
            })
        {
            continue;
        }
        out.push_str(line);
    }
    out
}

fn section_name(line: &str) -> Option<&str> {
    line.strip_prefix('[')
        .and_then(|line| line.strip_suffix(']'))
        .map(str::trim)
}

fn import_aws_credentials(credentials: &AwsCredentials) -> Result<(), String> {
    store_keychain_secret(AWS_ACCESS_KEY_ID, &credentials.access_key_id)?;
    store_keychain_secret(AWS_SECRET_ACCESS_KEY, &credentials.secret_access_key)
}

pub(crate) fn store_keychain_secret(account: &str, value: &str) -> Result<(), String> {
    crate::secrets::store_secret(account, value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn aws_stub_delegates_directly_to_av() {
        let path = temp_path("aws-stub");
        install_aws_stub(&path).unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), AWS_STUB);
        assert_eq!(AWS_STUB, "#!/usr/local/bin/av aws\n");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn aws_stub_validation_requires_exact_contents() {
        let path = temp_path("aws-stub-exact");
        fs::write(&path, AWS_STUB).unwrap();
        assert!(is_aws_stub(&path));

        fs::write(&path, format!("{AWS_STUB}\n# modified\n")).unwrap();
        assert!(!is_aws_stub(&path));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn root_skips_aws_credential_import_without_test_keychain() {
        assert!(!should_import_aws_credentials(true, false));
        assert!(should_import_aws_credentials(true, true));
        assert!(should_import_aws_credentials(false, false));
    }

    #[test]
    fn root_hardening_skips_credentials_and_installs_stub() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let dir = temp_path("aws-root");
        let aws_stub = dir.join("aws");
        fs::create_dir_all(&dir).unwrap();
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_EUID", "0");
            std::env::set_var("AUTOMIC_VAULT_TEST_AWS_STUB_PATH", &aws_stub);
        }

        let mut stdout = Vec::new();
        run_aws(&mut stdout, true).unwrap();

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_EUID");
            std::env::remove_var("AUTOMIC_VAULT_TEST_AWS_STUB_PATH");
        }
        assert_eq!(fs::read_to_string(&aws_stub).unwrap(), AWS_STUB);
        let stdout = String::from_utf8(stdout).unwrap();
        assert!(!stdout.contains("plaintext keys"));
        assert!(stdout.contains(
            "`aws` defaults to Read Only Access, adjust this in the app: `av open --secret-gate aws`"
        ));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn parses_profile_credentials() {
        assert_eq!(
            parse_aws_credentials(
                "[default]\naws_access_key_id = AKIA\naws_secret_access_key= secret\n",
                "default"
            )
            .unwrap(),
            Some(AwsCredentials {
                access_key_id: "AKIA".to_string(),
                secret_access_key: "secret".to_string()
            })
        );
    }

    #[test]
    fn removes_only_selected_profile_keys() {
        assert_eq!(
            remove_aws_credentials(
                "[default]\naws_access_key_id = AKIA\nregion = us-east-1\naws_secret_access_key = secret\n[dev]\naws_access_key_id = keep\n",
                "default"
            ),
            "[default]\nregion = us-east-1\n[dev]\naws_access_key_id = keep\n"
        );
    }

    #[test]
    fn harden_imports_keys_and_deletes_plaintext_credentials() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let dir = temp_path("aws-import");
        let credentials_path = dir.join("credentials");
        let keychain_dir = dir.join("keychain");
        let aws_vault = dir.join("aws-vault");
        let aws_stub = dir.join("aws");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&aws_vault, "").unwrap();
        fs::write(
            &credentials_path,
            "[default]\naws_access_key_id = AKIA\nregion = us-east-1\naws_secret_access_key = secret\n[dev]\naws_access_key_id = DEV\naws_secret_access_key = dev-secret\n",
        )
        .unwrap();
        unsafe {
            std::env::set_var("AWS_SHARED_CREDENTIALS_FILE", &credentials_path);
            std::env::set_var("AWS_PROFILE", "dev");
            std::env::set_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR", &keychain_dir);
            std::env::set_var("AUTOMIC_VAULT_TEST_AWS_VAULT_PATH", &aws_vault);
            std::env::set_var("AUTOMIC_VAULT_TEST_AWS_STUB_PATH", &aws_stub);
        }

        run_aws(&mut Vec::new(), true).unwrap();

        unsafe {
            std::env::remove_var("AWS_SHARED_CREDENTIALS_FILE");
            std::env::remove_var("AWS_PROFILE");
            std::env::remove_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR");
            std::env::remove_var("AUTOMIC_VAULT_TEST_AWS_VAULT_PATH");
            std::env::remove_var("AUTOMIC_VAULT_TEST_AWS_STUB_PATH");
        }
        assert_eq!(
            fs::read_to_string(keychain_dir.join(AWS_ACCESS_KEY_ID)).unwrap(),
            "AKIA"
        );
        assert_eq!(
            fs::read_to_string(keychain_dir.join(AWS_SECRET_ACCESS_KEY)).unwrap(),
            "secret"
        );
        assert_eq!(
            fs::read_to_string(credentials_path).unwrap(),
            "[default]\nregion = us-east-1\n[dev]\naws_access_key_id = DEV\naws_secret_access_key = dev-secret\n"
        );
        let _ = fs::remove_dir_all(dir);
    }

    fn temp_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("av-{label}-{}-{nanos}", std::process::id()))
    }
}
