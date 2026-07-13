#[cfg(all(target_os = "macos", not(test), not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const RUNPOD_API_KEY_ENV_KEY: &str = "RUNPOD_API_KEY";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[RUNPOD_API_KEY_ENV_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_default_files(&KeychainCredentialStore).map(|_| ())
}

pub fn migrate_default_files(store: &dyn CredentialStore) -> Result<bool, String> {
    let home = user_home()?;
    migrate_credentials_files(
        &home.join(".runpod/config.toml"),
        &home.join(".runpod.yaml"),
        store,
    )
}

pub fn migrate_credentials_files(
    toml_path: &Path,
    legacy_yaml_path: &Path,
    store: &dyn CredentialStore,
) -> Result<bool, String> {
    let toml = read_optional_secret_config(toml_path)?;
    let legacy_yaml = read_optional_secret_config(legacy_yaml_path)?;
    if toml.is_none() && legacy_yaml.is_none() {
        return Ok(false);
    }

    let api_keys = [toml.as_ref(), legacy_yaml.as_ref()]
        .into_iter()
        .flatten()
        .map(|config| config.api_key.as_str())
        .collect::<Vec<_>>();
    if api_keys.len() > 1 && api_keys[0] != api_keys[1] {
        return Err("multiple runpodctl API keys found; migrate them manually".to_string());
    }

    store.store_secret(RUNPOD_API_KEY_ENV_KEY, api_keys[0])?;

    if let Some(toml) = toml {
        fs::write(toml_path, toml.sanitized)
            .map_err(|err| format!("failed to write {}: {err}", toml_path.display()))?;
    }
    if let Some(legacy_yaml) = legacy_yaml {
        fs::write(legacy_yaml_path, legacy_yaml.sanitized)
            .map_err(|err| format!("failed to write {}: {err}", legacy_yaml_path.display()))?;
    }
    Ok(true)
}

struct SecretConfig {
    api_key: String,
    sanitized: String,
}

fn read_optional_secret_config(path: &Path) -> Result<Option<SecretConfig>, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    let Some(api_key) = config_api_key(&contents) else {
        return Ok(None);
    };
    Ok(Some(SecretConfig {
        api_key,
        sanitized: remove_api_key_lines(&contents),
    }))
}

fn remove_api_key_lines(contents: &str) -> String {
    let mut output = String::new();
    for line in contents.lines() {
        if line_api_key(line).is_some() {
            continue;
        }
        output.push_str(line);
        output.push('\n');
    }
    output
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn config_api_key(contents: &str) -> Option<String> {
    contents.lines().find_map(line_api_key)
}

fn line_api_key(line: &str) -> Option<String> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (key, value) = line.split_once(['=', ':'])?;
    if !matches!(key.trim(), "apiKey" | "api_key") {
        return None;
    }
    let value = quoted_config_value(value)?;
    if value.is_empty() {
        return None;
    }
    Some(value.to_string())
}

fn quoted_config_value(value: &str) -> Option<&str> {
    let value = value.trim();
    if let Some(value) = value.strip_prefix('"') {
        return value.split_once('"').map(|(value, _)| value);
    }
    if let Some(value) = value.strip_prefix('\'') {
        return value.split_once('\'').map(|(value, _)| value);
    }
    value.split_whitespace().next()
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

    struct FailingCredentialStore;

    impl CredentialStore for FailingCredentialStore {
        fn store_secret(&self, _key: &str, _value: &str) -> Result<(), String> {
            Err("store failed".to_string())
        }
    }

    #[test]
    fn migrates_toml_and_legacy_yaml_configs_with_same_api_key() {
        let temp = test_dir("runpodctl-migrate");
        let toml = temp.join("config.toml");
        let yaml = temp.join(".runpod.yaml");
        let toml_contents =
            "apiKey = \"fake-runpod-key\"\napiUrl = \"https://api.runpod.io/graphql\"\n";
        let yaml_contents = "apiKey: fake-runpod-key\napiUrl: https://api.runpod.io/graphql\n";
        fs::write(&toml, toml_contents).unwrap();
        fs::write(&yaml, yaml_contents).unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_credentials_files(&toml, &yaml, &store).unwrap());

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                RUNPOD_API_KEY_ENV_KEY.to_string(),
                "fake-runpod-key".to_string()
            )]
        );
        assert_eq!(
            fs::read_to_string(&toml).unwrap(),
            "apiUrl = \"https://api.runpod.io/graphql\"\n"
        );
        assert_eq!(
            fs::read_to_string(&yaml).unwrap(),
            "apiUrl: https://api.runpod.io/graphql\n"
        );
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn migrates_single_config() {
        let temp = test_dir("runpodctl-single-migrate");
        let toml = temp.join("config.toml");
        let yaml = temp.join(".runpod.yaml");
        let toml_contents = "apiKey = \"fake-runpod-key\"\n";
        fs::write(&toml, toml_contents).unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_credentials_files(&toml, &yaml, &store).unwrap());

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                RUNPOD_API_KEY_ENV_KEY.to_string(),
                "fake-runpod-key".to_string()
            )]
        );
        assert_eq!(fs::read_to_string(&toml).unwrap(), "");
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn refuses_different_toml_and_legacy_yaml_api_keys() {
        let temp = test_dir("runpodctl-multiple-keys");
        let toml = temp.join("config.toml");
        let yaml = temp.join(".runpod.yaml");
        let toml_contents = "apiKey = \"fake-runpod-key\"\n";
        let yaml_contents = "apiKey: fake-runpod-legacy-key\n";
        fs::write(&toml, toml_contents).unwrap();
        fs::write(&yaml, yaml_contents).unwrap();
        let store = TestCredentialStore::default();

        assert_eq!(
            migrate_credentials_files(&toml, &yaml, &store).unwrap_err(),
            "multiple runpodctl API keys found; migrate them manually"
        );

        assert!(store.values.borrow().is_empty());
        assert_eq!(fs::read_to_string(&toml).unwrap(), toml_contents);
        assert_eq!(fs::read_to_string(&yaml).unwrap(), yaml_contents);
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn does_not_migrate_without_api_key() {
        let temp = test_dir("runpodctl-no-migrate");
        let toml = temp.join("config.toml");
        let yaml = temp.join(".runpod.yaml");
        fs::write(&toml, "apiUrl = \"https://api.runpod.io/graphql\"\n").unwrap();
        let store = TestCredentialStore::default();

        assert!(!migrate_credentials_files(&toml, &yaml, &store).unwrap());
        assert!(store.values.borrow().is_empty());
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn preserves_files_on_store_failure() {
        let temp = test_dir("runpodctl-store-failure");
        let toml = temp.join("config.toml");
        let yaml = temp.join(".runpod.yaml");
        let toml_contents = "apiKey = \"fake-runpod-key\"\n";
        fs::write(&toml, toml_contents).unwrap();

        let err = migrate_credentials_files(&toml, &yaml, &FailingCredentialStore).unwrap_err();

        assert!(err.contains("store failed"));
        assert_eq!(fs::read_to_string(&toml).unwrap(), toml_contents);
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn top_level_migrate_credentials_ignores_missing_default_locations() {
        let home = test_dir("runpodctl-migrate-missing");
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
        fs::remove_dir_all(home).unwrap();
    }

    fn test_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "{}-{}-{}",
            name,
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }
}
