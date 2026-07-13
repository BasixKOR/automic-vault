#[cfg(all(target_os = "macos", not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const HEROKU_API_KEY_ENV_KEY: &str = "HEROKU_API_KEY";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[HEROKU_API_KEY_ENV_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_credentials_file(&netrc_path()?, &KeychainCredentialStore).map(|_| ())
}

pub fn migrate_credentials_file(path: &Path, store: &dyn CredentialStore) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    let Some(token) = heroku_netrc_token(&contents)? else {
        return Ok(false);
    };

    store.store_secret(HEROKU_API_KEY_ENV_KEY, &token)?;
    fs::write(path, remove_heroku_netrc_blocks(&contents))
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn netrc_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("NETRC").map(PathBuf::from) {
        return Ok(path);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".netrc"))
}

fn heroku_netrc_token(contents: &str) -> Result<Option<String>, String> {
    let mut unique = Vec::<String>::new();
    for token in heroku_netrc_tokens(contents) {
        if !unique.iter().any(|existing| existing == &token) {
            unique.push(token);
        }
    }
    match unique.len() {
        0 => Ok(None),
        1 => Ok(unique.pop()),
        _ => Err(
            "Heroku api.heroku.com and git.heroku.com netrc entries contain different tokens"
                .to_string(),
        ),
    }
}

fn heroku_netrc_tokens(contents: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut current_machine: Option<&str> = None;
    let tokens = netrc_tokens(contents);
    let mut index = 0;
    while index < tokens.len() {
        match tokens[index].as_str() {
            "machine" => {
                current_machine = tokens.get(index + 1).map(String::as_str);
                index += 2;
            }
            "default" => {
                current_machine = Some("default");
                index += 1;
            }
            "password" => {
                if matches!(current_machine, Some("api.heroku.com" | "git.heroku.com")) {
                    if let Some(token) = tokens.get(index + 1) {
                        found.push(token.clone());
                    }
                }
                index += 2;
            }
            _ => index += 1,
        }
    }
    found
}

fn netrc_tokens(contents: &str) -> Vec<String> {
    contents
        .lines()
        .flat_map(|line| line.split('#').next().unwrap_or("").split_whitespace())
        .map(str::to_string)
        .collect()
}

fn remove_heroku_netrc_blocks(contents: &str) -> String {
    let mut output = Vec::new();
    let mut skipping = false;

    for line in contents.lines() {
        if let Some(machine) = machine_start(line) {
            skipping = matches!(machine, "api.heroku.com" | "git.heroku.com");
        } else if starts_default_entry(line) {
            skipping = false;
        }

        if !skipping {
            output.push(line);
        }
    }

    let mut joined = output.join("\n");
    joined.push('\n');
    joined
}

fn machine_start(line: &str) -> Option<&str> {
    let mut fields = line.split_whitespace();
    match (fields.next(), fields.next()) {
        (Some("machine"), Some(machine)) => Some(machine),
        _ => None,
    }
}

fn starts_default_entry(line: &str) -> bool {
    matches!(line.split_whitespace().next(), Some("default"))
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
    fn migrates_heroku_netrc_token_and_preserves_other_machines() {
        let path = test_path("heroku-netrc");
        fs::write(
            &path,
            "machine example.com login a password keep\nmachine api.heroku.com login b password token\nmachine git.heroku.com login b password token\n",
        )
        .unwrap();
        let store = TestCredentialStore::default();

        migrate_credentials_file(&path, &store).unwrap();

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(HEROKU_API_KEY_ENV_KEY.to_string(), "token".to_string())]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "machine example.com login a password keep\n"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn migrates_multiline_heroku_netrc_block() {
        let path = test_path("heroku-netrc-multiline");
        fs::write(
            &path,
            "machine api.heroku.com\n  login b\n  password token\nmachine example.com login a password keep\n",
        )
        .unwrap();
        let store = TestCredentialStore::default();

        migrate_credentials_file(&path, &store).unwrap();

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(HEROKU_API_KEY_ENV_KEY.to_string(), "token".to_string())]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "machine example.com login a password keep\n"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_distinct_api_and_git_tokens() {
        let err = heroku_netrc_token(
            "machine api.heroku.com login a password one\nmachine git.heroku.com login a password two\n",
        )
        .unwrap_err();

        assert!(err.contains("different tokens"));
    }

    fn test_path(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
        let _ = fs::remove_file(&path);
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
