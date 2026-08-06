#[cfg(all(target_os = "macos", not(test), not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const ALGOLIA_ENV_ASSIGNMENTS_KEY: &str = "ALGOLIA_ENV_ASSIGNMENTS";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[ALGOLIA_ENV_ASSIGNMENTS_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_config_file(&algolia_config_path()?, &KeychainCredentialStore).map(|_| ())
}

pub fn migrate_config_file(path: &Path, store: &dyn CredentialStore) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };

    let assignments = algolia_env_assignments(&contents)?;
    if assignments.is_empty() {
        return Ok(false);
    }

    store.store_secret(ALGOLIA_ENV_ASSIGNMENTS_KEY, &assignments.join("\n"))?;
    fs::write(path, sanitized_config_toml(&contents))
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn algolia_config_path() -> Result<PathBuf, String> {
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty())
    {
        return Ok(PathBuf::from(config_home).join("algolia/config.toml"));
    }
    Ok(user_home()?.join(".config/algolia/config.toml"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn config_contains_secret(contents: &str) -> bool {
    ["api_key", "admin_api_key", "crawler_api_key"]
        .iter()
        .any(|field| toml_string_field_is_present(contents, field))
}

#[derive(Default)]
struct Profile {
    name: String,
    application_id: Option<String>,
    api_key: Option<String>,
    admin_api_key: Option<String>,
    crawler_user_id: Option<String>,
    crawler_api_key: Option<String>,
}

fn algolia_env_assignments(contents: &str) -> Result<Vec<String>, String> {
    let profiles = algolia_profiles(contents)?;
    let secret_profiles = profiles
        .iter()
        .filter(|profile| profile_contains_secret(profile))
        .collect::<Vec<_>>();
    if secret_profiles.is_empty() {
        return Ok(Vec::new());
    }
    if profiles.len() > 1 {
        return Err(
            "Algolia config has multiple profiles; env vars would override profile selection"
                .to_string(),
        );
    }

    secret_profiles[0].env_assignments()
}

impl Profile {
    fn env_assignments(&self) -> Result<Vec<String>, String> {
        let mut assignments = Vec::new();
        if self.api_key.is_some() || self.admin_api_key.is_some() {
            let application_id = self.application_id.as_deref().ok_or_else(|| {
                format!(
                    "Algolia profile [{}] has an API key but no application_id",
                    self.name
                )
            })?;
            reject_env_line_breaks("ALGOLIA_APPLICATION_ID", application_id)?;
            assignments.push(format!("ALGOLIA_APPLICATION_ID={application_id}"));
        }
        if let Some(api_key) = &self.api_key {
            reject_env_line_breaks("ALGOLIA_API_KEY", api_key)?;
            assignments.push(format!("ALGOLIA_API_KEY={api_key}"));
        }
        if let Some(admin_api_key) = &self.admin_api_key {
            reject_env_line_breaks("ALGOLIA_ADMIN_API_KEY", admin_api_key)?;
            assignments.push(format!("ALGOLIA_ADMIN_API_KEY={admin_api_key}"));
        }
        if let Some(crawler_api_key) = &self.crawler_api_key {
            let crawler_user_id = self.crawler_user_id.as_deref().ok_or_else(|| {
                format!(
                    "Algolia profile [{}] has a crawler API key but no crawler_user_id",
                    self.name
                )
            })?;
            reject_env_line_breaks("ALGOLIA_CRAWLER_USER_ID", crawler_user_id)?;
            reject_env_line_breaks("ALGOLIA_CRAWLER_API_KEY", crawler_api_key)?;
            assignments.push(format!("ALGOLIA_CRAWLER_USER_ID={crawler_user_id}"));
            assignments.push(format!("ALGOLIA_CRAWLER_API_KEY={crawler_api_key}"));
        }
        Ok(assignments)
    }
}

fn profile_contains_secret(profile: &Profile) -> bool {
    profile.api_key.is_some()
        || profile.admin_api_key.is_some()
        || profile.crawler_api_key.is_some()
}

fn algolia_profiles(contents: &str) -> Result<Vec<Profile>, String> {
    let mut profiles = Vec::new();
    let mut current = Profile::default();

    for line in contents.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if let Some(section) = section_name(line) {
            if !current.name.is_empty() {
                profiles.push(current);
            }
            current = Profile {
                name: section.to_string(),
                ..Profile::default()
            };
            continue;
        }

        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim();
        let value = toml_string_value(value).unwrap_or_default();
        if value.is_empty() {
            continue;
        }
        match name {
            "application_id" => current.application_id = Some(value),
            "api_key" => current.api_key = Some(value),
            "admin_api_key" => current.admin_api_key = Some(value),
            "crawler_user_id" => current.crawler_user_id = Some(value),
            "crawler_api_key" => current.crawler_api_key = Some(value),
            _ => {}
        }
    }

    if !current.name.is_empty() {
        profiles.push(current);
    }
    Ok(profiles)
}

fn section_name(line: &str) -> Option<&str> {
    line.strip_prefix('[')?.strip_suffix(']').map(str::trim)
}

fn sanitized_config_toml(contents: &str) -> String {
    let mut output = Vec::new();
    for line in contents.lines() {
        output.push(sanitized_line(line));
    }

    let mut rendered = output.join("\n");
    if contents.ends_with('\n') {
        rendered.push('\n');
    }
    rendered
}

fn sanitized_line(line: &str) -> String {
    let uncommented = line.split('#').next().unwrap_or("").trim();
    let Some((name, value)) = uncommented.split_once('=') else {
        return line.to_string();
    };
    if !matches!(name.trim(), "api_key" | "admin_api_key" | "crawler_api_key")
        || toml_string_value(value).is_none_or(|value| value.is_empty())
    {
        return line.to_string();
    }

    let leading = line
        .chars()
        .take_while(|character| character.is_whitespace())
        .collect::<String>();
    format!("{leading}{} = \"\"", name.trim())
}

fn reject_env_line_breaks(key: &str, value: &str) -> Result<(), String> {
    if value.contains('\n') || value.contains('\r') {
        return Err(format!("{key} cannot contain line breaks"));
    }
    Ok(())
}

fn toml_string_field_is_present(contents: &str, field: &str) -> bool {
    contents.lines().any(|line| {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some((name, value)) = line.split_once('=') else {
            return false;
        };
        name.trim() == field && toml_string_value(value).is_some_and(|value| !value.is_empty())
    })
}

fn toml_string_value(value: &str) -> Option<String> {
    let value = value.trim();
    let quote = value.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let mut escaped = false;
    let mut output = String::new();
    for character in value[quote.len_utf8()..].chars() {
        if escaped {
            output.push(character);
            escaped = false;
            continue;
        }
        match character {
            '\\' if quote == '"' => escaped = true,
            character if character == quote => return Some(output),
            _ => output.push(character),
        }
    }
    None
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
    Err(format!("failed to store secret {account}: {message}"))
}

#[cfg(any(not(target_os = "macos"), test, coverage))]
fn keychain_store_secret(_service: &str, _account: &str, _value: &str) -> Result<(), String> {
    Err("Automic Vault secret storage is only available on macOS".to_string())
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
    fn migrates_algolia_config_file() {
        let path = std::env::temp_dir().join(format!("algolia-config-{}", std::process::id()));
        let contents = "\
[default]
application_id = \"APPID\"
api_key = \"fake-key\"
admin_api_key = \"fake-admin-key\"
crawler_user_id = \"crawler-user\"
crawler_api_key = \"fake-crawler-key\"
";
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_config_file(&path, &store).unwrap());

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                ALGOLIA_ENV_ASSIGNMENTS_KEY.to_string(),
                "ALGOLIA_APPLICATION_ID=APPID\nALGOLIA_API_KEY=fake-key\nALGOLIA_ADMIN_API_KEY=fake-admin-key\nALGOLIA_CRAWLER_USER_ID=crawler-user\nALGOLIA_CRAWLER_API_KEY=fake-crawler-key".to_string()
            )]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "[default]\napplication_id = \"APPID\"\napi_key = \"\"\nadmin_api_key = \"\"\ncrawler_user_id = \"crawler-user\"\ncrawler_api_key = \"\"\n"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_multi_profile_configs() {
        let contents = "\
[default]
application_id = \"APPID\"
api_key = \"fake-key\"
[other]
application_id = \"OTHER\"
";

        let err = algolia_env_assignments(contents).unwrap_err();

        assert!(err.contains("multiple profiles"));
    }

    #[test]
    fn rejects_api_key_without_application_id() {
        let err = algolia_env_assignments("[default]\napi_key = \"fake-key\"\n").unwrap_err();

        assert!(err.contains("application_id"));
    }

    #[test]
    fn rejects_crawler_key_without_user_id() {
        let err =
            algolia_env_assignments("[default]\ncrawler_api_key = \"fake-key\"\n").unwrap_err();

        assert!(err.contains("crawler_user_id"));
    }

    #[test]
    fn does_not_migrate_without_api_key() {
        let path = std::env::temp_dir().join(format!("algolia-no-key-{}", std::process::id()));
        fs::write(&path, "[default]\napplication_id = \"APPID\"\n").unwrap();
        let store = TestCredentialStore::default();

        assert!(!migrate_config_file(&path, &store).unwrap());
        assert!(store.values.borrow().is_empty());
        fs::remove_file(path).unwrap();
    }
}
