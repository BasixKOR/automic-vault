#[cfg(all(target_os = "macos", not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const PULUMI_ACCESS_TOKEN_ENV_KEY: &str = "PULUMI_ACCESS_TOKEN";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[PULUMI_ACCESS_TOKEN_ENV_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_credentials_file(&pulumi_credentials_path()?, &KeychainCredentialStore).map(|_| ())
}

pub fn migrate_credentials_file(path: &Path, store: &dyn CredentialStore) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    let access_tokens = pulumi_credentials_access_tokens(&contents)?;
    if access_tokens.is_empty() {
        return Ok(false);
    }
    if access_tokens.len() > 1 {
        return Err("multiple Pulumi access tokens found; migrate them manually".to_string());
    }

    store.store_secret(PULUMI_ACCESS_TOKEN_ENV_KEY, &access_tokens[0])?;
    fs::write(path, sanitized_credentials_json(&contents)?)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn pulumi_credentials_path() -> Result<PathBuf, String> {
    if let Some(dir) = std::env::var_os("PULUMI_CREDENTIALS_PATH").filter(|value| !value.is_empty())
    {
        return Ok(PathBuf::from(dir).join("credentials.json"));
    }
    if let Some(dir) = std::env::var_os("PULUMI_HOME").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(dir).join("credentials.json"));
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".pulumi/credentials.json"))
}

fn pulumi_credentials_contains_access_token(contents: &str) -> bool {
    pulumi_credentials_access_tokens(contents).is_ok_and(|tokens| !tokens.is_empty())
}

fn pulumi_credentials_access_tokens(contents: &str) -> Result<Vec<String>, String> {
    let value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|err| format!("failed to parse Pulumi credentials JSON: {err}"))?;
    let Some(access_tokens) = value
        .get("accessTokens")
        .and_then(serde_json::Value::as_object)
    else {
        return Ok(Vec::new());
    };
    Ok(access_tokens
        .values()
        .filter_map(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect())
}

fn sanitized_credentials_json(contents: &str) -> Result<String, String> {
    let mut value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|err| format!("failed to parse Pulumi credentials JSON: {err}"))?;
    if let Some(object) = value.as_object_mut() {
        object.remove("accessTokens");
    }
    let mut json = serde_json::to_string_pretty(&value)
        .map_err(|err| format!("failed to encode Pulumi credentials JSON: {err}"))?;
    json.push('\n');
    Ok(json)
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
    fn keys_include_pulumi_access_token() {
        assert_eq!(keys(), &[PULUMI_ACCESS_TOKEN_ENV_KEY]);
    }

    #[test]
    fn migrates_single_access_token() {
        let path = std::env::temp_dir().join(format!(
            "pulumi-credentials-{}-credentials.json",
            std::process::id()
        ));
        let contents = r#"{"current":"https://api.pulumi.com","accessTokens":{"https://api.pulumi.com":"pul-secret"}}"#;
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        migrate_credentials_file(&path, &store).unwrap();

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                PULUMI_ACCESS_TOKEN_ENV_KEY.to_string(),
                "pul-secret".to_string()
            )]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "{\n  \"current\": \"https://api.pulumi.com\"\n}\n"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn refuses_multiple_access_tokens() {
        let path = std::env::temp_dir().join(format!(
            "pulumi-credentials-multiple-{}-credentials.json",
            std::process::id()
        ));
        let contents = r#"{"accessTokens":{"https://api.pulumi.com":"pul-secret","https://api.example.invalid":"other-secret"}}"#;
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert_eq!(
            migrate_credentials_file(&path, &store).unwrap_err(),
            "multiple Pulumi access tokens found; migrate them manually"
        );
        assert!(store.values.borrow().is_empty());
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);
        fs::remove_file(path).unwrap();
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
