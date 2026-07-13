#[cfg(all(target_os = "macos", not(test), not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const GRAFANACTL_ENV_ASSIGNMENTS_KEY: &str = "GRAFANACTL_ENV_ASSIGNMENTS";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[GRAFANACTL_ENV_ASSIGNMENTS_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_config_file(&grafanactl_config_path()?, &KeychainCredentialStore).map(|_| ())
}

pub fn migrate_config_file(path: &Path, store: &dyn CredentialStore) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };

    let assignments = grafanactl_env_assignments(&contents)?;
    if assignments.is_empty() {
        return Ok(false);
    }

    store.store_secret(GRAFANACTL_ENV_ASSIGNMENTS_KEY, &assignments.join("\n"))?;
    fs::write(path, sanitized_config_yaml(&contents))
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn grafanactl_config_path() -> Result<PathBuf, String> {
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty())
    {
        return Ok(PathBuf::from(config_home).join("grafanactl/config.yaml"));
    }
    Ok(user_home()?.join(".config/grafanactl/config.yaml"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn config_contains_secret(contents: &str) -> bool {
    contents.lines().any(yaml_secret_line_is_present)
}

fn yaml_secret_line_is_present(line: &str) -> bool {
    let line = line.split('#').next().unwrap_or("").trim();
    let Some((name, value)) = line.split_once(':') else {
        return false;
    };
    let name = name.trim();
    let value = value.trim().trim_matches('"').trim_matches('\'');
    matches!(name, "token" | "password") && !value.is_empty()
}

#[derive(Default)]
struct GrafanaContext {
    name: String,
    token: Option<String>,
    user: Option<String>,
    password: Option<String>,
}

fn grafanactl_env_assignments(contents: &str) -> Result<Vec<String>, String> {
    let contexts = grafana_secret_contexts(contents);
    match contexts.len() {
        0 => Ok(Vec::new()),
        1 => contexts
            .into_iter()
            .next()
            .expect("one context")
            .env_assignments(),
        _ => Err(
            "grafanactl configs with multiple secret-bearing contexts must be migrated manually"
                .to_string(),
        ),
    }
}

impl GrafanaContext {
    fn contains_secret(&self) -> bool {
        self.token.is_some() || self.password.is_some()
    }

    fn env_assignments(self) -> Result<Vec<String>, String> {
        let mut assignments = Vec::new();
        if let Some(token) = self.token {
            reject_env_line_breaks("GRAFANA_TOKEN", &token)?;
            assignments.push(format!("GRAFANA_TOKEN={token}"));
        }
        if let Some(password) = self.password {
            let user = self.user.ok_or_else(|| {
                format!(
                    "grafanactl context [{}] has a password but no user",
                    self.name
                )
            })?;
            reject_env_line_breaks("GRAFANA_USER", &user)?;
            reject_env_line_breaks("GRAFANA_PASSWORD", &password)?;
            assignments.push(format!("GRAFANA_USER={user}"));
            assignments.push(format!("GRAFANA_PASSWORD={password}"));
        }
        Ok(assignments)
    }
}

fn grafana_secret_contexts(contents: &str) -> Vec<GrafanaContext> {
    let mut contexts = Vec::new();
    let mut current = GrafanaContext::default();
    let mut in_contexts = false;
    let mut in_grafana = false;

    for line in contents.lines() {
        let uncommented = line.split('#').next().unwrap_or("");
        let trimmed = uncommented.trim();
        if trimmed.is_empty() {
            continue;
        }

        let indent = uncommented.len() - uncommented.trim_start().len();
        if indent == 0 {
            if current.contains_secret() {
                contexts.push(current);
            }
            current = GrafanaContext::default();
            in_contexts = trimmed == "contexts:";
            in_grafana = false;
            continue;
        }
        if !in_contexts {
            continue;
        }
        if indent == 2 && trimmed.ends_with(':') {
            if current.contains_secret() {
                contexts.push(current);
            }
            current = GrafanaContext {
                name: trimmed.trim_end_matches(':').to_string(),
                ..GrafanaContext::default()
            };
            in_grafana = false;
            continue;
        }
        if indent == 4 && trimmed == "grafana:" {
            in_grafana = true;
            continue;
        }
        if !in_grafana || indent < 6 {
            continue;
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let value = unquote_yaml_scalar(value.trim());
        if value.is_empty() {
            continue;
        }
        match key.trim() {
            "token" => current.token = Some(value.to_string()),
            "user" => current.user = Some(value.to_string()),
            "password" => current.password = Some(value.to_string()),
            _ => {}
        }
    }

    if current.contains_secret() {
        contexts.push(current);
    }
    contexts
}

fn sanitized_config_yaml(contents: &str) -> String {
    let mut output = Vec::new();
    for line in contents.lines() {
        let uncommented = line.split('#').next().unwrap_or("").trim();
        if uncommented.split_once(':').is_some_and(|(key, value)| {
            matches!(key.trim(), "token" | "password")
                && !unquote_yaml_scalar(value.trim()).is_empty()
        }) {
            let indent = line.len() - line.trim_start().len();
            let key = uncommented
                .split_once(':')
                .map(|(key, _)| key.trim())
                .unwrap_or("token");
            output.push(format!("{}{}: \"\"", " ".repeat(indent), key));
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

fn reject_env_line_breaks(key: &str, value: &str) -> Result<(), String> {
    if value.contains('\n') || value.contains('\r') {
        return Err(format!("{key} cannot contain line breaks"));
    }
    Ok(())
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
    fn migrates_grafanactl_config_file() {
        let path = std::env::temp_dir().join(format!("grafanactl-config-{}", std::process::id()));
        let contents = "contexts:\n  default:\n    grafana:\n      server: https://grafana.example.com\n      token: fake-token\n";
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_config_file(&path, &store).unwrap());

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                GRAFANACTL_ENV_ASSIGNMENTS_KEY.to_string(),
                "GRAFANA_TOKEN=fake-token".to_string()
            )]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "contexts:\n  default:\n    grafana:\n      server: https://grafana.example.com\n      token: \"\"\n"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn migrates_basic_auth_password_with_user() {
        let contents = "contexts:\n  default:\n    grafana:\n      user: admin\n      password: fake-password\n";

        assert_eq!(
            grafanactl_env_assignments(contents).unwrap(),
            vec![
                "GRAFANA_USER=admin".to_string(),
                "GRAFANA_PASSWORD=fake-password".to_string()
            ]
        );
    }

    #[test]
    fn rejects_password_without_user() {
        let err = grafanactl_env_assignments(
            "contexts:\n  default:\n    grafana:\n      password: fake\n",
        )
        .unwrap_err();

        assert!(err.contains("password but no user"));
    }

    #[test]
    fn rejects_multiple_secret_contexts() {
        let contents = "contexts:\n  default:\n    grafana:\n      token: one\n  prod:\n    grafana:\n      token: two\n";

        let err = grafanactl_env_assignments(contents).unwrap_err();

        assert!(err.contains("multiple secret-bearing contexts"));
    }

    #[test]
    fn does_not_migrate_without_secret() {
        let path = std::env::temp_dir().join(format!("grafanactl-no-token-{}", std::process::id()));
        fs::write(
            &path,
            "contexts:\n  default:\n    grafana:\n      org-id: 1\n",
        )
        .unwrap();
        let store = TestCredentialStore::default();

        assert!(!migrate_config_file(&path, &store).unwrap());
        assert!(store.values.borrow().is_empty());
        fs::remove_file(path).unwrap();
    }
}
