#[cfg(all(target_os = "macos", not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const SNYK_ENV_ASSIGNMENTS_KEY: &str = "SNYK_ENV_ASSIGNMENTS";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[SNYK_ENV_ASSIGNMENTS_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_credentials_file(&snyk_config_path()?, &KeychainCredentialStore).map(|_| ())
}

pub fn migrate_credentials_file(path: &Path, store: &dyn CredentialStore) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };

    let assignments = snyk_env_assignments(&contents)?;
    if assignments.is_empty() {
        return Ok(false);
    }

    store.store_secret(SNYK_ENV_ASSIGNMENTS_KEY, &assignments.join("\n"))?;
    fs::write(path, sanitized_config_json(&contents)?)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn snyk_config_path() -> Result<PathBuf, String> {
    let config_home = if let Some(path) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(path)
    } else {
        user_home()?.join(".config")
    };
    Ok(config_home.join("configstore/snyk.json"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn config_has_secrets(contents: &str) -> bool {
    for key in [
        "api",
        "token",
        "oauth-token",
        "oauthToken",
        "oci-registry-password",
        "client-secret",
        "clientSecret",
    ] {
        if json_string_key_has_nonempty_value(contents, key) {
            return true;
        }
    }
    false
}

fn snyk_env_assignments(contents: &str) -> Result<Vec<String>, String> {
    let value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|err| format!("failed to parse snyk configstore JSON: {err}"))?;
    if has_unsupported_secret(&value) {
        return Err(
            "Snyk configstore contains registry passwords or client secrets; migrate them manually"
                .to_string(),
        );
    }

    let api_token = unique_string_values(&value, &["api", "token"])?;
    let oauth_token = unique_string_values(&value, &["oauth-token", "oauthToken"])?;
    if api_token.is_none() && oauth_token.is_none() {
        return Ok(Vec::new());
    }

    let mut assignments = Vec::new();
    if let Some(token) = api_token {
        reject_env_line_breaks("SNYK_TOKEN", &token)?;
        assignments.push(format!("SNYK_TOKEN={token}"));
    }
    if let Some(token) = oauth_token {
        reject_env_line_breaks("SNYK_OAUTH_TOKEN", &token)?;
        assignments.push(format!("SNYK_OAUTH_TOKEN={token}"));
    }
    Ok(assignments)
}

fn sanitized_config_json(contents: &str) -> Result<String, String> {
    let mut value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|err| format!("failed to parse snyk configstore JSON: {err}"))?;
    if let Some(object) = value.as_object_mut() {
        for key in ["api", "token", "oauth-token", "oauthToken"] {
            if object
                .get(key)
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| !value.is_empty())
            {
                object.insert(key.to_string(), serde_json::Value::String(String::new()));
            }
        }
    }
    let mut json = serde_json::to_string_pretty(&value)
        .map_err(|err| format!("failed to encode sanitized snyk configstore JSON: {err}"))?;
    json.push('\n');
    Ok(json)
}

fn has_unsupported_secret(value: &serde_json::Value) -> bool {
    ["oci-registry-password", "client-secret", "clientSecret"]
        .iter()
        .any(|key| {
            value
                .get(*key)
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| !value.is_empty())
        })
}

fn unique_string_values(
    value: &serde_json::Value,
    keys: &[&str],
) -> Result<Option<String>, String> {
    let mut values = Vec::new();
    for key in keys {
        if let Some(value) = value
            .get(*key)
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
            && !values.iter().any(|existing| existing == value)
        {
            values.push(value.to_string());
        }
    }

    match values.len() {
        0 => Ok(None),
        1 => Ok(values.pop()),
        _ => Err(format!(
            "Snyk configstore has conflicting values for {}; migrate it manually",
            keys.join("/")
        )),
    }
}

fn reject_env_line_breaks(key: &str, value: &str) -> Result<(), String> {
    if value.contains('\n') || value.contains('\r') {
        return Err(format!("{key} cannot contain line breaks"));
    }
    Ok(())
}

fn json_string_key_has_nonempty_value(contents: &str, key: &str) -> bool {
    let quoted_key = format!("\"{key}\"");
    let mut rest = contents;
    while let Some(index) = rest.find(&quoted_key) {
        let after_key = &rest[index + quoted_key.len()..];
        let Some(colon_index) = after_key.find(':') else {
            return false;
        };
        let value = after_key[colon_index + 1..].trim_start();
        if value.starts_with('"') {
            return !value.starts_with("\"\"");
        }
        rest = &after_key[colon_index + 1..];
    }
    false
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
    Err(format!("failed to store isotope key {account}: {message}"))
}

#[cfg(any(not(target_os = "macos"), coverage))]
fn keychain_store_secret(_service: &str, _account: &str, _value: &str) -> Result<(), String> {
    Err("isotope keychain integration is only available on macOS".to_string())
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
    fn migrates_config_with_api_token() {
        let temp = test_dir("snyk-migrate");
        let config = temp.join("snyk.json");
        fs::write(
            &config,
            r#"{"api":"snyk-token","oauthToken":"oauth-token","org":"example"}"#,
        )
        .unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_credentials_file(&config, &store).unwrap());

        let values = store.values.borrow();
        assert_eq!(
            values.as_slice(),
            &[(
                SNYK_ENV_ASSIGNMENTS_KEY.to_string(),
                "SNYK_TOKEN=snyk-token\nSNYK_OAUTH_TOKEN=oauth-token".to_string()
            )]
        );
        let sanitized = fs::read_to_string(config).unwrap();
        assert!(sanitized.contains("\"api\": \"\""));
        assert!(sanitized.contains("\"oauthToken\": \"\""));
        assert!(sanitized.contains("\"org\": \"example\""));
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn rejects_unsupported_secret_shapes() {
        let err = snyk_env_assignments(
            r#"{"api":"snyk-token","oci-registry-password":"registry-secret"}"#,
        )
        .unwrap_err();

        assert!(err.contains("registry passwords"));
    }

    #[test]
    fn rejects_conflicting_api_token_keys() {
        let err = snyk_env_assignments(r#"{"api":"one","token":"two"}"#).unwrap_err();

        assert!(err.contains("conflicting values"));
    }

    #[test]
    fn does_not_migrate_without_secret_keys() {
        let temp = test_dir("snyk-no-migrate");
        let config = temp.join("snyk.json");
        fs::write(&config, r#"{"endpoint":"https://api.snyk.io"}"#).unwrap();
        let store = TestCredentialStore::default();

        assert!(!migrate_credentials_file(&config, &store).unwrap());
        assert!(store.values.borrow().is_empty());
        fs::remove_dir_all(temp).unwrap();
    }

    fn test_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn top_level_migrate_credentials_ignores_missing_default_locations() {
        let home = std::env::temp_dir().join(format!(
            "{}-migrate-missing-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let xdg = home.join("xdg");
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&xdg).unwrap();

        let previous_home = std::env::var_os("HOME");
        let previous_xdg = std::env::var_os("XDG_CONFIG_HOME");
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("XDG_CONFIG_HOME", &xdg);
        }

        migrate_credentials().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_xdg {
                Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
        }

        std::fs::remove_dir_all(home).unwrap();
    }
}
