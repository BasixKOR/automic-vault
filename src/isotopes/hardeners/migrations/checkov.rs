#[cfg(all(target_os = "macos", not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::PathBuf;

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const BC_API_KEY_ENV_KEY: &str = "BC_API_KEY";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[BC_API_KEY_ENV_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_credentials_path(&bridgecrew_credentials_path()?, &KeychainCredentialStore).map(|_| ())
}

pub fn migrate_credentials_path(
    path: &PathBuf,
    store: &dyn CredentialStore,
) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    let token = contents.trim();
    if token.is_empty() {
        return Ok(false);
    }

    store.store_secret(BC_API_KEY_ENV_KEY, token)?;
    fs::write(path, "").map_err(|err| format!("failed to clear {}: {err}", path.display()))?;
    Ok(true)
}

fn bridgecrew_credentials_path() -> Result<PathBuf, String> {
    Ok(user_home()?.join(".bridgecrew/credentials"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
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

    struct FailingCredentialStore;

    impl CredentialStore for FailingCredentialStore {
        fn store_secret(&self, _key: &str, _value: &str) -> Result<(), String> {
            Err("store failed".to_string())
        }
    }

    #[test]
    fn migrates_credentials_file_to_env_key() {
        let temp = test_dir("checkov-migrate");
        let credentials = temp.join("credentials");
        fs::write(&credentials, "access_key::secret_key\n").unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_credentials_path(&credentials, &store).unwrap());

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                BC_API_KEY_ENV_KEY.to_string(),
                "access_key::secret_key".to_string()
            )]
        );
        assert_eq!(fs::read_to_string(&credentials).unwrap(), "");
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn skips_empty_credentials_file() {
        let temp = test_dir("checkov-clean");
        let credentials = temp.join("credentials");
        fs::write(&credentials, "\n").unwrap();
        let store = TestCredentialStore::default();

        assert!(!migrate_credentials_path(&credentials, &store).unwrap());
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

    #[test]
    fn keys_and_home_helpers_cover_edge_cases() {
        assert_eq!(keys(), &[BC_API_KEY_ENV_KEY]);

        let previous_home = std::env::var_os("HOME");
        unsafe { std::env::remove_var("HOME") };
        let err = user_home().unwrap_err();
        assert!(err.contains("HOME is not set"));
        let err = bridgecrew_credentials_path().unwrap_err();
        assert!(err.contains("HOME is not set"));
        match previous_home {
            Some(value) => unsafe { std::env::set_var("HOME", value) },
            None => unsafe { std::env::remove_var("HOME") },
        }
    }

    #[test]
    fn migrate_credentials_ignores_missing_and_preserves_file_on_store_failure() {
        let temp = test_dir("checkov-store-failure");
        let missing = temp.join("missing");
        let credentials = temp.join("credentials");
        fs::write(&credentials, "access_key::secret_key\n").unwrap();
        let store = TestCredentialStore::default();

        assert!(!migrate_credentials_path(&missing, &store).unwrap());
        let err = migrate_credentials_path(&credentials, &FailingCredentialStore).unwrap_err();
        assert!(err.contains("store failed"));
        assert_eq!(
            fs::read_to_string(&credentials).unwrap(),
            "access_key::secret_key\n"
        );
        fs::remove_dir_all(temp).unwrap();
    }
}
