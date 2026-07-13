#[cfg(all(target_os = "macos", not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const K6_CLOUD_TOKEN_ENV_KEY: &str = "K6_CLOUD_TOKEN";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[K6_CLOUD_TOKEN_ENV_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    for path in k6_config_paths()? {
        if migrate_credentials_file(&path, &KeychainCredentialStore)? {
            return Ok(());
        }
    }
    Ok(())
}

pub fn migrate_credentials_file(path: &Path, store: &dyn CredentialStore) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    let Some(token) = k6_config_token(&contents)? else {
        return Ok(false);
    };

    store.store_secret(K6_CLOUD_TOKEN_ENV_KEY, &token)?;
    fs::write(path, sanitized_config_json(&contents)?)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn k6_config_paths() -> Result<Vec<PathBuf>, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(vec![
        home.join("Library/Application Support/k6/config.json"),
        home.join(".config/k6/config.json"),
    ])
}

fn config_contains_token(contents: &str) -> bool {
    json_string_field(contents, "token").is_some_and(|value| !value.trim().is_empty())
}

fn k6_config_token(contents: &str) -> Result<Option<String>, String> {
    if !contents.contains("\"token\"") {
        return Ok(None);
    }

    let value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|err| format!("failed to parse k6 config JSON: {err}"))?;

    Ok(value
        .get("token")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string))
}

fn sanitized_config_json(contents: &str) -> Result<String, String> {
    let mut value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|err| format!("failed to parse k6 config JSON: {err}"))?;

    if let Some(object) = value.as_object_mut() {
        object.remove("token");
    }

    let mut json = serde_json::to_string_pretty(&value)
        .map_err(|err| format!("failed to serialize sanitized k6 config JSON: {err}"))?;
    json.push('\n');
    Ok(json)
}

fn json_string_field<'a>(contents: &'a str, field: &str) -> Option<&'a str> {
    let quoted = format!("\"{field}\"");
    let after_key = contents.split(&quoted).nth(1)?.split_once(':')?.1;
    after_key
        .trim_start()
        .strip_prefix('"')?
        .split_once('"')
        .map(|(value, _)| value)
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
        previous: Option<OsString>,
    }

    impl EnvGuard {
        fn set_home(value: Option<&Path>) -> Self {
            let previous = std::env::var_os("HOME");
            unsafe {
                match value {
                    Some(value) => std::env::set_var("HOME", value),
                    None => std::env::remove_var("HOME"),
                }
            }
            Self { previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe {
                match self.previous.take() {
                    Some(value) => std::env::set_var("HOME", value),
                    None => std::env::remove_var("HOME"),
                }
            }
        }
    }

    #[test]
    fn migrates_k6_config() {
        let path = std::env::temp_dir().join(format!("k6-config-{}", std::process::id()));
        let contents = "{\"token\":\"secret\",\"cloud\":{\"name\":\"demo\"}}\n";
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        migrate_credentials_file(&path, &store).unwrap();

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(K6_CLOUD_TOKEN_ENV_KEY.to_string(), "secret".to_string())]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "{\n  \"cloud\": {\n    \"name\": \"demo\"\n  }\n}\n"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn does_not_migrate_without_token() {
        let path = std::env::temp_dir().join(format!("k6-no-token-{}", std::process::id()));
        fs::write(&path, "{\"cloud\":{\"name\":\"demo\"}}\n").unwrap();
        let store = TestCredentialStore::default();

        assert!(!migrate_credentials_file(&path, &store).unwrap());
        assert!(store.values.borrow().is_empty());
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

    #[test]
    fn path_helpers_cover_defaults_and_missing_home() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = test_dir("k6-home");
        {
            let _env = EnvGuard::set_home(Some(&home));
            assert_eq!(
                k6_config_paths().unwrap(),
                vec![
                    home.join("Library/Application Support/k6/config.json"),
                    home.join(".config/k6/config.json")
                ]
            );
            assert_eq!(keys(), &[K6_CLOUD_TOKEN_ENV_KEY]);
            migrate_credentials().unwrap();
        }
        {
            let _env = EnvGuard::set_home(None);
            assert_eq!(k6_config_paths().unwrap_err(), "HOME is not set");
        }
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn json_helpers_cover_invalid_empty_and_sanitized_shapes() {
        assert!(config_contains_token(r#"{"token":"secret"}"#));
        assert!(!config_contains_token(r#"{"token":"   "}"#));
        assert_eq!(k6_config_token(r#"{"token":42}"#).unwrap(), None);
        assert!(
            k6_config_token(r#"{"token":"unterminated"#)
                .unwrap_err()
                .contains("failed to parse k6 config JSON")
        );
        assert_eq!(
            json_string_field(r#"{"cloud":{},"token":"secret"}"#, "token"),
            Some("secret")
        );
        assert_eq!(json_string_field(r#"{"token":false}"#, "token"), None);
        assert_eq!(
            sanitized_config_json(r#"{"token":"secret","cloud":{"token":"nested"}}"#).unwrap(),
            "{\n  \"cloud\": {\n    \"token\": \"nested\"\n  }\n}\n"
        );
    }

    #[test]
    fn reports_read_errors_and_preserves_config_on_store_failure() {
        let temp = test_dir("k6-errors");
        let dir_path = temp.join("dir");
        fs::create_dir_all(&dir_path).unwrap();
        assert!(
            migrate_credentials_file(&dir_path, &TestCredentialStore::default())
                .unwrap_err()
                .contains("failed to read")
        );

        let path = temp.join("config.json");
        let contents = r#"{"token":"secret","cloud":{"name":"demo"}}"#;
        fs::write(&path, contents).unwrap();
        assert_eq!(
            migrate_credentials_file(&path, &FailingCredentialStore).unwrap_err(),
            "store failed"
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);
        fs::remove_dir_all(temp).unwrap();
    }

    fn test_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }
}
