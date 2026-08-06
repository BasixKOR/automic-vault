#[cfg(all(target_os = "macos", not(test), not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const GPTCOMMIT_OPENAI_API_KEY_ENV_KEY: &str = "GPTCOMMIT__OPENAI__API_KEY";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[GPTCOMMIT_OPENAI_API_KEY_ENV_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_config_file(&gptcommit_config_path()?, &KeychainCredentialStore).map(|_| ())
}

pub fn migrate_config_file(path: &Path, store: &dyn CredentialStore) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    let Some(api_key) = gptcommit_api_key(&contents).filter(|value| !value.is_empty()) else {
        return Ok(false);
    };

    store.store_secret(GPTCOMMIT_OPENAI_API_KEY_ENV_KEY, api_key)?;
    fs::write(path, scrub_gptcommit_api_key(&contents))
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn gptcommit_config_path() -> Result<PathBuf, String> {
    Ok(user_home()?
        .join(".config")
        .join("gptcommit")
        .join("config.toml"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn gptcommit_api_key(contents: &str) -> Option<&str> {
    toml_string_value_for_key(contents, "openai", "api_key")
        .or_else(|| toml_string_value_for_dotted_key(contents, "openai.api_key"))
}

fn toml_string_value_for_key<'a>(contents: &'a str, section: &str, key: &str) -> Option<&'a str> {
    let mut in_section = false;
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(name) = table_name(trimmed) {
            in_section = name == section;
            continue;
        }
        if !in_section {
            continue;
        }
        let (line_key, value) = trimmed.split_once('=')?;
        if line_key.trim() == key {
            return toml_string_value(value);
        }
    }
    None
}

fn toml_string_value_for_dotted_key<'a>(contents: &'a str, key: &str) -> Option<&'a str> {
    contents.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }
        let (line_key, value) = trimmed.split_once('=')?;
        (line_key.trim() == key)
            .then(|| toml_string_value(value))
            .flatten()
    })
}

fn scrub_gptcommit_api_key(contents: &str) -> String {
    let mut output = String::new();
    let mut in_openai_section = false;
    for line in contents.lines() {
        let trimmed = line.trim();
        if let Some(name) = table_name(trimmed) {
            in_openai_section = name == "openai";
        }
        let should_remove = !trimmed.starts_with('#')
            && trimmed.split_once('=').is_some_and(|(key, _)| {
                key.trim() == "openai.api_key" || (in_openai_section && key.trim() == "api_key")
            });
        if should_remove {
            continue;
        }
        output.push_str(line);
        output.push('\n');
    }
    if !contents.ends_with('\n') {
        output.pop();
    }
    output
}

fn table_name(line: &str) -> Option<&str> {
    line.strip_prefix('[')
        .and_then(|line| line.strip_suffix(']'))
        .map(str::trim)
}

fn toml_string_value(value: &str) -> Option<&str> {
    let value = value.trim();
    value
        .strip_prefix('"')
        .and_then(|value| value.split_once('"').map(|(value, _)| value))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.split_once('\'').map(|(value, _)| value))
        })
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

    struct FailingCredentialStore;

    impl CredentialStore for FailingCredentialStore {
        fn store_secret(&self, _key: &str, _value: &str) -> Result<(), String> {
            Err("store failed".to_string())
        }
    }

    #[test]
    fn migrates_api_key_and_preserves_other_config() {
        let temp = test_dir("gptcommit-migrate");
        let path = temp.join("config.toml");
        let contents = "model_provider = \"openai\"\n[openai]\napi_base = \"https://api.openai.com/v1\"\napi_key = \"fake-openai-key\"\nmodel = \"gpt-4.1-nano\"\n";
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_config_file(&path, &store).unwrap());

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                GPTCOMMIT_OPENAI_API_KEY_ENV_KEY.to_string(),
                "fake-openai-key".to_string()
            )]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "model_provider = \"openai\"\n[openai]\napi_base = \"https://api.openai.com/v1\"\nmodel = \"gpt-4.1-nano\"\n"
        );
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn migrates_dotted_api_key() {
        let temp = test_dir("gptcommit-dotted-migrate");
        let path = temp.join("config.toml");
        fs::write(
            &path,
            "openai.api_key = 'fake-openai-key'\nallow_amend = true\n",
        )
        .unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_config_file(&path, &store).unwrap());

        assert_eq!(fs::read_to_string(&path).unwrap(), "allow_amend = true\n");
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn skips_missing_or_empty_api_key() {
        let missing =
            std::env::temp_dir().join(format!("missing-gptcommit-{}", std::process::id()));
        let temp = test_dir("gptcommit-empty");
        let empty = temp.join("config.toml");
        fs::write(&empty, "[openai]\napi_key = \"\"\n").unwrap();
        let store = TestCredentialStore::default();

        assert!(!migrate_config_file(&missing, &store).unwrap());
        assert!(!migrate_config_file(&empty, &store).unwrap());
        assert!(store.values.borrow().is_empty());
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn does_not_scrub_when_store_fails() {
        let temp = test_dir("gptcommit-fail");
        let path = temp.join("config.toml");
        let contents = "[openai]\napi_key = \"fake-openai-key\"\n";
        fs::write(&path, contents).unwrap();

        assert_eq!(
            migrate_config_file(&path, &FailingCredentialStore).unwrap_err(),
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
