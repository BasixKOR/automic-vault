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
    let Some(migration) = npmrc_token_migration(&contents)? else {
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
    use std::ffi::OsString;

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

    struct EnvGuard {
        key: &'static str,
        previous: Option<OsString>,
    }

    impl EnvGuard {
        fn set_path(key: &'static str, value: Option<&Path>) -> Self {
            let previous = std::env::var_os(key);
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
            Self { key, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe {
                match self.previous.take() {
                    Some(value) => std::env::set_var(self.key, value),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }

    #[test]
    fn migrates_npmrc_token_to_keychain_placeholder() {
        let path = std::env::temp_dir().join(format!("pnpm-npmrc-{}", std::process::id()));
        fs::write(
            &path,
            "registry=https://registry.npmjs.org/\n//registry.npmjs.org/:_authToken=npm_secret\n",
        )
        .unwrap();
        let store = TestCredentialStore::default();

        migrate_credentials_file(&path, &store).unwrap();

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(NPM_TOKEN_ENV_KEY.to_string(), "npm_secret".to_string())]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "registry=https://registry.npmjs.org/\n//registry.npmjs.org/:_authToken=${NODE_AUTH_TOKEN}\n"
        );
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
    fn path_helpers_prefer_userconfig_and_require_home() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = test_dir("pnpm-home");
        let userconfig = home.join("custom.npmrc");
        {
            let _userconfig = EnvGuard::set_path("NPM_CONFIG_USERCONFIG", Some(&userconfig));
            let _home = EnvGuard::set_path("HOME", Some(&home));
            assert_eq!(npm_user_config_path().unwrap(), userconfig);
            assert_eq!(keys(), &[NPM_TOKEN_ENV_KEY]);
        }
        {
            let _userconfig = EnvGuard::set_path("NPM_CONFIG_USERCONFIG", None);
            let _home = EnvGuard::set_path("HOME", Some(&home));
            assert_eq!(npm_user_config_path().unwrap(), home.join(".npmrc"));
        }
        {
            let _userconfig = EnvGuard::set_path("NPM_CONFIG_USERCONFIG", None);
            let _home = EnvGuard::set_path("HOME", None);
            assert_eq!(npm_user_config_path().unwrap_err(), "HOME is not set");
        }
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn npmrc_parser_covers_comments_placeholders_duplicates_and_helpers() {
        assert_eq!(parse_auth_token_line(""), None);
        assert_eq!(parse_auth_token_line("# _authToken=secret"), None);
        assert_eq!(parse_auth_token_line("; _authToken=secret"), None);
        assert_eq!(parse_auth_token_line("registry=https://example.test"), None);
        assert_eq!(
            parse_auth_token_line(" //registry.npmjs.org/:_authToken = secret "),
            Some(("//registry.npmjs.org/:_authToken", "secret"))
        );
        assert!(!auth_token_value_is_plaintext(""));
        assert!(!auth_token_value_is_plaintext(NPM_TOKEN_PLACEHOLDER));
        assert!(auth_token_value_is_plaintext("secret"));

        assert!(
            npmrc_token_migration("_authToken=${NODE_AUTH_TOKEN}\n")
                .unwrap()
                .is_none()
        );
        let migration =
            npmrc_token_migration("_authToken=secret\n//registry.npmjs.org/:_authToken=secret\n")
                .unwrap()
                .unwrap();
        assert_eq!(migration.token, "secret");
        assert_eq!(
            migration.rewritten,
            "_authToken=${NODE_AUTH_TOKEN}\n//registry.npmjs.org/:_authToken=${NODE_AUTH_TOKEN}\n"
        );
        assert!(
            npmrc_token_migration("_authToken=one\n//registry.npmjs.org/:_authToken=two\n")
                .unwrap_err()
                .contains("multiple distinct npm auth tokens")
        );

        let mut rewritten = String::new();
        push_line(&mut rewritten, "line");
        assert_eq!(rewritten, "line\n");
    }

    #[test]
    fn reports_read_errors_and_preserves_npmrc_on_store_failure() {
        let temp = test_dir("pnpm-errors");
        let dir_path = temp.join("dir");
        fs::create_dir_all(&dir_path).unwrap();
        assert!(
            migrate_credentials_file(&dir_path, &TestCredentialStore::default())
                .unwrap_err()
                .contains("failed to read")
        );

        let path = temp.join(".npmrc");
        let contents = "_authToken=secret\n";
        fs::write(&path, contents).unwrap();
        assert_eq!(
            migrate_credentials_file(&path, &FailingCredentialStore).unwrap_err(),
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
