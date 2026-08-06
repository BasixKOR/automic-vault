#[cfg(all(target_os = "macos", not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const HCLOUD_TOKEN_ENV_KEY: &str = "HCLOUD_TOKEN";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[HCLOUD_TOKEN_ENV_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_credentials_file(&hcloud_config_path()?, &KeychainCredentialStore).map(|_| ())
}

pub fn migrate_credentials_file(path: &Path, store: &dyn CredentialStore) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };

    let Some(token) = migratable_token(&contents)? else {
        return Ok(false);
    };

    store.store_secret(HCLOUD_TOKEN_ENV_KEY, &token)?;
    fs::write(path, sanitized_config_toml(&contents))
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn hcloud_config_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("HCLOUD_CONFIG").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }

    let config_home = if let Some(path) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(path)
    } else {
        user_home()?.join(".config")
    };
    Ok(config_home.join("hcloud/cli.toml"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn config_contains_token(contents: &str) -> bool {
    contents.lines().any(line_has_token)
}

fn migratable_token(contents: &str) -> Result<Option<String>, String> {
    let mut tokens = Vec::new();
    for token in token_values(contents) {
        if !tokens.contains(&token) {
            tokens.push(token);
        }
    }

    match tokens.len() {
        0 => Ok(None),
        1 => Ok(tokens.pop()),
        _ => Err("multiple hcloud context tokens found; migrate them manually".to_string()),
    }
}

fn token_values(contents: &str) -> Vec<String> {
    contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            if key.trim() != "token" {
                return None;
            }
            let token = toml_string_value(value).unwrap_or_default();
            (!token.is_empty()).then(|| token.to_string())
        })
        .collect()
}

fn sanitized_config_toml(contents: &str) -> String {
    let mut sanitized = String::new();
    for line in contents.split_inclusive('\n') {
        sanitized.push_str(&sanitized_line(line));
    }
    if !contents.ends_with('\n') {
        let Some(last_line) = contents.lines().next_back() else {
            return sanitized;
        };
        if contents == last_line {
            return sanitized_line(last_line);
        }
    }
    sanitized
}

fn sanitized_line(line: &str) -> String {
    let (body, ending) = if let Some(body) = line.strip_suffix("\r\n") {
        (body, "\r\n")
    } else if let Some(body) = line.strip_suffix('\n') {
        (body, "\n")
    } else {
        (line, "")
    };

    let Some((key, value)) = body.split_once('=') else {
        return line.to_string();
    };
    if key.trim() != "token" || toml_string_value(value).unwrap_or_default().is_empty() {
        return line.to_string();
    }

    let leading = key
        .chars()
        .take_while(|character| character.is_whitespace())
        .collect::<String>();
    format!("{leading}token = \"\"{ending}")
}

fn line_has_token(line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return false;
    }
    let Some((key, value)) = line.split_once('=') else {
        return false;
    };
    key.trim() == "token" && !toml_string_value(value).unwrap_or_default().is_empty()
}

fn toml_string_value(value: &str) -> Option<&str> {
    value
        .trim()
        .strip_prefix('"')
        .and_then(|value| value.split_once('"').map(|(value, _)| value))
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

    #[test]
    fn migrates_hcloud_config() {
        let path = std::env::temp_dir().join(format!("hcloud-config-{}", std::process::id()));
        let contents = "active_context = \"prod\"\n[[contexts]]\ntoken = \"hcloud-token\"\n";
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_credentials_file(&path, &store).unwrap());

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(HCLOUD_TOKEN_ENV_KEY.to_string(), "hcloud-token".to_string())]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "active_context = \"prod\"\n[[contexts]]\ntoken = \"\"\n"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn migrates_duplicate_token_values() {
        let path = std::env::temp_dir().join(format!("hcloud-duplicate-{}", std::process::id()));
        let contents =
            "[[contexts]]\ntoken = \"same-token\"\n[[contexts]]\ntoken = \"same-token\"\n";
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_credentials_file(&path, &store).unwrap());

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(HCLOUD_TOKEN_ENV_KEY.to_string(), "same-token".to_string())]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "[[contexts]]\ntoken = \"\"\n[[contexts]]\ntoken = \"\"\n"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn refuses_multiple_distinct_context_tokens() {
        let path = std::env::temp_dir().join(format!("hcloud-multi-{}", std::process::id()));
        let contents = "[[contexts]]\ntoken = \"one\"\n[[contexts]]\ntoken = \"two\"\n";
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert_eq!(
            migrate_credentials_file(&path, &store).unwrap_err(),
            "multiple hcloud context tokens found; migrate them manually"
        );
        assert!(store.values.borrow().is_empty());
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn does_not_migrate_without_token() {
        let path = std::env::temp_dir().join(format!("hcloud-no-token-{}", std::process::id()));
        fs::write(&path, "active_context = \"prod\"\n").unwrap();
        let store = TestCredentialStore::default();

        assert!(!migrate_credentials_file(&path, &store).unwrap());
        assert!(store.values.borrow().is_empty());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn helpers_cover_detection_and_sanitizing_edges() {
        assert_eq!(keys(), &[HCLOUD_TOKEN_ENV_KEY]);
        assert!(config_contains_token(
            "[[contexts]]\ntoken = \"hcloud-token\"\n"
        ));
        assert!(!config_contains_token("[[contexts]]\ntoken = \"\"\n"));
        assert_eq!(
            sanitized_config_toml("  token = \"hcloud-token\" # comment\r\nname = \"prod\"\n"),
            "  token = \"\"\r\nname = \"prod\"\n"
        );
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
