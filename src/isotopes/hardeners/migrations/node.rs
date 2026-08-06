#[cfg(all(target_os = "macos", not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const NPM_TOKEN_ENV_KEY: &str = "NODE_AUTH_TOKEN";
const NPM_TOKEN_PLACEHOLDER: &str = "${NODE_AUTH_TOKEN}";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[NPM_TOKEN_ENV_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_credentials_file(&npm_user_config_path()?, &KeychainCredentialStore).map(|_| ())
}

pub fn migrate_credentials_file(path: &Path, store: &dyn CredentialStore) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    let migration = npmrc_token_migration(&contents)?;
    let Some(migration) = migration else {
        return Ok(false);
    };

    store.store_secret(NPM_TOKEN_ENV_KEY, &migration.token)?;
    fs::write(path, migration.rewritten)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn npm_user_config_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("NPM_CONFIG_USERCONFIG").filter(|value| !value.is_empty())
    {
        return Ok(PathBuf::from(path));
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".npmrc"))
}

#[derive(Debug)]
struct NpmTokenMigration {
    token: String,
    rewritten: String,
}

fn npmrc_token_migration(contents: &str) -> Result<Option<NpmTokenMigration>, String> {
    let mut token: Option<String> = None;
    let mut changed = false;
    let mut rewritten = String::new();

    for line in contents.lines() {
        let Some((key, value)) = parse_auth_token_line(line) else {
            push_line(&mut rewritten, line);
            continue;
        };
        if !auth_token_value_is_plaintext(value) {
            push_line(&mut rewritten, line);
            continue;
        }

        match token.as_deref() {
            Some(existing) if existing != value => {
                return Err(
                    "multiple distinct npm auth tokens found; migrate them manually".to_string(),
                );
            }
            None => token = Some(value.to_string()),
            _ => {}
        }

        push_line(&mut rewritten, &format!("{key}={NPM_TOKEN_PLACEHOLDER}"));
        changed = true;
    }

    if !changed {
        return Ok(None);
    }
    let token = token.expect("changed npm token migration without token");
    Ok(Some(NpmTokenMigration { token, rewritten }))
}

fn parse_auth_token_line(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
        return None;
    }

    let (key, value) = trimmed.split_once('=')?;
    let key = key.trim();
    if !key.ends_with(":_authToken") && key != "_authToken" {
        return None;
    }

    Some((key, value.trim()))
}

fn auth_token_value_is_plaintext(value: &str) -> bool {
    !value.is_empty() && value != NPM_TOKEN_PLACEHOLDER
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
    fn keys_include_node_auth_token() {
        assert_eq!(keys(), &[NPM_TOKEN_ENV_KEY]);
    }

    #[test]
    fn migrates_npmrc_token_to_keychain_placeholder() {
        let path = std::env::temp_dir().join(format!("node-migrate-npmrc-{}", std::process::id()));
        fs::write(
            &path,
            "\
registry=https://registry.npmjs.org/
//registry.npmjs.org/:_authToken=npm_secret
",
        )
        .unwrap();
        let store = TestCredentialStore::default();

        migrate_credentials_file(&path, &store).unwrap();

        assert_eq!(
            store.values.into_inner(),
            vec![(NPM_TOKEN_ENV_KEY.to_string(), "npm_secret".to_string())]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "\
registry=https://registry.npmjs.org/
//registry.npmjs.org/:_authToken=${NODE_AUTH_TOKEN}
"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn migration_ignores_existing_placeholder() {
        let contents = "//registry.npmjs.org/:_authToken=${NODE_AUTH_TOKEN}\n";

        assert!(npmrc_token_migration(contents).unwrap().is_none());
    }

    #[test]
    fn migration_rejects_multiple_distinct_tokens() {
        let contents = "\
//registry.npmjs.org/:_authToken=first
//registry.example.com/:_authToken=second
";

        assert_eq!(
            npmrc_token_migration(contents).unwrap_err(),
            "multiple distinct npm auth tokens found; migrate them manually"
        );
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
