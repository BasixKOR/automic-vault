#[cfg(all(target_os = "macos", not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const ARGOCD_AUTH_TOKEN_ENV_KEY: &str = "ARGOCD_AUTH_TOKEN";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[ARGOCD_AUTH_TOKEN_ENV_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    let paths = candidate_config_paths()?;
    for path in &paths {
        if migrate_credentials_file(path, &KeychainCredentialStore)? {
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
    let Some(token) = migratable_auth_token(&contents)? else {
        return Ok(false);
    };

    store.store_secret(ARGOCD_AUTH_TOKEN_ENV_KEY, &token)?;
    fs::write(path, sanitized_argocd_config(&contents))
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn candidate_config_paths() -> Result<Vec<PathBuf>, String> {
    if let Some(dir) = std::env::var_os("ARGOCD_CONFIG_DIR").filter(|value| !value.is_empty()) {
        return Ok(vec![PathBuf::from(dir).join("config")]);
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    let mut paths = vec![home.join(".argocd/config")];
    if let Some(config) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        paths.push(PathBuf::from(config).join("argocd/config"));
    }
    paths.push(home.join(".config/argocd/config"));
    Ok(paths)
}

fn migratable_auth_token(contents: &str) -> Result<Option<String>, String> {
    if config_contains_refresh_token(contents) {
        return Err(
            "Argo CD refresh-token configs cannot be represented with ARGOCD_AUTH_TOKEN"
                .to_string(),
        );
    }

    let mut tokens = Vec::new();
    for token in auth_tokens(contents) {
        if !tokens.iter().any(|existing| existing == &token) {
            tokens.push(token);
        }
    }

    match tokens.len() {
        0 => Ok(None),
        1 => {
            let token = tokens.pop().expect("one token");
            reject_env_line_breaks(&token)?;
            Ok(Some(token))
        }
        _ => Err("Argo CD configs with multiple auth tokens must be migrated manually".to_string()),
    }
}

fn config_contains_refresh_token(contents: &str) -> bool {
    contents.lines().any(|line| {
        let trimmed = line.trim_start().trim_start_matches("- ");
        line_has_non_empty_value(trimmed, "refresh-token:")
    })
}

fn auth_tokens(contents: &str) -> Vec<String> {
    contents
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start().trim_start_matches("- ");
            yaml_value(trimmed, "auth-token:")
        })
        .collect()
}

fn yaml_value(line: &str, prefix: &str) -> Option<String> {
    line.strip_prefix(prefix)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "\"\"" && *value != "''")
        .map(unquote_yaml_scalar)
        .map(str::to_string)
}

fn sanitized_argocd_config(contents: &str) -> String {
    let mut output = Vec::new();
    for line in contents.lines() {
        let trimmed = line.trim_start();
        let list_marker = trimmed.starts_with("- ");
        let logical = trimmed.trim_start_matches("- ");
        if line_has_non_empty_value(logical, "auth-token:") {
            let indent = line.len() - trimmed.len();
            let marker = if list_marker { "- " } else { "" };
            output.push(format!("{}{}auth-token: \"\"", " ".repeat(indent), marker));
        } else {
            output.push(line.to_string());
        }
    }

    let mut rendered = output.join("\n");
    if contents.ends_with('\n') {
        rendered.push('\n');
    }
    rendered
}

fn unquote_yaml_scalar(value: &str) -> &str {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
        {
            return &value[1..value.len() - 1];
        }
    }
    value
}

fn reject_env_line_breaks(value: &str) -> Result<(), String> {
    if value.contains('\n') || value.contains('\r') {
        return Err("ARGOCD_AUTH_TOKEN cannot contain line breaks".to_string());
    }
    Ok(())
}

fn line_has_non_empty_value(line: &str, prefix: &str) -> bool {
    line.strip_prefix(prefix)
        .map(str::trim)
        .is_some_and(|value| !value.is_empty() && value != "\"\"" && value != "''")
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
    fn migrates_argocd_config() {
        let path = std::env::temp_dir().join(format!("argocd-config-{}", std::process::id()));
        let contents = "contexts:\n- name: prod\n  server: https://argocd.example.com\n  user: prod\nusers:\n- name: prod\n  auth-token: token\n";
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        migrate_credentials_file(&path, &store).unwrap();

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(ARGOCD_AUTH_TOKEN_ENV_KEY.to_string(), "token".to_string())]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "contexts:\n- name: prod\n  server: https://argocd.example.com\n  user: prod\nusers:\n- name: prod\n  auth-token: \"\"\n"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_refresh_token_configs() {
        let err =
            migratable_auth_token("users:\n- name: prod\n  refresh-token: refresh\n").unwrap_err();

        assert!(err.contains("refresh-token"));
    }

    #[test]
    fn rejects_multiple_auth_tokens() {
        let contents =
            "users:\n- name: prod\n  auth-token: one\n- name: staging\n  auth-token: two\n";

        let err = migratable_auth_token(contents).unwrap_err();

        assert!(err.contains("multiple auth tokens"));
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
