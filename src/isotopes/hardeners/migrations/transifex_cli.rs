#[cfg(all(target_os = "macos", not(test), not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const TRANSIFEX_ENV_ASSIGNMENTS_KEY: &str = "TRANSIFEX_ENV_ASSIGNMENTS";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[TRANSIFEX_ENV_ASSIGNMENTS_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_root_config(&transifex_root_config_path()?, &KeychainCredentialStore).map(|_| ())
}

pub fn migrate_root_config(path: &Path, store: &dyn CredentialStore) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    let assignments = transifex_env_assignments(&contents)?;
    if assignments.is_empty() {
        return Ok(false);
    }

    store.store_secret(TRANSIFEX_ENV_ASSIGNMENTS_KEY, &assignments.join("\n"))?;
    fs::write(path, scrub_root_config(&contents))
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn transifex_root_config_path() -> Result<PathBuf, String> {
    Ok(user_home()?.join(".transifexrc"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn root_config_contains_secret(contents: &str) -> bool {
    contents.lines().any(line_has_secret)
}

fn transifex_env_assignments(contents: &str) -> Result<Vec<String>, String> {
    if root_config_contains_legacy_password(contents) {
        return Err(
            "Transifex legacy password configs cannot be represented with TX_TOKEN".to_string(),
        );
    }

    let tokens = transifex_tokens(contents);
    match tokens.len() {
        0 => Ok(Vec::new()),
        1 => {
            let token = tokens.into_iter().next().expect("one token");
            reject_env_line_breaks("TX_TOKEN", &token.token)?;
            let mut assignments = vec![format!("TX_TOKEN={}", token.token)];
            if let Some(hostname) = token.hostname {
                reject_env_line_breaks("TX_HOSTNAME", &hostname)?;
                assignments.push(format!("TX_HOSTNAME={hostname}"));
            }
            Ok(assignments)
        }
        _ => Err("Transifex root config has multiple tokens; migrate them manually".to_string()),
    }
}

struct TransifexToken {
    token: String,
    hostname: Option<String>,
}

fn transifex_tokens(contents: &str) -> Vec<TransifexToken> {
    let mut tokens = Vec::new();
    let mut current_token = None;
    let mut current_rest_hostname = None;
    let mut current_section = None;

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if let Some(section) = section_name(trimmed) {
            push_token(
                &mut tokens,
                current_token.take(),
                current_rest_hostname.take().or(current_section.take()),
            );
            current_section = Some(section.to_string());
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = unquote(value.trim());
        if value.is_empty() {
            continue;
        }
        match key {
            "token" if value != "__api_token__" => current_token = Some(value.to_string()),
            "rest_hostname" => current_rest_hostname = Some(value.to_string()),
            _ => {}
        }
    }

    push_token(
        &mut tokens,
        current_token,
        current_rest_hostname.or(current_section),
    );
    tokens
}

fn push_token(tokens: &mut Vec<TransifexToken>, token: Option<String>, hostname: Option<String>) {
    let Some(token) = token else {
        return;
    };
    if tokens
        .iter()
        .any(|existing| existing.token == token && existing.hostname == hostname)
    {
        return;
    }
    tokens.push(TransifexToken { token, hostname });
}

fn section_name(trimmed: &str) -> Option<&str> {
    trimmed.strip_prefix('[')?.strip_suffix(']').map(str::trim)
}

fn root_config_contains_legacy_password(contents: &str) -> bool {
    contents.lines().any(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            return false;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            return false;
        };
        let value = unquote(value.trim());
        key.trim() == "password" && !value.is_empty() && value != "__password_or_api_token__"
    })
}

fn reject_env_line_breaks(key: &str, value: &str) -> Result<(), String> {
    if value.contains('\n') || value.contains('\r') {
        return Err(format!("{key} cannot contain line breaks"));
    }
    Ok(())
}

fn line_has_secret(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
        return false;
    }
    let Some((key, value)) = trimmed.split_once('=') else {
        return false;
    };
    let key = key.trim();
    let value = unquote(value.trim());
    matches!(key, "token" | "password")
        && !value.is_empty()
        && value != "__api_token__"
        && value != "__password_or_api_token__"
}

fn scrub_root_config(contents: &str) -> String {
    let mut output = String::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed
            .split_once('=')
            .is_some_and(|(key, _)| key.trim() == "token" && line_has_secret(line))
        {
            let indent_len = line.len() - line.trim_start().len();
            output.push_str(&" ".repeat(indent_len));
            output.push_str("token =\n");
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }
    if !contents.ends_with('\n') {
        output.pop();
    }
    output
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
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
    fn migrates_and_scrubs_root_config_token() {
        let temp = test_dir("transifex-migrate");
        let path = temp.join(".transifexrc");
        let contents = "[https://app.transifex.com]\nrest_hostname = https://rest.api.transifex.com\ntoken = fake-token\n";
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_root_config(&path, &store).unwrap());

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                TRANSIFEX_ENV_ASSIGNMENTS_KEY.to_string(),
                "TX_TOKEN=fake-token\nTX_HOSTNAME=https://rest.api.transifex.com".to_string()
            )]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "[https://app.transifex.com]\nrest_hostname = https://rest.api.transifex.com\ntoken =\n"
        );
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn rejects_legacy_password_configs() {
        let err = transifex_env_assignments("[host]\npassword = legacy-password\n").unwrap_err();

        assert!(err.contains("legacy password"));
    }

    #[test]
    fn rejects_multiple_token_configs() {
        let contents = "[host-a]\ntoken = one\n[host-b]\ntoken = two\n";

        let err = transifex_env_assignments(contents).unwrap_err();

        assert!(err.contains("multiple tokens"));
    }

    #[test]
    fn skips_root_config_without_secret() {
        let temp = test_dir("transifex-skip");
        let path = temp.join(".transifexrc");
        fs::write(&path, "[https://app.transifex.com]\ntoken =\n").unwrap();
        let store = TestCredentialStore::default();

        assert!(!migrate_root_config(&path, &store).unwrap());
        assert!(store.values.borrow().is_empty());
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn does_not_scrub_when_store_fails() {
        let temp = test_dir("transifex-fail");
        let path = temp.join(".transifexrc");
        let contents = "[host]\ntoken = fake-token\n";
        fs::write(&path, contents).unwrap();

        assert_eq!(
            migrate_root_config(&path, &FailingCredentialStore).unwrap_err(),
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
