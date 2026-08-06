#[cfg(all(target_os = "macos", not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const LUAROCKS_API_KEY_ENV_KEY: &str = "LUAROCKS_API_KEY";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[LUAROCKS_API_KEY_ENV_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_upload_configs(&upload_config_paths()?, &KeychainCredentialStore).map(|_| ())
}

pub fn migrate_upload_configs(
    paths: &[PathBuf],
    store: &dyn CredentialStore,
) -> Result<bool, String> {
    let mut migrations = Vec::new();
    let mut api_key: Option<String> = None;

    for path in paths {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
        };
        let Some(migration) = upload_config_migration(&contents)? else {
            continue;
        };

        match api_key.as_deref() {
            Some(existing) if existing != migration.api_key => {
                return Err(
                    "multiple distinct LuaRocks upload API keys found; migrate them manually"
                        .to_string(),
                );
            }
            None => api_key = Some(migration.api_key.clone()),
            _ => {}
        }
        migrations.push((path.clone(), migration.rewritten));
    }

    let Some(api_key) = api_key else {
        return Ok(false);
    };

    store.store_secret(LUAROCKS_API_KEY_ENV_KEY, &api_key)?;
    for (path, rewritten) in migrations {
        fs::write(&path, rewritten)
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    }
    Ok(true)
}

#[derive(Debug)]
struct UploadConfigMigration {
    api_key: String,
    rewritten: String,
}

fn upload_config_migration(contents: &str) -> Result<Option<UploadConfigMigration>, String> {
    let mut api_key: Option<String> = None;
    let mut rewritten = String::new();
    let mut changed = false;

    for line in contents.lines() {
        let Some(assignment) = parse_key_assignment(line) else {
            push_line(&mut rewritten, line);
            continue;
        };

        match api_key.as_deref() {
            Some(existing) if existing != assignment.value => {
                return Err(
                    "multiple distinct LuaRocks upload API keys found in one config".to_string(),
                );
            }
            None => api_key = Some(assignment.value),
            _ => {}
        }

        rewritten.push_str(&line[..assignment.value_start]);
        rewritten.push_str("nil");
        rewritten.push_str(&line[assignment.value_end..]);
        rewritten.push('\n');
        changed = true;
    }

    if !changed {
        return Ok(None);
    }

    Ok(Some(UploadConfigMigration {
        api_key: api_key.expect("changed LuaRocks upload config without API key"),
        rewritten,
    }))
}

fn upload_config_paths() -> Result<Vec<PathBuf>, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    let mut paths = Vec::new();

    for (key, value) in std::env::vars_os() {
        let Some(key) = key.to_str() else {
            continue;
        };
        if key == "LUAROCKS_CONFIG" || key.starts_with("LUAROCKS_CONFIG_") {
            if !value.is_empty() {
                paths.push(upload_config_path_for_user_config(Path::new(&value)));
            }
        }
    }

    if let Some(xdg_config_home) =
        std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty())
    {
        paths.push(PathBuf::from(xdg_config_home).join("luarocks/upload_config.lua"));
    } else {
        paths.push(home.join(".config/luarocks/upload_config.lua"));
    }
    paths.push(home.join(".luarocks/upload_config.lua"));

    Ok(dedupe_paths(paths))
}

fn upload_config_path_for_user_config(path: &Path) -> PathBuf {
    path.parent()
        .map(|parent| parent.join("upload_config.lua"))
        .unwrap_or_else(|| PathBuf::from("upload_config.lua"))
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut deduped = Vec::new();
    for path in paths {
        if !deduped.iter().any(|existing| existing == &path) {
            deduped.push(path);
        }
    }
    deduped
}

#[derive(Debug, PartialEq, Eq)]
struct KeyAssignment {
    value_start: usize,
    value_end: usize,
    value: String,
}

fn parse_key_assignment(line: &str) -> Option<KeyAssignment> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("--") {
        return None;
    }

    let equals = line.find('=')?;
    let key_side = line[..equals].trim_end();
    if !key_side_names_key(key_side) {
        return None;
    }

    let value_prefix = &line[equals + 1..];
    let whitespace = value_prefix.len() - value_prefix.trim_start().len();
    let quote_start = equals + 1 + whitespace;
    let quote = line.as_bytes().get(quote_start).copied()?;
    if quote != b'\'' && quote != b'"' {
        return None;
    }

    let mut escaped = false;
    for (offset, byte) in line[quote_start + 1..].bytes().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if byte == b'\\' {
            escaped = true;
            continue;
        }
        if byte == quote {
            let value_start = quote_start;
            let value_end = quote_start + 1 + offset + 1;
            let value = line[quote_start + 1..quote_start + 1 + offset].to_string();
            if value.is_empty() {
                return None;
            }
            return Some(KeyAssignment {
                value_start,
                value_end,
                value,
            });
        }
    }
    None
}

fn key_side_names_key(key_side: &str) -> bool {
    let key_side = key_side.trim_end();
    if key_side.ends_with("[\"key\"]") || key_side.ends_with("['key']") {
        return true;
    }
    if !key_side.ends_with("key") {
        return false;
    }
    key_side
        .chars()
        .rev()
        .nth(3)
        .map(|previous| !matches!(previous, '_' | '-' | 'a'..='z' | 'A'..='Z' | '0'..='9'))
        .unwrap_or(true)
}

fn push_line(output: &mut String, line: &str) {
    output.push_str(line);
    output.push('\n');
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
    fn keys_include_luarocks_api_key() {
        assert_eq!(keys(), &[LUAROCKS_API_KEY_ENV_KEY]);
    }

    #[test]
    fn rewrites_upload_config_key_to_nil() {
        let migration = upload_config_migration("return { key = \"lr_secret\", server = \"x\" }\n")
            .unwrap()
            .unwrap();

        assert_eq!(migration.api_key, "lr_secret");
        assert_eq!(
            migration.rewritten,
            "return { key = nil, server = \"x\" }\n"
        );
    }

    #[test]
    fn migrates_all_upload_configs_with_same_key() {
        let temp = test_dir("luarocks-migrate");
        let first = temp.join("one/upload_config.lua");
        let second = temp.join("two/upload_config.lua");
        fs::create_dir_all(first.parent().unwrap()).unwrap();
        fs::create_dir_all(second.parent().unwrap()).unwrap();
        fs::write(&first, "return {\n   key = \"lr_secret\",\n}\n").unwrap();
        fs::write(&second, "key = 'lr_secret'\n").unwrap();
        let store = TestCredentialStore::default();

        migrate_upload_configs(&[first.clone(), second.clone()], &store).unwrap();

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                LUAROCKS_API_KEY_ENV_KEY.to_string(),
                "lr_secret".to_string()
            )]
        );
        assert!(fs::read_to_string(first).unwrap().contains("key = nil"));
        assert_eq!(fs::read_to_string(second).unwrap(), "key = nil\n");
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn rejects_multiple_distinct_upload_keys() {
        let temp = test_dir("luarocks-multiple");
        let first = temp.join("one.lua");
        let second = temp.join("two.lua");
        fs::write(&first, "key = \"one\"\n").unwrap();
        fs::write(&second, "key = \"two\"\n").unwrap();
        let store = TestCredentialStore::default();

        let err = migrate_upload_configs(&[first, second], &store).unwrap_err();

        assert!(err.contains("multiple distinct"));
        fs::remove_dir_all(temp).unwrap();
    }

    fn test_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
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
