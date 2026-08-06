#[cfg(all(target_os = "macos", not(test), not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const GOTIFY_TOKEN_ENV_KEY: &str = "GOTIFY_TOKEN";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[GOTIFY_TOKEN_ENV_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_default_files(&KeychainCredentialStore).map(|_| ())
}

pub fn migrate_default_files(store: &dyn CredentialStore) -> Result<bool, String> {
    let paths = gotify_user_config_paths()?;
    migrate_config_files(&paths, store)
}

pub fn migrate_config_files(
    paths: &[PathBuf],
    store: &dyn CredentialStore,
) -> Result<bool, String> {
    let mut migrated = false;
    for path in paths {
        migrated |= migrate_config_file(path, store)?;
    }
    Ok(migrated)
}

pub fn migrate_config_file(path: &Path, store: &dyn CredentialStore) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    let Some(token) = json_string_value(&contents, "token").filter(|value| !value.is_empty())
    else {
        return Ok(false);
    };

    store.store_secret(GOTIFY_TOKEN_ENV_KEY, token)?;
    fs::write(path, rewrite_json_string_key(&contents, "token", ""))
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn gotify_user_config_paths() -> Result<Vec<PathBuf>, String> {
    Ok(vec![
        config_home()?.join("gotify/cli.json"),
        user_home()?.join(".gotify/cli.json"),
    ])
}

fn config_home() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    Ok(user_home()?.join(".config"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn json_string_value<'a>(contents: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\"");
    let mut remaining = contents;
    while let Some(index) = remaining.find(&needle) {
        let after_key = &remaining[index + needle.len()..];
        let Some(after_colon) = after_key
            .split_once(':')
            .map(|(_, value)| value.trim_start())
        else {
            return None;
        };
        if let Some(value) = after_colon.strip_prefix('"') {
            let end = value.find('"')?;
            return Some(&value[..end]);
        }
        if after_key.is_empty() {
            return None;
        }
        remaining = &after_key[1..];
    }
    None
}

fn rewrite_json_string_key(contents: &str, key: &str, replacement: &str) -> String {
    let needle = format!("\"{key}\"");
    let Some(key_index) = contents.find(&needle) else {
        return contents.to_string();
    };
    let after_key_index = key_index + needle.len();
    let Some(colon_offset) = contents[after_key_index..].find(':') else {
        return contents.to_string();
    };
    let after_colon_index = after_key_index + colon_offset + 1;
    let value_start = after_colon_index
        + contents[after_colon_index..]
            .find('"')
            .unwrap_or(contents.len() - after_colon_index);
    if value_start >= contents.len() {
        return contents.to_string();
    }
    let value_body_start = value_start + 1;
    let Some(value_end_offset) = contents[value_body_start..].find('"') else {
        return contents.to_string();
    };
    let value_end = value_body_start + value_end_offset;

    let mut output = String::with_capacity(contents.len() + replacement.len());
    output.push_str(&contents[..value_body_start]);
    output.push_str(replacement);
    output.push_str(&contents[value_end..]);
    output
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
    Err(format!("failed to store secret {account}: {message}"))
}

#[cfg(any(not(target_os = "macos"), test, coverage))]
fn keychain_store_secret(_service: &str, _account: &str, _value: &str) -> Result<(), String> {
    Err("Automic Vault secret storage is only available on macOS".to_string())
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
    use std::fs;

    #[derive(Default)]
    struct TestCredentialStore {
        values: RefCell<Vec<(String, String)>>,
    }

    struct FailingStore;

    impl CredentialStore for TestCredentialStore {
        fn store_secret(&self, key: &str, value: &str) -> Result<(), String> {
            self.values
                .borrow_mut()
                .push((key.to_string(), value.to_string()));
            Ok(())
        }
    }

    impl CredentialStore for FailingStore {
        fn store_secret(&self, _key: &str, _value: &str) -> Result<(), String> {
            Err("store failed".to_string())
        }
    }

    #[test]
    fn migrates_token_and_preserves_other_config() {
        let path = std::env::temp_dir().join(format!("gotify-config-{}", std::process::id()));
        fs::write(
            &path,
            r#"{"token":"fake-gotify-token","url":"https://push.example","defaultPriority":5}"#,
        )
        .unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_config_file(&path, &store).unwrap());

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                GOTIFY_TOKEN_ENV_KEY.to_string(),
                "fake-gotify-token".to_string()
            )]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            r#"{"token":"","url":"https://push.example","defaultPriority":5}"#
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn ignores_missing_or_empty_token() {
        let missing = std::env::temp_dir().join(format!("missing-gotify-{}", std::process::id()));
        let empty = std::env::temp_dir().join(format!("empty-gotify-{}", std::process::id()));
        fs::write(&empty, r#"{"token":"","url":"https://push.example"}"#).unwrap();
        let store = TestCredentialStore::default();

        assert!(!migrate_config_file(&missing, &store).unwrap());
        assert!(!migrate_config_file(&empty, &store).unwrap());
        assert!(store.values.borrow().is_empty());
        fs::remove_file(empty).unwrap();
    }

    #[test]
    fn migrates_multiple_config_paths() {
        let temp = std::env::temp_dir().join(format!("gotify-multi-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        let xdg = temp.join("xdg.json");
        let legacy = temp.join("legacy.json");
        fs::write(
            &xdg,
            r#"{"token":"fake-xdg-token","url":"https://one.example"}"#,
        )
        .unwrap();
        fs::write(
            &legacy,
            r#"{"token":"fake-legacy-token","url":"https://two.example"}"#,
        )
        .unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_config_files(&[xdg, legacy], &store).unwrap());

        assert_eq!(store.values.borrow().len(), 2);
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn migrate_config_file_propagates_store_and_read_errors() {
        let temp =
            std::env::temp_dir().join(format!("gotify-migrate-errors-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        let config = temp.join("cli.json");
        fs::write(&config, r#"{"token":"gotify-token"}"#).unwrap();

        assert_eq!(
            migrate_config_file(&config, &FailingStore).unwrap_err(),
            "store failed"
        );
        assert_eq!(
            migrate_config_file(&temp, &TestCredentialStore::default()).unwrap_err(),
            format!(
                "failed to read {}: Is a directory (os error 21)",
                temp.display()
            )
        );

        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn gotify_user_config_paths_prefer_xdg_and_require_home() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!("gotify-migrate-home-{}", std::process::id()));
        let xdg = home.join("xdg");
        let previous_home = std::env::var_os("HOME");
        let previous_config_home = std::env::var_os("XDG_CONFIG_HOME");

        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("XDG_CONFIG_HOME", &xdg);
        }
        let paths = gotify_user_config_paths().unwrap();
        assert_eq!(paths[0], xdg.join("gotify/cli.json"));
        assert_eq!(paths[1], home.join(".gotify/cli.json"));

        unsafe {
            std::env::remove_var("HOME");
            std::env::remove_var("XDG_CONFIG_HOME");
        }
        assert_eq!(gotify_user_config_paths().unwrap_err(), "HOME is not set");

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_config_home {
                Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
        }
    }

    #[test]
    fn json_helpers_cover_invalid_shapes_and_rewrite_fallbacks() {
        assert_eq!(json_string_value(r#"{"token":1}"#, "token"), None);
        assert_eq!(
            json_string_value(r#"{"token":"unterminated}"#, "token"),
            None
        );
        assert_eq!(
            rewrite_json_string_key(r#"{"other":"value"}"#, "token", ""),
            r#"{"other":"value"}"#
        );
    }

    #[test]
    fn test_build_keychain_store_secret_is_stubbed() {
        assert_eq!(
            keychain_store_secret(KEYCHAIN_SERVICE, GOTIFY_TOKEN_ENV_KEY, "value").unwrap_err(),
            "Automic Vault secret storage is only available on macOS"
        );
    }
}
