#[cfg(all(target_os = "macos", not(test), not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const CIVO_TOKEN_ENV_KEY: &str = "CIVO_TOKEN";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[CIVO_TOKEN_ENV_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_credentials_file(&civo_config_path()?, &KeychainCredentialStore).map(|_| ())
}

pub fn migrate_credentials_file(path: &Path, store: &dyn CredentialStore) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };

    if !contents.contains("\"apikey\"") && !contents.contains("\"apikeys\"") {
        return Ok(false);
    }

    let Some(token) = civo_config_token(&contents)? else {
        return Ok(false);
    };

    store.store_secret(CIVO_TOKEN_ENV_KEY, &token)?;
    fs::write(path, sanitized_config_json(&contents)?)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn civo_config_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("CIVO_CONFIG").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    Ok(user_home()?.join(".civo.json"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn civo_config_token(contents: &str) -> Result<Option<String>, String> {
    let value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|err| format!("failed to parse civo config JSON: {err}"))?;

    if let Some(token) = non_empty_json_string(value.get("apikey")) {
        return Ok(Some(token.to_string()));
    }

    let Some(api_keys) = value.get("apikeys").and_then(serde_json::Value::as_object) else {
        return Ok(None);
    };

    if let Some(current_name) = non_empty_json_string(value.get("current_apikey")) {
        if let Some(token) = non_empty_json_string(api_keys.get(current_name)) {
            return Ok(Some(token.to_string()));
        }
    }

    Ok(api_keys
        .values()
        .find_map(|value| non_empty_json_string(Some(value)))
        .map(str::to_string))
}

fn non_empty_json_string(value: Option<&serde_json::Value>) -> Option<&str> {
    value
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
}

fn sanitized_config_json(contents: &str) -> Result<String, String> {
    let mut value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|err| format!("failed to parse civo config JSON: {err}"))?;

    if let Some(object) = value.as_object_mut() {
        object.remove("apikey");
        object.remove("apikeys");
    }

    let mut json = serde_json::to_string_pretty(&value)
        .map_err(|err| format!("failed to serialize sanitized civo config JSON: {err}"))?;
    json.push('\n');
    Ok(json)
}

impl CredentialStore for KeychainCredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String> {
        keychain_store_secret(KEYCHAIN_SERVICE, key, value)
    }
}

#[cfg(all(target_os = "macos", not(test), not(coverage)))]
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

#[cfg(any(not(target_os = "macos"), test, coverage))]
fn keychain_store_secret(_service: &str, _account: &str, _value: &str) -> Result<(), String> {
    Err("isotope keychain integration is only available on macOS".to_string())
}

#[cfg(all(target_os = "macos", not(test), not(coverage)))]
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
    use std::ffi::OsString;

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

    struct FailingCredentialStore;

    impl CredentialStore for FailingCredentialStore {
        fn store_secret(&self, _key: &str, _value: &str) -> Result<(), String> {
            Err("store failed".to_string())
        }
    }

    struct EnvGuard {
        values: Vec<(&'static str, Option<OsString>)>,
    }

    impl EnvGuard {
        fn set(values: &[(&'static str, Option<&Path>)]) -> Self {
            let previous = values
                .iter()
                .map(|(key, value)| {
                    let previous = std::env::var_os(key);
                    unsafe {
                        match value {
                            Some(value) => std::env::set_var(key, value),
                            None => std::env::remove_var(key),
                        }
                    }
                    (*key, previous)
                })
                .collect();
            Self { values: previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, value) in self.values.drain(..).rev() {
                unsafe {
                    match value {
                        Some(value) => std::env::set_var(key, value),
                        None => std::env::remove_var(key),
                    }
                }
            }
        }
    }

    #[test]
    fn migrates_civo_config() {
        let path = std::env::temp_dir().join(format!("civo-config-{}", std::process::id()));
        let contents = r#"{"apikey":"fake-civo-key","region":"NYC1"}"#;
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_credentials_file(&path, &store).unwrap());

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(CIVO_TOKEN_ENV_KEY.to_string(), "fake-civo-key".to_string())]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "{\n  \"region\": \"NYC1\"\n}\n"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn migrates_selected_named_api_key_and_preserves_profile_selection() {
        let path = std::env::temp_dir().join(format!("civo-named-config-{}", std::process::id()));
        let contents = r#"{"apikeys":{"work":"fake-work-key","test":"fake-test-key"},"current_apikey":"work","region":"NYC1"}"#;
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_credentials_file(&path, &store).unwrap());

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(CIVO_TOKEN_ENV_KEY.to_string(), "fake-work-key".to_string())]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "{\n  \"current_apikey\": \"work\",\n  \"region\": \"NYC1\"\n}\n"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn does_not_migrate_without_api_key() {
        let path = std::env::temp_dir().join(format!("civo-no-key-{}", std::process::id()));
        fs::write(&path, r#"{"region":"NYC1"}"#).unwrap();
        let store = TestCredentialStore::default();

        assert!(!migrate_credentials_file(&path, &store).unwrap());
        assert!(store.values.borrow().is_empty());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn reports_invalid_json_when_api_key_migration_is_needed() {
        let path = std::env::temp_dir().join(format!("civo-invalid-{}", std::process::id()));
        fs::write(&path, r#"{"apikey":"fake-civo-key""#).unwrap();
        let store = TestCredentialStore::default();

        let error = migrate_credentials_file(&path, &store).unwrap_err();

        assert!(error.contains("failed to parse civo config JSON"));
        assert!(store.values.borrow().is_empty());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn path_helpers_cover_env_home_and_missing_home() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = test_dir("civo-home");
        let explicit = home.join("custom-civo.json");
        {
            let _env = EnvGuard::set(&[("HOME", Some(&home)), ("CIVO_CONFIG", Some(&explicit))]);
            assert_eq!(civo_config_path().unwrap(), explicit);
            assert_eq!(keys(), &[CIVO_TOKEN_ENV_KEY]);
            migrate_credentials().unwrap();
        }
        {
            let _env = EnvGuard::set(&[("HOME", Some(&home)), ("CIVO_CONFIG", None)]);
            assert_eq!(civo_config_path().unwrap(), home.join(".civo.json"));
        }
        {
            let _env = EnvGuard::set(&[("HOME", None), ("CIVO_CONFIG", None)]);
            assert_eq!(user_home().unwrap_err(), "HOME is not set");
            assert_eq!(civo_config_path().unwrap_err(), "HOME is not set");
        }
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn token_helpers_cover_fallbacks_and_empty_values() {
        assert_eq!(
            civo_config_token(r#"{"apikeys":{"first":"fake-first","second":""}}"#)
                .unwrap()
                .as_deref(),
            Some("fake-first")
        );
        assert_eq!(
            civo_config_token(r#"{"apikey":"","apikeys":{"first":""}}"#).unwrap(),
            None
        );
        assert_eq!(non_empty_json_string(None), None);
        assert_eq!(
            sanitized_config_json(r#"{"apikeys":{"first":"fake"},"region":"NYC1"}"#).unwrap(),
            "{\n  \"region\": \"NYC1\"\n}\n"
        );
    }

    #[test]
    fn read_and_store_errors_preserve_config() {
        let temp = test_dir("civo-errors");
        let dir_path = temp.join("dir");
        fs::create_dir_all(&dir_path).unwrap();
        assert!(
            migrate_credentials_file(&dir_path, &TestCredentialStore::default())
                .unwrap_err()
                .contains("failed to read")
        );

        let path = temp.join("config.json");
        let contents = r#"{"apikey":"fake-civo-key","region":"NYC1"}"#;
        fs::write(&path, contents).unwrap();
        assert_eq!(
            migrate_credentials_file(&path, &FailingCredentialStore).unwrap_err(),
            "store failed"
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);

        assert!(
            KeychainCredentialStore
                .store_secret(CIVO_TOKEN_ENV_KEY, "fake-civo-key")
                .unwrap_err()
                .contains("keychain integration")
        );
        fs::remove_dir_all(temp).unwrap();
    }

    fn test_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }
}
