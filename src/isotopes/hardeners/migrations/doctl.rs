#[cfg(all(target_os = "macos", not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const DIGITALOCEAN_ACCESS_TOKEN_KEY: &str = "DIGITALOCEAN_ACCESS_TOKEN";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[DIGITALOCEAN_ACCESS_TOKEN_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_credentials_file(&doctl_config_path()?, &KeychainCredentialStore).map(|_| ())
}

pub fn migrate_credentials_file(path: &Path, store: &dyn CredentialStore) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    let migration = config_migration(&contents)?;
    let Some(token) = migration.token else {
        return Ok(false);
    };

    store.store_secret(DIGITALOCEAN_ACCESS_TOKEN_KEY, &token)?;
    fs::write(path, migration.sanitized)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn doctl_config_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("DIGITALOCEAN_CONFIG").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    if cfg!(target_os = "macos") {
        Ok(home.join("Library/Application Support/doctl/config.yaml"))
    } else if let Some(xdg_config_home) =
        std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty())
    {
        Ok(PathBuf::from(xdg_config_home).join("doctl/config.yaml"))
    } else {
        Ok(home.join(".config/doctl/config.yaml"))
    }
}

fn doctl_config_contains_token(contents: &str) -> bool {
    let mut in_auth_contexts = false;

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent == 0 {
            in_auth_contexts = trimmed == "auth-contexts:";
            if line_has_non_empty_yaml_value(trimmed, "access-token") {
                return true;
            }
            continue;
        }
        if in_auth_contexts
            && yaml_value_after_colon(trimmed).is_some_and(|value| !value.is_empty())
        {
            return true;
        }
    }

    false
}

#[derive(Debug)]
struct ConfigMigration {
    sanitized: String,
    token: Option<String>,
}

fn config_migration(contents: &str) -> Result<ConfigMigration, String> {
    let mut output = Vec::new();
    let mut token = None;
    let mut current_context = None;
    let mut in_auth_contexts = false;

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            output.push(line.to_string());
            continue;
        }

        let indent = line.len() - line.trim_start().len();
        if indent == 0 {
            in_auth_contexts = trimmed == "auth-contexts:";
            if let Some(value) = top_level_yaml_value(trimmed, "context") {
                if !value.is_empty() {
                    current_context = Some(value.to_string());
                }
            }
            if let Some(value) = top_level_yaml_value(trimmed, "access-token") {
                if !value.is_empty() {
                    token = Some(value.to_string());
                    output.push("access-token: \"\"".to_string());
                    continue;
                }
            }
            output.push(line.to_string());
            continue;
        }

        if in_auth_contexts
            && yaml_value_after_colon(trimmed).is_some_and(|value| !value.is_empty())
        {
            return Err(
                "doctl named auth-context tokens must be migrated manually; \
DIGITALOCEAN_ACCESS_TOKEN only covers the default context"
                    .to_string(),
            );
        }
        output.push(line.to_string());
    }

    if token.is_some()
        && !current_context
            .as_deref()
            .unwrap_or("default")
            .eq("default")
    {
        return Err("doctl non-default contexts must be migrated manually; \
DIGITALOCEAN_ACCESS_TOKEN only covers the default context"
            .to_string());
    }

    let sanitized = if token.is_some() {
        let mut rendered = output.join("\n");
        if contents.ends_with('\n') {
            rendered.push('\n');
        }
        rendered
    } else {
        contents.to_string()
    };

    Ok(ConfigMigration { sanitized, token })
}

fn top_level_yaml_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.strip_prefix(key)
        .and_then(|rest| rest.strip_prefix(':'))
        .map(str::trim)
        .map(unquote_yaml_scalar)
}

fn line_has_non_empty_yaml_value(line: &str, key: &str) -> bool {
    line.strip_prefix(key)
        .and_then(|rest| rest.strip_prefix(':'))
        .map(str::trim)
        .map(unquote_yaml_scalar)
        .is_some_and(|value| !value.is_empty())
}

fn yaml_value_after_colon(line: &str) -> Option<&str> {
    line.split_once(':')
        .map(|(_, value)| unquote_yaml_scalar(value.trim()))
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
    fn keys_include_digitalocean_access_token() {
        assert_eq!(keys(), &[DIGITALOCEAN_ACCESS_TOKEN_KEY]);
    }

    #[test]
    fn detects_default_and_named_tokens() {
        assert!(doctl_config_contains_token("access-token: do_secret\n"));
        assert!(doctl_config_contains_token(
            "access-token: \"\"\nauth-contexts:\n  team: do_team_secret\n"
        ));
    }

    #[test]
    fn migrates_default_context_token_to_environment() {
        let path = std::env::temp_dir().join(format!("doctl-config-{}", std::process::id()));
        let contents = "access-token: do_secret\ncontext: default\n";
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        migrate_credentials_file(&path, &store).unwrap();

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                DIGITALOCEAN_ACCESS_TOKEN_KEY.to_string(),
                "do_secret".to_string()
            )]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "access-token: \"\"\ncontext: default\n"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_named_auth_context_tokens() {
        let err = config_migration("auth-contexts:\n  team: do_team_secret\n").unwrap_err();
        assert!(err.contains("auth-context"));
    }

    #[test]
    fn rejects_non_default_context_tokens() {
        let err = config_migration("access-token: do_secret\ncontext: team\n").unwrap_err();
        assert!(err.contains("default context"));
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
