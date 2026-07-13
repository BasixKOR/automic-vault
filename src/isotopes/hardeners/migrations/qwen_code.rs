#[cfg(all(target_os = "macos", not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const QWEN_ENV_ASSIGNMENTS_KEY: &str = "QWEN_ENV_ASSIGNMENTS";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[QWEN_ENV_ASSIGNMENTS_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_credentials_file(&qwen_settings_path()?, &KeychainCredentialStore).map(|_| ())
}

pub fn migrate_credentials_file(path: &Path, store: &dyn CredentialStore) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    let assignments = qwen_env_assignments(&contents)?;
    if assignments.is_empty() {
        return Ok(false);
    }

    store.store_secret(QWEN_ENV_ASSIGNMENTS_KEY, &assignments.join("\n"))?;
    fs::write(path, sanitized_settings_json(&contents)?)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn qwen_settings_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".qwen/settings.json"))
}

fn qwen_settings_contains_env_secret(contents: &str) -> bool {
    json_object_field(contents, "env").is_some_and(json_object_contains_nonempty_string_value)
}

fn qwen_env_assignments(contents: &str) -> Result<Vec<String>, String> {
    let value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|err| format!("failed to parse qwen settings JSON: {err}"))?;
    let Some(env) = value.get("env").and_then(serde_json::Value::as_object) else {
        return Ok(Vec::new());
    };

    let mut assignments = Vec::new();
    for (key, value) in env {
        let Some(value) = value.as_str().filter(|value| !value.is_empty()) else {
            continue;
        };
        if !env_safe_name(key) {
            return Err(format!(
                "Qwen env key {key:?} cannot be represented as an environment variable"
            ));
        }
        reject_env_line_breaks(value)?;
        assignments.push(format!("{key}={value}"));
    }
    Ok(assignments)
}

fn env_safe_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn reject_env_line_breaks(value: &str) -> Result<(), String> {
    if value.contains('\n') || value.contains('\r') {
        return Err("Qwen env value cannot be exported safely".to_string());
    }
    Ok(())
}

fn sanitized_settings_json(contents: &str) -> Result<String, String> {
    let mut value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|err| format!("failed to parse qwen settings JSON: {err}"))?;
    if let Some(env) = value
        .get_mut("env")
        .and_then(serde_json::Value::as_object_mut)
    {
        env.retain(|_, value| !value.as_str().is_some_and(|value| !value.is_empty()));
    }
    let mut json = serde_json::to_string_pretty(&value)
        .map_err(|err| format!("failed to encode sanitized qwen settings JSON: {err}"))?;
    json.push('\n');
    Ok(json)
}

fn json_object_field<'a>(contents: &'a str, field: &str) -> Option<&'a str> {
    let quoted = format!("\"{field}\"");
    let key_start = contents.find(&quoted)?;
    let after_key = &contents[key_start + quoted.len()..];
    let after_colon = after_key.split_once(':')?.1.trim_start();
    let object_start = after_colon.find('{')?;
    let object = &after_colon[object_start..];
    let mut in_string = false;
    let mut escaped = false;
    let mut depth = 0usize;

    for (index, byte) in object.bytes().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }

        match byte {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(&object[..=index]);
                }
            }
            _ => {}
        }
    }

    None
}

fn json_object_contains_nonempty_string_value(object: &str) -> bool {
    let mut rest = object;
    while let Some((_, after_colon)) = rest.split_once(':') {
        let value = after_colon.trim_start();
        if let Some(after_quote) = value.strip_prefix('"') {
            if let Some((string_value, after_string)) = after_quote.split_once('"') {
                if !string_value.is_empty() {
                    return true;
                }
                rest = after_string;
                continue;
            }
            return false;
        }
        rest = after_colon.get(1..).unwrap_or_default();
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

    #[test]
    fn migrates_qwen_settings() {
        let path = std::env::temp_dir().join(format!("qwen-settings-{}", std::process::id()));
        let contents = "{\"modelProviders\":{\"dashscope\":[{\"envKey\":\"DASHSCOPE_API_KEY\"}]},\"env\":{\"DASHSCOPE_API_KEY\":\"sk-test\",\"EMPTY\":\"\"}}\n";
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        migrate_credentials_file(&path, &store).unwrap();

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                QWEN_ENV_ASSIGNMENTS_KEY.to_string(),
                "DASHSCOPE_API_KEY=sk-test".to_string()
            )]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "{\n  \"env\": {\n    \"EMPTY\": \"\"\n  },\n  \"modelProviders\": {\n    \"dashscope\": [\n      {\n        \"envKey\": \"DASHSCOPE_API_KEY\"\n      }\n    ]\n  }\n}\n"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn refuses_env_keys_that_cannot_be_exported() {
        let path = std::env::temp_dir().join(format!("qwen-bad-env-{}", std::process::id()));
        let contents = "{\"env\":{\"BAD-NAME\":\"sk-test\"}}\n";
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert_eq!(
            migrate_credentials_file(&path, &store).unwrap_err(),
            "Qwen env key \"BAD-NAME\" cannot be represented as an environment variable"
        );
        assert!(store.values.borrow().is_empty());
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn refuses_values_with_line_breaks() {
        let path = std::env::temp_dir().join(format!("qwen-bad-value-{}", std::process::id()));
        let contents = "{\"env\":{\"DASHSCOPE_API_KEY\":\"sk\\ntest\"}}\n";
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert_eq!(
            migrate_credentials_file(&path, &store).unwrap_err(),
            "Qwen env value cannot be exported safely"
        );
        assert!(store.values.borrow().is_empty());
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn keys_and_helpers_cover_edges() {
        assert_eq!(keys(), &[QWEN_ENV_ASSIGNMENTS_KEY]);
        assert!(env_safe_name("DASHSCOPE_API_KEY"));
        assert!(env_safe_name("_TOKEN"));
        assert!(!env_safe_name("1TOKEN"));
        assert!(!env_safe_name("BAD-NAME"));
        assert!(!qwen_settings_contains_env_secret(
            r#"{"env":{"DASHSCOPE_API_KEY":""}}"#
        ));
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
