#[cfg(all(target_os = "macos", not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::PathBuf;

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const NETLIFY_AUTH_TOKEN_ENV_KEY: &str = "NETLIFY_AUTH_TOKEN";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[NETLIFY_AUTH_TOKEN_ENV_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_candidate_config(&candidate_config_paths()?, &KeychainCredentialStore).map(|_| ())
}

fn migrate_candidate_config(
    paths: &[PathBuf],
    store: &dyn CredentialStore,
) -> Result<bool, String> {
    let candidates = migratable_configs(paths)?;

    if candidates.is_empty() {
        return Ok(false);
    }
    if candidates.len() > 1 {
        let joined = candidates
            .iter()
            .map(|candidate| candidate.path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "multiple Netlify config files contain plaintext credentials; migrate them manually: {joined}"
        ));
    }

    let candidate = &candidates[0];
    store.store_secret(NETLIFY_AUTH_TOKEN_ENV_KEY, &candidate.token)?;
    fs::write(&candidate.path, &candidate.sanitized)
        .map_err(|err| format!("failed to write {}: {err}", candidate.path.display()))?;
    Ok(true)
}

fn migratable_configs(paths: &[PathBuf]) -> Result<Vec<MigratableConfig>, String> {
    let mut configs = Vec::new();

    for path in paths {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
        };

        let Some(migratable) = migratable_config_json(&contents)? else {
            continue;
        };

        configs.push(MigratableConfig {
            path: path.clone(),
            token: migratable.token,
            sanitized: migratable.sanitized,
        });
    }

    Ok(configs)
}

fn candidate_config_paths() -> Result<Vec<PathBuf>, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(vec![
        home.join("Library/Preferences/netlify/config.json"),
        home.join(".netlify/config.json"),
    ])
}

#[derive(Debug)]
struct MigratableConfig {
    path: PathBuf,
    token: String,
    sanitized: String,
}

#[derive(Debug)]
struct MigratableJson {
    token: String,
    sanitized: String,
}

fn migratable_config_json(contents: &str) -> Result<Option<MigratableJson>, String> {
    let mut value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|err| format!("failed to parse netlify config JSON: {err}"))?;
    let tokens = netlify_auth_tokens(&value)?;
    if tokens.is_empty() {
        return Ok(None);
    }
    if netlify_github_token_count(&value) > 0 {
        return Err(
            "Netlify configs with embedded GitHub tokens must be migrated manually".to_string(),
        );
    }
    if tokens.len() > 1 {
        return Err(
            "Netlify configs with multiple auth tokens must be migrated manually".to_string(),
        );
    }
    let token = tokens.into_iter().next().expect("one token");

    if let Some(users) = value
        .get_mut("users")
        .and_then(serde_json::Value::as_object_mut)
    {
        for user in users.values_mut() {
            let Some(user) = user.as_object_mut() else {
                continue;
            };
            let Some(auth) = user
                .get_mut("auth")
                .and_then(serde_json::Value::as_object_mut)
            else {
                continue;
            };

            if let Some(token) = auth.get_mut("token")
                && token.as_str().is_some_and(|value| !value.is_empty())
            {
                *token = serde_json::Value::String(String::new());
            }
        }
    }

    let mut json = serde_json::to_string_pretty(&value)
        .map_err(|err| format!("failed to encode sanitized netlify config JSON: {err}"))?;
    json.push('\n');
    Ok(Some(MigratableJson {
        token,
        sanitized: json,
    }))
}

fn netlify_auth_tokens(value: &serde_json::Value) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    if let Some(users) = value.get("users").and_then(serde_json::Value::as_object) {
        for user in users.values() {
            let Some(auth) = user
                .as_object()
                .and_then(|user| user.get("auth"))
                .and_then(serde_json::Value::as_object)
            else {
                continue;
            };
            if let Some(token) = auth
                .get("token")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.is_empty())
            {
                reject_env_line_breaks(token)?;
                if !tokens.iter().any(|existing| existing == token) {
                    tokens.push(token.to_string());
                }
            }
        }
    }
    Ok(tokens)
}

fn netlify_github_token_count(value: &serde_json::Value) -> usize {
    value
        .get("users")
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flat_map(|users| users.values())
        .filter_map(serde_json::Value::as_object)
        .filter_map(|user| user.get("auth").and_then(serde_json::Value::as_object))
        .filter_map(|auth| {
            auth.get("github")
                .and_then(serde_json::Value::as_object)
                .and_then(|github| github.get("token"))
                .and_then(serde_json::Value::as_str)
        })
        .filter(|value| !value.is_empty())
        .count()
}

fn reject_env_line_breaks(value: &str) -> Result<(), String> {
    if value.contains('\n') || value.contains('\r') {
        return Err("NETLIFY_AUTH_TOKEN cannot contain line breaks".to_string());
    }
    Ok(())
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
    fn migrates_current_config_and_blanks_netlify_token() {
        let home = std::env::temp_dir().join(format!("netlify-home-{}", std::process::id()));
        let path = home.join("Library/Preferences/netlify/config.json");
        let _ = fs::remove_dir_all(&home);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let contents = r#"{
  "userId": "user-1",
  "users": {
    "user-1": {
      "id": "user-1",
      "auth": {
        "token": "ntl_secret"
      }
    }
  }
}
"#;
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_candidate_config(&[path.clone()], &store).unwrap());
        let sanitized = fs::read_to_string(&path).unwrap();
        assert!(sanitized.contains("\"token\": \"\""));
        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                NETLIFY_AUTH_TOKEN_ENV_KEY.to_string(),
                "ntl_secret".to_string()
            )]
        );
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn rejects_embedded_github_tokens() {
        let contents = r#"{
  "users": {
    "user-1": {
      "auth": {
        "token": "ntl_secret",
        "github": { "token": "gho_secret" }
      }
    }
  }
}
"#;

        let err = migratable_config_json(contents).unwrap_err();

        assert!(err.contains("GitHub tokens"));
    }

    #[test]
    fn rejects_multiple_netlify_tokens() {
        let contents = r#"{
  "users": {
    "user-1": { "auth": { "token": "one" } },
    "user-2": { "auth": { "token": "two" } }
  }
}
"#;

        let err = migratable_config_json(contents).unwrap_err();

        assert!(err.contains("multiple auth tokens"));
    }

    #[test]
    fn prefers_manual_resolution_when_multiple_configs_need_migration() {
        let home = std::env::temp_dir().join(format!("netlify-multi-{}", std::process::id()));
        let current = home.join("Library/Preferences/netlify/config.json");
        let legacy = home.join(".netlify/config.json");
        let _ = fs::remove_dir_all(&home);
        fs::create_dir_all(current.parent().unwrap()).unwrap();
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        fs::write(
            &current,
            "{\"users\":{\"a\":{\"auth\":{\"token\":\"one\"}}}}\n",
        )
        .unwrap();
        fs::write(
            &legacy,
            "{\"users\":{\"b\":{\"auth\":{\"token\":\"two\"}}}}\n",
        )
        .unwrap();
        let store = TestCredentialStore::default();

        let error =
            migrate_candidate_config(&[current.clone(), legacy.clone()], &store).unwrap_err();

        assert!(error.contains("multiple Netlify config files contain plaintext credentials"));
        assert!(store.values.borrow().is_empty());
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn top_level_migrate_credentials_ignores_missing_default_locations() {
        let home = std::env::temp_dir().join(format!(
            "{}-migrate-missing-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();

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

        std::fs::remove_dir_all(home).unwrap();
    }
}
