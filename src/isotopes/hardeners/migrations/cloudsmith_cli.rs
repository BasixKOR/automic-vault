#[cfg(all(target_os = "macos", not(test), not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const CLOUDSMITH_API_KEY_ENV_KEY: &str = "CLOUDSMITH_API_KEY";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[CLOUDSMITH_API_KEY_ENV_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    let store = KeychainCredentialStore;
    for path in cloudsmith_credentials_paths()? {
        if migrate_credentials_file(&path, &store)? {
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

    let api_keys = credentials_api_keys(&contents);
    if api_keys.is_empty() {
        return Ok(false);
    }
    if api_keys.len() > 1 {
        return Err("multiple Cloudsmith API keys found; migrate them manually".to_string());
    }

    store.store_secret(CLOUDSMITH_API_KEY_ENV_KEY, &api_keys[0])?;
    fs::write(path, remove_api_key_lines(&contents))
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn cloudsmith_credentials_paths() -> Result<Vec<PathBuf>, String> {
    let home = user_home()?;
    Ok(vec![
        home.join("Library/Application Support/cloudsmith/credentials.ini"),
        home.join(".cloudsmith/credentials.ini"),
    ])
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn credentials_api_keys(contents: &str) -> Vec<String> {
    contents.lines().filter_map(line_api_key).collect()
}

fn line_api_key(line: &str) -> Option<String> {
    let line = line.split(['#', ';']).next().unwrap_or("").trim();
    let (name, value) = line.split_once('=')?;
    if name.trim() != "api_key" {
        return None;
    }
    let value = value.trim().trim_matches('"').trim_matches('\'').trim();
    if value.is_empty() {
        return None;
    }
    Some(value.to_string())
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
    fn migrates_single_api_key() {
        let path =
            std::env::temp_dir().join(format!("cloudsmith-credentials-{}", std::process::id()));
        let contents = "[default]\napi_key=fake-cloudsmith-key\napi_host=api.cloudsmith.io\n";
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_credentials_file(&path, &store).unwrap());

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                CLOUDSMITH_API_KEY_ENV_KEY.to_string(),
                "fake-cloudsmith-key".to_string()
            )]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "[default]\napi_host=api.cloudsmith.io\n"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn refuses_multiple_api_keys() {
        let path = std::env::temp_dir().join(format!("cloudsmith-multiple-{}", std::process::id()));
        let contents = "[default]\napi_key=one\n[profile:prod]\napi_key=two\n";
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert_eq!(
            migrate_credentials_file(&path, &store).unwrap_err(),
            "multiple Cloudsmith API keys found; migrate them manually"
        );
        assert!(store.values.borrow().is_empty());
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn does_not_migrate_without_api_key() {
        let path =
            std::env::temp_dir().join(format!("cloudsmith-no-credentials-{}", std::process::id()));
        fs::write(&path, "[default]\napi_key=\n").unwrap();
        let store = TestCredentialStore::default();

        assert!(!migrate_credentials_file(&path, &store).unwrap());
        assert!(store.values.borrow().is_empty());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn path_and_keychain_helpers_cover_defaults_and_missing_home() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = test_dir("cloudsmith-home");
        {
            let _env = EnvGuard::set_home(Some(&home));
            assert_eq!(
                cloudsmith_credentials_paths().unwrap(),
                vec![
                    home.join("Library/Application Support/cloudsmith/credentials.ini"),
                    home.join(".cloudsmith/credentials.ini")
                ]
            );
            assert_eq!(user_home().unwrap(), home);
            assert_eq!(keys(), &[CLOUDSMITH_API_KEY_ENV_KEY]);
            migrate_credentials().unwrap();
        }
        {
            let _env = EnvGuard::set_home(None);
            assert_eq!(user_home().unwrap_err(), "HOME is not set");
            assert_eq!(
                cloudsmith_credentials_paths().unwrap_err(),
                "HOME is not set"
            );
        }
        assert!(
            KeychainCredentialStore
                .store_secret(CLOUDSMITH_API_KEY_ENV_KEY, "secret")
                .unwrap_err()
                .contains("keychain integration")
        );
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn parser_helpers_cover_quotes_comments_empty_values_and_removal() {
        assert_eq!(
            line_api_key("api_key = ' fake-key ' # comment").as_deref(),
            Some("fake-key")
        );
        assert_eq!(
            line_api_key("api_key = \"fake-key\" ; comment").as_deref(),
            Some("fake-key")
        );
        assert_eq!(line_api_key("api_key = "), None);
        assert_eq!(line_api_key("token = fake-key"), None);
        assert_eq!(
            credentials_api_keys("[default]\napi_key=one\n[profile:prod]\napi_key = two\n"),
            vec!["one".to_string(), "two".to_string()]
        );
        assert_eq!(
            remove_api_key_lines("[default]\napi_key=secret\napi_host=api.cloudsmith.io\n"),
            "[default]\napi_host=api.cloudsmith.io\n"
        );
    }

    #[test]
    fn reports_read_errors_and_preserves_file_on_store_failure() {
        let temp = test_dir("cloudsmith-errors");
        let dir_path = temp.join("dir");
        fs::create_dir_all(&dir_path).unwrap();
        assert!(
            migrate_credentials_file(&dir_path, &TestCredentialStore::default())
                .unwrap_err()
                .contains("failed to read")
        );

        let path = temp.join("credentials.ini");
        let contents = "[default]\napi_key=secret\n";
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
