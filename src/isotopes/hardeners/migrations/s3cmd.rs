#[cfg(all(target_os = "macos", not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const S3CMD_ENV_ASSIGNMENTS_KEY: &str = "S3CMD_ENV_ASSIGNMENTS";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[S3CMD_ENV_ASSIGNMENTS_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_config_file(&s3cmd_config_path()?, &KeychainCredentialStore).map(|_| ())
}

pub fn migrate_config_file(path: &Path, store: &dyn CredentialStore) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    let assignments = s3cmd_env_assignments(&contents)?;
    if assignments.is_empty() {
        return Ok(false);
    }

    store.store_secret(S3CMD_ENV_ASSIGNMENTS_KEY, &assignments.join("\n"))?;
    fs::write(path, sanitized_config(&contents))
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn s3cmd_config_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".s3cfg"))
}

fn sanitized_config(contents: &str) -> String {
    let mut changed = false;
    let mut output = Vec::new();

    for line in contents.lines() {
        let sanitized = sanitize_line(line, &mut changed);
        output.push(sanitized);
    }

    if !changed {
        return contents.to_string();
    }

    let mut rendered = output.join("\n");
    if contents.ends_with('\n') {
        rendered.push('\n');
    }
    rendered
}

fn sanitize_line(line: &str, changed: &mut bool) -> String {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
        return line.to_string();
    }

    let Some((before_equals, after_equals)) = line.split_once('=') else {
        return line.to_string();
    };
    let key = before_equals.trim().to_ascii_lowercase();
    let Some(env_key) = sensitive_env_key(&key) else {
        return line.to_string();
    };

    let value = after_equals.trim();
    if value.is_empty() || value == "\"\"" || value == "''" {
        return line.to_string();
    }

    *changed = true;
    if env_key == "S3CMD_GPG_PASSPHRASE" {
        return format!("{before_equals}= ${env_key}");
    }
    format!("{before_equals}= ")
}

fn s3cmd_env_assignments(contents: &str) -> Result<Vec<String>, String> {
    let secrets = s3cmd_secrets(contents)?;
    if secrets.is_empty() {
        return Ok(Vec::new());
    }
    validate_credential_pair(&secrets)?;

    let mut assignments = Vec::new();
    for env_key in [
        "AWS_ACCESS_KEY_ID",
        "AWS_SECRET_ACCESS_KEY",
        "AWS_SESSION_TOKEN",
        "S3CMD_GPG_PASSPHRASE",
    ] {
        if let Some(value) = secrets
            .iter()
            .find(|secret| secret.env_key == env_key)
            .map(|secret| secret.value.as_str())
        {
            reject_env_line_breaks(env_key, value)?;
            assignments.push(format!("{env_key}={value}"));
        }
    }
    Ok(assignments)
}

struct SecretValue {
    env_key: &'static str,
    value: String,
}

fn s3cmd_secrets(contents: &str) -> Result<Vec<SecretValue>, String> {
    let mut secrets = Vec::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        let Some((before_equals, after_equals)) = line.split_once('=') else {
            continue;
        };
        let key = before_equals.trim().to_ascii_lowercase();
        let Some(env_key) = sensitive_env_key(&key) else {
            continue;
        };
        let value = unquote_config_value(after_equals.trim());
        if value.is_empty() {
            continue;
        }
        merge_secret(&mut secrets, env_key, value)?;
    }
    Ok(secrets)
}

fn merge_secret(
    secrets: &mut Vec<SecretValue>,
    env_key: &'static str,
    value: &str,
) -> Result<(), String> {
    if let Some(existing) = secrets.iter().find(|secret| secret.env_key == env_key) {
        if existing.value != value {
            return Err(format!(
                "multiple s3cmd values map to {env_key}; migrate them manually"
            ));
        }
        return Ok(());
    }
    secrets.push(SecretValue {
        env_key,
        value: value.to_string(),
    });
    Ok(())
}

fn validate_credential_pair(secrets: &[SecretValue]) -> Result<(), String> {
    let has_access_key = secrets
        .iter()
        .any(|secret| secret.env_key == "AWS_ACCESS_KEY_ID");
    let has_secret_key = secrets
        .iter()
        .any(|secret| secret.env_key == "AWS_SECRET_ACCESS_KEY");
    if has_access_key != has_secret_key {
        return Err(
            "s3cmd access_key and secret_key must both be present to migrate to env vars"
                .to_string(),
        );
    }

    let has_session_token = secrets
        .iter()
        .any(|secret| secret.env_key == "AWS_SESSION_TOKEN");
    if has_session_token && !has_access_key {
        return Err(
            "s3cmd session token cannot be migrated without access_key and secret_key".to_string(),
        );
    }
    Ok(())
}

fn sensitive_env_key(key: &str) -> Option<&'static str> {
    match key {
        "access_key" => Some("AWS_ACCESS_KEY_ID"),
        "secret_key" => Some("AWS_SECRET_ACCESS_KEY"),
        "access_token" | "session_token" => Some("AWS_SESSION_TOKEN"),
        "gpg_passphrase" => Some("S3CMD_GPG_PASSPHRASE"),
        _ => None,
    }
}

fn unquote_config_value(value: &str) -> &str {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
        {
            return &value[1..value.len() - 1];
        }
    }
    value
}

fn reject_env_line_breaks(key: &str, value: &str) -> Result<(), String> {
    if value.contains('\n') || value.contains('\r') {
        return Err(format!("{key} cannot contain line breaks"));
    }
    Ok(())
}

impl CredentialStore for KeychainCredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String> {
        keychain_store_secret(KEYCHAIN_SERVICE, key, value)
    }
}

#[cfg(all(target_os = "macos", not(coverage)))]
fn keychain_store_secret(service: &str, account: &str, value: &str) -> Result<(), String> {
    unsafe extern "C" {
        fn isotope_store_generic_password_json(
            service_cstr: *const c_char,
            account_cstr: *const c_char,
            value_cstr: *const c_char,
            error_cstr: *mut *mut c_char,
        ) -> bool;
    }

    let service_cstr =
        CString::new(service).map_err(|_| "invalid keychain service name".to_string())?;
    let account_cstr =
        CString::new(account).map_err(|_| "invalid keychain account name".to_string())?;
    let value_cstr =
        CString::new(value).map_err(|_| "invalid keychain secret value".to_string())?;
    let mut error = std::ptr::null_mut();
    if unsafe {
        isotope_store_generic_password_json(
            service_cstr.as_ptr(),
            account_cstr.as_ptr(),
            value_cstr.as_ptr(),
            &mut error,
        )
    } {
        return Ok(());
    }

    let message =
        unsafe { take_bridge_string(error) }.unwrap_or_else(|| "keychain write failed".to_string());
    Err(format!("failed to store secret {account}: {message}"))
}

#[cfg(any(not(target_os = "macos"), coverage))]
fn keychain_store_secret(_service: &str, _account: &str, _value: &str) -> Result<(), String> {
    Err("Automic Vault secret storage is only available on macOS".to_string())
}

#[cfg(all(target_os = "macos", not(coverage)))]
unsafe fn take_bridge_string(value: *mut c_char) -> Option<String> {
    if value.is_null() {
        return None;
    }

    unsafe extern "C" {
        fn isotope_free_c_string(value: *mut c_char);
    }

    let bytes = unsafe { std::ffi::CStr::from_ptr(value) }
        .to_str()
        .ok()
        .map(str::to_owned);
    unsafe { isotope_free_c_string(value) };
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct TestCredentialStore {
        values: RefCell<Vec<(String, String)>>,
    }

    impl CredentialStore for TestCredentialStore {
        fn store_secret(&self, key: &str, value: &str) -> Result<(), String> {
            self.values
                .borrow_mut()
                .push((key.to_string(), value.to_string()));
            Ok(())
        }
    }

    #[test]
    fn blanks_sensitive_values_but_keeps_other_settings() {
        let contents = "\
access_key = AKIAEXAMPLE\n\
secret_key = very-secret\n\
gpg_passphrase = also-secret\n\
host_base = s3.amazonaws.com\n";
        let sanitized = sanitized_config(contents);

        assert!(sanitized.contains("access_key = "));
        assert!(sanitized.contains("secret_key = "));
        assert!(sanitized.contains("gpg_passphrase = $S3CMD_GPG_PASSPHRASE"));
        assert!(sanitized.contains("host_base = s3.amazonaws.com"));
    }

    #[test]
    fn migrates_default_config_and_stores_env_assignments() {
        let path = std::env::temp_dir().join(format!("s3cfg-{}", std::process::id()));
        let contents = "\
access_key = AKIAEXAMPLE\n\
secret_key = very-secret\n\
access_token = session-token\n\
gpg_passphrase = also-secret\n";
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        migrate_config_file(&path, &store).unwrap();

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                S3CMD_ENV_ASSIGNMENTS_KEY.to_string(),
                "AWS_ACCESS_KEY_ID=AKIAEXAMPLE\nAWS_SECRET_ACCESS_KEY=very-secret\nAWS_SESSION_TOKEN=session-token\nS3CMD_GPG_PASSPHRASE=also-secret".to_string()
            )]
        );
        let sanitized = fs::read_to_string(&path).unwrap();
        assert!(sanitized.contains("access_key = "));
        assert!(sanitized.contains("secret_key = "));
        assert!(sanitized.contains("access_token = "));
        assert!(sanitized.contains("gpg_passphrase = $S3CMD_GPG_PASSPHRASE"));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_incomplete_access_key_pair() {
        let err = s3cmd_env_assignments("access_key = AKIAEXAMPLE\n").unwrap_err();

        assert!(err.contains("access_key and secret_key"));
    }

    #[test]
    fn rejects_conflicting_session_token_keys() {
        let contents = "\
access_key = AKIAEXAMPLE\n\
secret_key = very-secret\n\
access_token = first-token\n\
session_token = second-token\n";

        let err = s3cmd_env_assignments(contents).unwrap_err();

        assert!(err.contains("AWS_SESSION_TOKEN"));
    }

    #[test]
    fn top_level_migrate_credentials_ignores_missing_default_location() {
        let home = std::env::temp_dir().join(format!(
            "{}-migrate-missing-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();

        let previous_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", &home);
        }

        migrate_credentials().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }

        std::fs::remove_dir_all(home).unwrap();
    }
}
