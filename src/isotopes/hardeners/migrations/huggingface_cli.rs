#[cfg(all(target_os = "macos", not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const HF_TOKEN_ENV_KEY: &str = "HF_TOKEN";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[HF_TOKEN_ENV_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_credentials_file(&huggingface_token_path()?, &KeychainCredentialStore).map(|_| ())
}

pub fn migrate_credentials_file(path: &Path, store: &dyn CredentialStore) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    let token = contents.trim();
    if token.is_empty() {
        return Ok(false);
    }

    store.store_secret(HF_TOKEN_ENV_KEY, token)?;
    fs::write(path, "").map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn huggingface_token_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".cache/huggingface/token"))
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

    struct FailingCredentialStore;

    impl CredentialStore for FailingCredentialStore {
        fn store_secret(&self, _key: &str, _value: &str) -> Result<(), String> {
            Err("store failed".to_string())
        }
    }

    #[test]
    fn migrates_huggingface_token() {
        let path = std::env::temp_dir().join(format!("hf-token-{}", std::process::id()));
        fs::write(&path, "hf_secret\n").unwrap();
        let store = TestCredentialStore::default();

        migrate_credentials_file(&path, &store).unwrap();

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(HF_TOKEN_ENV_KEY.to_string(), "hf_secret".to_string())]
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), "");
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
    fn keys_and_default_path_require_home() {
        assert_eq!(keys(), &[HF_TOKEN_ENV_KEY]);

        let previous_home = std::env::var_os("HOME");
        unsafe { std::env::remove_var("HOME") };
        let err = huggingface_token_path().unwrap_err();
        assert!(err.contains("HOME is not set"));
        match previous_home {
            Some(value) => unsafe { std::env::set_var("HOME", value) },
            None => unsafe { std::env::remove_var("HOME") },
        }
    }

    #[test]
    fn migrate_credentials_ignores_missing_and_blank_token_files() {
        let temp = std::env::temp_dir();
        let missing = temp.join(format!("hf-missing-{}", std::process::id()));
        let blank = temp.join(format!("hf-blank-{}", std::process::id()));
        let store = TestCredentialStore::default();

        assert!(!migrate_credentials_file(&missing, &store).unwrap());
        fs::write(&blank, " \n").unwrap();
        assert!(!migrate_credentials_file(&blank, &store).unwrap());
        assert!(store.values.borrow().is_empty());
        fs::remove_file(blank).unwrap();
    }

    #[test]
    fn migrate_credentials_preserves_file_when_store_fails() {
        let path = std::env::temp_dir().join(format!("hf-store-failure-{}", std::process::id()));
        fs::write(&path, "hf_secret\n").unwrap();

        let err = migrate_credentials_file(&path, &FailingCredentialStore).unwrap_err();

        assert!(err.contains("store failed"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "hf_secret\n");
        fs::remove_file(path).unwrap();
    }
}
