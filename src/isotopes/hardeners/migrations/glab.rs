#[cfg(all(target_os = "macos", not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const GLAB_ENV_ASSIGNMENTS_KEY: &str = "GLAB_ENV_ASSIGNMENTS";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[GLAB_ENV_ASSIGNMENTS_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    for path in candidate_config_paths()? {
        if migrate_credentials_file(&path, &KeychainCredentialStore)? {
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
    let assignments = glab_env_assignments(&contents)?;
    if assignments.is_empty() {
        return Ok(false);
    }

    store.store_secret(GLAB_ENV_ASSIGNMENTS_KEY, &assignments.join("\n"))?;
    fs::write(path, sanitized_glab_config(&contents))
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn candidate_config_paths() -> Result<Vec<PathBuf>, String> {
    if let Some(dir) = std::env::var_os("GLAB_CONFIG_DIR").filter(|value| !value.is_empty()) {
        return Ok(vec![PathBuf::from(dir).join("config.yml")]);
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    let mut paths = vec![home.join(".config/glab-cli/config.yml")];
    if let Some(config) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        paths.push(PathBuf::from(config).join("glab-cli/config.yml"));
    }
    paths.push(home.join("Library/Application Support/glab-cli/config.yml"));
    Ok(paths)
}

fn glab_config_contains_token(contents: &str) -> bool {
    contents.lines().any(|line| {
        let trimmed = line.trim_start();
        ["token:", "oauth2_refresh_token:"]
            .iter()
            .any(|prefix| line_has_non_empty_value(trimmed, prefix))
    })
}

struct HostToken {
    host: String,
    token: String,
}

fn glab_env_assignments(contents: &str) -> Result<Vec<String>, String> {
    if glab_config_contains_oauth_refresh_token(contents) {
        return Err(
            "GLab OAuth refresh-token configs cannot be represented with a token env var"
                .to_string(),
        );
    }

    let tokens = glab_host_tokens(contents);
    match tokens.len() {
        0 => Ok(Vec::new()),
        1 => {
            let token = tokens.into_iter().next().expect("one token");
            reject_env_line_breaks("GITLAB_TOKEN", &token.token)?;
            reject_env_line_breaks("GITLAB_HOST", &token.host)?;
            Ok(vec![
                format!("GITLAB_TOKEN={}", token.token),
                format!("GITLAB_HOST={}", token.host),
            ])
        }
        _ => Err("GLab configs with multiple host tokens must be migrated manually".to_string()),
    }
}

fn glab_config_contains_oauth_refresh_token(contents: &str) -> bool {
    contents
        .lines()
        .any(|line| line_has_non_empty_value(line.trim_start(), "oauth2_refresh_token:"))
}

fn glab_host_tokens(contents: &str) -> Vec<HostToken> {
    let mut tokens = Vec::new();
    let mut in_hosts = false;
    let mut current_host = None;

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed == "hosts:" {
            in_hosts = true;
            current_host = None;
            continue;
        }
        if !in_hosts {
            continue;
        }
        if !line.starts_with(' ') && !line.starts_with('\t') {
            in_hosts = false;
            current_host = None;
            continue;
        }
        if host_header_line(line) {
            current_host = trimmed.strip_suffix(':').map(str::to_string);
            continue;
        }
        if let Some(value) = yaml_value(trimmed, "token:")
            && let Some(host) = current_host.clone()
        {
            tokens.push(HostToken {
                host,
                token: value.to_string(),
            });
        }
    }

    tokens
}

fn host_header_line(line: &str) -> bool {
    let indent = line.len() - line.trim_start().len();
    indent == 2 && line.trim().ends_with(':')
}

fn yaml_value<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    line.strip_prefix(prefix)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "\"\"" && *value != "''")
        .map(unquote_yaml_scalar)
}

fn sanitized_glab_config(contents: &str) -> String {
    let mut output = Vec::new();
    for line in contents.lines() {
        if line_has_non_empty_value(line.trim_start(), "token:") {
            let indent = line.len() - line.trim_start().len();
            output.push(format!("{}token: \"\"", " ".repeat(indent)));
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
    fn migrates_glab_config() {
        let path = std::env::temp_dir().join(format!("glab-config-{}", std::process::id()));
        let contents = "hosts:\n  gitlab.example.com:\n    token: glpat-secret\n    api_host: https://gitlab.example.com/api/v4/\n";
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        migrate_credentials_file(&path, &store).unwrap();

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                GLAB_ENV_ASSIGNMENTS_KEY.to_string(),
                "GITLAB_TOKEN=glpat-secret\nGITLAB_HOST=gitlab.example.com".to_string()
            )]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "hosts:\n  gitlab.example.com:\n    token: \"\"\n    api_host: https://gitlab.example.com/api/v4/\n"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_oauth_refresh_token_configs() {
        let err = glab_env_assignments(
            "hosts:\n  gitlab.com:\n    token: oauth-token\n    oauth2_refresh_token: refresh\n",
        )
        .unwrap_err();

        assert!(err.contains("OAuth refresh-token"));
    }

    #[test]
    fn rejects_multiple_host_tokens() {
        let err = glab_env_assignments(
            "hosts:\n  gitlab.com:\n    token: one\n  gitlab.example.com:\n    token: two\n",
        )
        .unwrap_err();

        assert!(err.contains("multiple host tokens"));
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
