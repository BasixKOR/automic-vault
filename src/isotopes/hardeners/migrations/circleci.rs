#[cfg(all(target_os = "macos", not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::PathBuf;

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const CIRCLECI_TOKEN_ENV_KEY: &str = "CIRCLECI_CLI_TOKEN";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[CIRCLECI_TOKEN_ENV_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_circleci_config(&circleci_config_path()?, &KeychainCredentialStore).map(|_| ())
}

pub fn migrate_circleci_config(
    path: &std::path::Path,
    store: &dyn CredentialStore,
) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };

    let Some(token) = circleci_config_token(&contents) else {
        return Ok(false);
    };

    store.store_secret(CIRCLECI_TOKEN_ENV_KEY, &token)?;
    fs::write(path, scrub_circleci_token(&contents))
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn circleci_config_path() -> Result<PathBuf, String> {
    Ok(user_home()?.join(".circleci/cli.yml"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn circleci_config_has_token(contents: &str) -> bool {
    circleci_config_token(contents).is_some()
}

fn circleci_config_token(contents: &str) -> Option<String> {
    contents.lines().find_map(token_from_line)
}

fn line_has_token(line: &str) -> bool {
    token_from_line(line).is_some()
}

fn token_from_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let (key, value) = trimmed.split_once(':')?;
    let value = value.trim().trim_matches('"').trim_matches('\'');
    (key.trim() == "token"
        && !value.is_empty()
        && !value.eq_ignore_ascii_case("null")
        && value != "token")
        .then(|| value.to_string())
}

fn scrub_circleci_token(contents: &str) -> String {
    let mut output = String::new();
    for line in contents.lines() {
        if line_has_token(line) {
            let indent_len = line.len() - line.trim_start().len();
            output.push_str(&" ".repeat(indent_len));
            output.push_str("token: \"\"\n");
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }
    output
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

    #[test]
    fn migrates_and_scrubs_circleci_token() {
        let temp = test_dir("circleci-migrate");
        let path = temp.join("cli.yml");
        fs::write(&path, "host: https://circleci.com\ntoken: abc123\n").unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_circleci_config(&path, &store).unwrap());

        let values = store.values.borrow();
        assert_eq!(
            values.as_slice(),
            &[(CIRCLECI_TOKEN_ENV_KEY.to_string(), "abc123".to_string())]
        );
        assert_eq!(
            fs::read_to_string(path).unwrap(),
            "host: https://circleci.com\ntoken: \"\"\n"
        );
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn skips_config_without_token() {
        let temp = test_dir("circleci-skip");
        let path = temp.join("cli.yml");
        fs::write(&path, "host: https://circleci.com\n").unwrap();
        let store = TestCredentialStore::default();

        assert!(!migrate_circleci_config(&path, &store).unwrap());
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
}
