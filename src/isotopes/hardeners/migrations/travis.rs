#[cfg(all(target_os = "macos", not(test), not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const TRAVIS_TOKEN_ENV_KEY: &str = "TRAVIS_TOKEN";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[TRAVIS_TOKEN_ENV_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_credentials_file(&travis_config_path()?, &KeychainCredentialStore).map(|_| ())
}

pub fn migrate_credentials_file(path: &Path, store: &dyn CredentialStore) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };

    let tokens = config_access_tokens(&contents);
    if tokens.is_empty() {
        return Ok(false);
    }
    if tokens.len() > 1 {
        return Err("multiple Travis access tokens found; migrate them manually".to_string());
    }

    store.store_secret(TRAVIS_TOKEN_ENV_KEY, &tokens[0])?;
    fs::write(path, remove_access_token_lines(&contents))
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn travis_config_path() -> Result<PathBuf, String> {
    let home = user_home()?;
    Ok(home.join(".travis/config.yml"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn config_contains_access_token(contents: &str) -> bool {
    !config_access_tokens(contents).is_empty()
}

fn config_access_tokens(contents: &str) -> Vec<String> {
    contents.lines().filter_map(line_access_token).collect()
}

fn line_access_token(line: &str) -> Option<String> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (key, value) = line.split_once(':')?;
    if key.trim() != "access_token" {
        return None;
    }
    let value = yaml_scalar_value(value)?;
    if value.is_empty() {
        return None;
    }
    Some(value.to_string())
}

fn line_has_access_token(line: &str) -> bool {
    line_access_token(line).is_some()
}

fn yaml_scalar_value(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    Some(value.trim_matches('"').trim_matches('\'').trim())
}

fn remove_access_token_lines(contents: &str) -> String {
    let mut output = String::new();
    for line in contents.lines() {
        if line_has_access_token(line) {
            continue;
        }
        output.push_str(line);
        output.push('\n');
    }
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
    fn migrates_single_travis_token() {
        let path = std::env::temp_dir().join(format!("travis-config-{}", std::process::id()));
        let contents = concat!(
            "endpoints:\n",
            "  https://api.travis-ci.com/:\n",
            "    access_token: fake-travis-token\n",
            "    insecure: false\n",
        );
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_credentials_file(&path, &store).unwrap());

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                TRAVIS_TOKEN_ENV_KEY.to_string(),
                "fake-travis-token".to_string()
            )]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "endpoints:\n  https://api.travis-ci.com/:\n    insecure: false\n"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn refuses_multiple_travis_tokens() {
        let path =
            std::env::temp_dir().join(format!("travis-multiple-tokens-{}", std::process::id()));
        let contents = concat!(
            "endpoints:\n",
            "  https://api.travis-ci.com/:\n",
            "    access_token: one\n",
            "  https://api.travis-ci.org/:\n",
            "    access_token: two\n",
        );
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert_eq!(
            migrate_credentials_file(&path, &store).unwrap_err(),
            "multiple Travis access tokens found; migrate them manually"
        );
        assert!(store.values.borrow().is_empty());
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn does_not_migrate_without_access_token() {
        let path = std::env::temp_dir().join(format!("travis-no-token-{}", std::process::id()));
        fs::write(&path, "endpoints: {}\n").unwrap();
        let store = TestCredentialStore::default();

        assert!(!migrate_credentials_file(&path, &store).unwrap());
        assert!(store.values.borrow().is_empty());
        fs::remove_file(path).unwrap();
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
