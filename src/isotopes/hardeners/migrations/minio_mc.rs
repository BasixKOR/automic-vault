#[cfg(all(target_os = "macos", not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const MC_HOST_ENV_KEY: &str = "MINIO_MC_HOST_ENV";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[MC_HOST_ENV_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_credentials_file(&mc_config_path()?, &KeychainCredentialStore).map(|_| ())
}

pub fn migrate_credentials_file(path: &Path, store: &dyn CredentialStore) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };

    let host_env = mc_host_env(&contents)?;
    if host_env.is_empty() {
        return Ok(false);
    }

    store.store_secret(MC_HOST_ENV_KEY, &host_env.join("\n"))?;
    fs::write(path, sanitized_config_json(&contents)?)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn mc_config_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".mc/config.json"))
}

fn config_contains_secret(contents: &str) -> bool {
    json_string_key_has_nonempty_value(contents, "secretKey")
        || json_string_key_has_nonempty_value(contents, "sessionToken")
}

fn mc_host_env(contents: &str) -> Result<Vec<String>, String> {
    let value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|err| format!("failed to parse mc config JSON: {err}"))?;
    let Some(aliases) = value.get("aliases").and_then(serde_json::Value::as_object) else {
        return Ok(Vec::new());
    };

    let mut env = Vec::new();
    for (alias, alias_value) in aliases {
        let Some(alias) = env_safe_alias(alias) else {
            if alias_has_secret(alias_value) {
                return Err(format!(
                    "MinIO mc alias {alias:?} cannot be represented as an MC_HOST environment variable"
                ));
            }
            continue;
        };
        let Some(host) = mc_host_value(alias_value)? else {
            continue;
        };
        env.push(format!("MC_HOST_{alias}={host}"));
    }
    Ok(env)
}

fn env_safe_alias(alias: &str) -> Option<&str> {
    if alias.is_empty()
        || !alias
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return None;
    }
    Some(alias)
}

fn alias_has_secret(alias: &serde_json::Value) -> bool {
    string_field(alias, "secretKey").is_some_and(|value| !value.is_empty())
        || string_field(alias, "sessionToken").is_some_and(|value| !value.is_empty())
}

fn mc_host_value(alias: &serde_json::Value) -> Result<Option<String>, String> {
    let session_token = string_field(alias, "sessionToken").unwrap_or("");
    let Some(secret_key) = string_field(alias, "secretKey").filter(|value| !value.is_empty())
    else {
        if !session_token.is_empty() {
            return Err("MinIO mc alias with sessionToken is missing secretKey".to_string());
        }
        return Ok(None);
    };
    let access_key = string_field(alias, "accessKey").unwrap_or("");
    let url = string_field(alias, "url").unwrap_or("");
    if access_key.is_empty() || url.is_empty() {
        return Err("MinIO mc alias with secretKey is missing accessKey or url".to_string());
    }

    for value in [access_key, secret_key, url] {
        reject_env_line_breaks(value)?;
    }
    reject_env_line_breaks(session_token)?;

    Ok(Some(format_mc_host(
        access_key,
        secret_key,
        session_token,
        url,
    )))
}

fn string_field<'a>(value: &'a serde_json::Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(serde_json::Value::as_str)
}

fn reject_env_line_breaks(value: &str) -> Result<(), String> {
    if value.contains('\n') || value.contains('\r') {
        return Err("MinIO mc alias contains a value that cannot be exported safely".to_string());
    }
    Ok(())
}

fn format_mc_host(access_key: &str, secret_key: &str, session_token: &str, url: &str) -> String {
    let credential = if session_token.is_empty() {
        format!("{access_key}:{secret_key}")
    } else {
        format!("{access_key}:{secret_key}:{session_token}")
    };
    if let Some(rest) = url.strip_prefix("https://") {
        return format!("https://{credential}@{rest}");
    }
    if let Some(rest) = url.strip_prefix("http://") {
        return format!("http://{credential}@{rest}");
    }
    format!("{credential}@{url}")
}

fn sanitized_config_json(contents: &str) -> Result<String, String> {
    let mut value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|err| format!("failed to parse mc config JSON: {err}"))?;
    if let Some(aliases) = value
        .get_mut("aliases")
        .and_then(serde_json::Value::as_object_mut)
    {
        for alias in aliases.values_mut() {
            if let Some(alias) = alias.as_object_mut() {
                alias.remove("secretKey");
                alias.remove("sessionToken");
            }
        }
    }
    let mut json = serde_json::to_string_pretty(&value)
        .map_err(|err| format!("failed to encode sanitized mc config JSON: {err}"))?;
    json.push('\n');
    Ok(json)
}

fn json_string_key_has_nonempty_value(contents: &str, key: &str) -> bool {
    let quoted_key = format!("\"{key}\"");
    let mut rest = contents;
    while let Some(index) = rest.find(&quoted_key) {
        let after_key = &rest[index + quoted_key.len()..];
        let Some(colon_index) = after_key.find(':') else {
            return false;
        };
        let value = after_key[colon_index + 1..].trim_start();
        if value.starts_with('"') {
            return !value.starts_with("\"\"");
        }
        rest = &after_key[colon_index + 1..];
    }
    false
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

    struct FailingCredentialStore;

    impl CredentialStore for FailingCredentialStore {
        fn store_secret(&self, _key: &str, _value: &str) -> Result<(), String> {
            Err("store failed".to_string())
        }
    }

    #[test]
    fn migrates_minio_mc_config() {
        let path = std::env::temp_dir().join(format!("minio-mc-config-{}", std::process::id()));
        let contents = r#"{"aliases":{"minio":{"url":"https://play.min.io","accessKey":"fake","secretKey":"secret"}}}"#;
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_credentials_file(&path, &store).unwrap());

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                MC_HOST_ENV_KEY.to_string(),
                "MC_HOST_minio=https://fake:secret@play.min.io".to_string()
            )]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "{\n  \"aliases\": {\n    \"minio\": {\n      \"accessKey\": \"fake\",\n      \"url\": \"https://play.min.io\"\n    }\n  }\n}\n"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn migrates_session_token_alias() {
        let path =
            std::env::temp_dir().join(format!("minio-mc-session-config-{}", std::process::id()));
        let contents = r#"{"aliases":{"sts":{"url":"https://play.min.io","accessKey":"fake","secretKey":"secret","sessionToken":"session"}}}"#;
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_credentials_file(&path, &store).unwrap());

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                MC_HOST_ENV_KEY.to_string(),
                "MC_HOST_sts=https://fake:secret:session@play.min.io".to_string()
            )]
        );
        assert!(!fs::read_to_string(&path).unwrap().contains("sessionToken"));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn refuses_aliases_that_cannot_be_exported() {
        let path = std::env::temp_dir().join(format!("minio-mc-bad-alias-{}", std::process::id()));
        let contents = r#"{"aliases":{"my-minio":{"url":"https://play.min.io","accessKey":"fake","secretKey":"secret"}}}"#;
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert_eq!(
            migrate_credentials_file(&path, &store).unwrap_err(),
            "MinIO mc alias \"my-minio\" cannot be represented as an MC_HOST environment variable"
        );
        assert!(store.values.borrow().is_empty());
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn refuses_session_token_without_secret_key() {
        let path =
            std::env::temp_dir().join(format!("minio-mc-session-only-{}", std::process::id()));
        let contents = r#"{"aliases":{"sts":{"url":"https://play.min.io","accessKey":"fake","sessionToken":"session"}}}"#;
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert_eq!(
            migrate_credentials_file(&path, &store).unwrap_err(),
            "MinIO mc alias with sessionToken is missing secretKey"
        );
        assert!(store.values.borrow().is_empty());
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn does_not_migrate_without_secret_values() {
        let path = std::env::temp_dir().join(format!("minio-mc-empty-{}", std::process::id()));
        fs::write(&path, r#"{"aliases":{"minio":{"accessKey":"fake"}}}"#).unwrap();
        let store = TestCredentialStore::default();

        assert!(!migrate_credentials_file(&path, &store).unwrap());
        assert!(store.values.borrow().is_empty());
        fs::remove_file(path).unwrap();
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

    #[test]
    fn keys_path_and_secret_detection_helpers_cover_edge_cases() {
        assert_eq!(keys(), &[MC_HOST_ENV_KEY]);
        assert!(config_contains_secret("{\"secretKey\":\"secret\"}"));
        assert!(config_contains_secret("{\"sessionToken\":\"secret\"}"));
        assert!(!config_contains_secret("{\"secretKey\":\"\"}"));
        assert!(json_string_key_has_nonempty_value(
            "{\"secretKey\":\"secret\"}",
            "secretKey"
        ));
        assert!(!json_string_key_has_nonempty_value(
            "{\"secretKey\":\"\"}",
            "secretKey"
        ));

        let previous_home = std::env::var_os("HOME");
        unsafe { std::env::remove_var("HOME") };
        let err = mc_config_path().unwrap_err();
        assert!(err.contains("HOME is not set"));
        match previous_home {
            Some(value) => unsafe { std::env::set_var("HOME", value) },
            None => unsafe { std::env::remove_var("HOME") },
        }
    }

    #[test]
    fn migrate_credentials_ignores_missing_and_preserves_file_on_store_failure() {
        let temp = std::env::temp_dir();
        let missing = temp.join(format!("minio-missing-{}", std::process::id()));
        let path = temp.join(format!("minio-store-failure-{}", std::process::id()));
        let contents = r#"{"aliases":{"minio":{"url":"https://play.min.io","accessKey":"fake","secretKey":"secret"}}}"#;
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert!(!migrate_credentials_file(&missing, &store).unwrap());
        let err = migrate_credentials_file(&path, &FailingCredentialStore).unwrap_err();
        assert!(err.contains("store failed"));
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);
        fs::remove_file(path).unwrap();
    }
}
