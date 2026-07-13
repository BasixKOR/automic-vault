#[cfg(all(target_os = "macos", not(test), not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const CENSYS_API_ID_ENV_KEY: &str = "CENSYS_API_ID";
const CENSYS_API_SECRET_ENV_KEY: &str = "CENSYS_API_SECRET";
const CENSYS_ASM_API_KEY_ENV_KEY: &str = "CENSYS_ASM_API_KEY";
const SECRET_KEYS: &[(&str, &str)] = &[
    ("api_id", CENSYS_API_ID_ENV_KEY),
    ("api_secret", CENSYS_API_SECRET_ENV_KEY),
    ("asm_api_key", CENSYS_ASM_API_KEY_ENV_KEY),
];

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[
        CENSYS_API_ID_ENV_KEY,
        CENSYS_API_SECRET_ENV_KEY,
        CENSYS_ASM_API_KEY_ENV_KEY,
    ]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_credentials_file(&censys_config_path()?, &KeychainCredentialStore).map(|_| ())
}

pub fn migrate_credentials_file(path: &Path, store: &dyn CredentialStore) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };

    let credentials = config_credentials(&contents);
    if credentials.is_empty() {
        return Ok(false);
    }

    for env_key in keys() {
        let value = credentials
            .iter()
            .find(|credential| credential.env_key == *env_key)
            .map(|credential| credential.value.as_str())
            .unwrap_or("");
        store.store_secret(env_key, value)?;
    }
    fs::write(path, remove_secret_lines(&contents))
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn censys_config_path() -> Result<PathBuf, String> {
    let home = user_home()?;
    Ok(home.join(".config/censys/censys.cfg"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

struct Credential {
    env_key: &'static str,
    value: String,
}

fn config_credentials(contents: &str) -> Vec<Credential> {
    contents.lines().filter_map(line_credential).collect()
}

fn line_credential(line: &str) -> Option<Credential> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
        return None;
    }
    let (key, value) = line.split_once('=')?;
    let env_key = SECRET_KEYS
        .iter()
        .find(|(secret_key, _)| key.trim() == *secret_key)?
        .1;
    let value = ini_value(value);
    if value.is_empty() {
        return None;
    }
    Some(Credential {
        env_key,
        value: value.to_string(),
    })
}

fn line_has_secret(line: &str) -> bool {
    line_credential(line).is_some()
}

fn ini_value(value: &str) -> &str {
    value.trim().trim_matches('"').trim_matches('\'').trim()
}

fn remove_secret_lines(contents: &str) -> String {
    let mut output = String::new();
    for line in contents.lines() {
        if line_has_secret(line) {
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
    fn migrates_censys_credentials() {
        let path = std::env::temp_dir().join(format!("censys-config-{}", std::process::id()));
        let contents = concat!(
            "[DEFAULT]\n",
            "api_id = fake-censys-id\n",
            "api_secret = fake-censys-secret\n",
            "asm_api_key = fake-censys-asm-key\n",
            "color = auto\n",
        );
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_credentials_file(&path, &store).unwrap());

        assert_eq!(
            store.values.borrow().as_slice(),
            &[
                (
                    CENSYS_API_ID_ENV_KEY.to_string(),
                    "fake-censys-id".to_string()
                ),
                (
                    CENSYS_API_SECRET_ENV_KEY.to_string(),
                    "fake-censys-secret".to_string()
                ),
                (
                    CENSYS_ASM_API_KEY_ENV_KEY.to_string(),
                    "fake-censys-asm-key".to_string()
                ),
            ]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "[DEFAULT]\ncolor = auto\n"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn stores_empty_counterparts_for_partial_credentials() {
        let path =
            std::env::temp_dir().join(format!("censys-partial-config-{}", std::process::id()));
        let contents = "[DEFAULT]\nasm_api_key = fake-censys-asm-key\n";
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_credentials_file(&path, &store).unwrap());

        assert_eq!(
            store.values.borrow().as_slice(),
            &[
                (CENSYS_API_ID_ENV_KEY.to_string(), String::new()),
                (CENSYS_API_SECRET_ENV_KEY.to_string(), String::new()),
                (
                    CENSYS_ASM_API_KEY_ENV_KEY.to_string(),
                    "fake-censys-asm-key".to_string()
                ),
            ]
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), "[DEFAULT]\n");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn does_not_migrate_without_credentials() {
        let path = std::env::temp_dir().join(format!("censys-no-secret-{}", std::process::id()));
        fs::write(&path, "[DEFAULT]\ncolor = auto\n").unwrap();
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
