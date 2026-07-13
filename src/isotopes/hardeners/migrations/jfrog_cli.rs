#[cfg(all(target_os = "macos", not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const JFROG_ENV_ASSIGNMENTS_KEY: &str = "JFROG_ENV_ASSIGNMENTS";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[JFROG_ENV_ASSIGNMENTS_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_credentials_file(&jfrog_config_path()?, &KeychainCredentialStore).map(|_| ())
}

pub fn migrate_credentials_file(path: &Path, store: &dyn CredentialStore) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    if !config_contains_secret(&contents) {
        return Ok(false);
    }
    let migration = config_migration(&contents)?;
    if !migration.changed {
        return Ok(false);
    }

    store.store_secret(JFROG_ENV_ASSIGNMENTS_KEY, &migration.assignments.join("\n"))?;
    fs::write(path, migration.sanitized)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn jfrog_config_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".jfrog/jfrog-cli.conf.v6"))
}

fn config_contains_secret(contents: &str) -> bool {
    ["password", "accessToken", "refreshToken", "sshPassphrase"]
        .iter()
        .any(|field| json_string_field(contents, field).is_some_and(|value| !value.is_empty()))
}

#[derive(Debug)]
struct ConfigMigration {
    sanitized: String,
    assignments: Vec<String>,
    changed: bool,
}

#[derive(Debug)]
struct ServerSecret {
    index: usize,
    assignments: Vec<String>,
}

fn config_migration(contents: &str) -> Result<ConfigMigration, String> {
    let mut value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|err| format!("failed to parse JFrog CLI config JSON: {err}"))?;
    let Some(servers) = value.as_array_mut() else {
        return Err("JFrog CLI config must be a JSON array".to_string());
    };

    let mut secret_servers = Vec::new();
    for (index, server) in servers.iter().enumerate() {
        if let Some(secret) = server_secret(index, server)? {
            secret_servers.push(secret);
        }
    }

    if secret_servers.is_empty() {
        return Ok(ConfigMigration {
            sanitized: contents.to_string(),
            assignments: Vec::new(),
            changed: false,
        });
    }
    if secret_servers.len() > 1 {
        return Err(
            "JFrog CLI configs with credentials for multiple servers must be migrated manually"
                .to_string(),
        );
    }

    let secret = secret_servers.remove(0);
    let server = servers
        .get_mut(secret.index)
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| "JFrog CLI server config must be a JSON object".to_string())?;
    for key in ["accessToken", "password"] {
        if let Some(value) = server.get_mut(key) {
            *value = serde_json::Value::String(String::new());
        }
    }

    let mut sanitized = serde_json::to_string_pretty(&value)
        .map_err(|err| format!("failed to render JFrog CLI config JSON: {err}"))?;
    sanitized.push('\n');
    Ok(ConfigMigration {
        sanitized,
        assignments: secret.assignments,
        changed: true,
    })
}

fn server_secret(index: usize, server: &serde_json::Value) -> Result<Option<ServerSecret>, String> {
    let access_token = json_value_string(server, "accessToken");
    let password = json_value_string(server, "password");
    let refresh_token = json_value_string(server, "refreshToken");
    let ssh_passphrase = json_value_string(server, "sshPassphrase");

    if refresh_token.is_some() || ssh_passphrase.is_some() {
        return Err(
            "JFrog CLI refreshToken and sshPassphrase configs must be migrated manually"
                .to_string(),
        );
    }
    if access_token.is_none() && password.is_none() {
        return Ok(None);
    }
    if access_token.is_some() && password.is_some() {
        return Err(
            "JFrog CLI configs with both accessToken and password must be migrated manually"
                .to_string(),
        );
    }

    let url = required_json_string(server, "url")?;
    let mut assignments = vec![format!("JFROG_URL={url}")];
    if let Some(token) = access_token {
        assignments.push(format!("JFROG_ACCESS_TOKEN={token}"));
    } else if let Some(password) = password {
        let user = required_json_string(server, "user")?;
        assignments.push(format!("JFROG_USER={user}"));
        assignments.push(format!("JFROG_PASSWORD={password}"));
    }
    for assignment in &assignments {
        if assignment.contains('\n') || assignment.contains('\r') {
            return Err("JFrog CLI env assignments cannot contain line breaks".to_string());
        }
    }
    Ok(Some(ServerSecret { index, assignments }))
}

fn required_json_string(value: &serde_json::Value, field: &str) -> Result<String, String> {
    json_value_string(value, field)
        .ok_or_else(|| format!("JFrog CLI credential configs require non-empty {field}"))
}

fn json_value_string(value: &serde_json::Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn json_string_field<'a>(contents: &'a str, field: &str) -> Option<&'a str> {
    let quoted = format!("\"{field}\"");
    let after_key = contents.split(&quoted).nth(1)?.split_once(':')?.1;
    after_key
        .trim_start()
        .strip_prefix('"')?
        .split_once('"')
        .map(|(value, _)| value)
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
    fn migrates_single_access_token_server_to_env() {
        let path = std::env::temp_dir().join(format!("jfrog-config-{}", std::process::id()));
        let contents = "[{\"serverId\":\"prod\",\"url\":\"https://example.jfrog.io\",\"accessToken\":\"secret\"}]\n";
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        migrate_credentials_file(&path, &store).unwrap();

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                JFROG_ENV_ASSIGNMENTS_KEY.to_string(),
                "JFROG_URL=https://example.jfrog.io\nJFROG_ACCESS_TOKEN=secret".to_string()
            )]
        );
        let sanitized = fs::read_to_string(&path).unwrap();
        assert!(sanitized.contains(r#""serverId": "prod""#));
        assert!(sanitized.contains(r#""url": "https://example.jfrog.io""#));
        assert!(sanitized.contains(r#""accessToken": """#));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn migrates_single_basic_auth_server_to_env() {
        let migration = config_migration(
            r#"[{"serverId":"prod","url":"https://example.jfrog.io","user":"u","password":"p"}]"#,
        )
        .unwrap();

        assert_eq!(
            migration.assignments,
            vec![
                "JFROG_URL=https://example.jfrog.io",
                "JFROG_USER=u",
                "JFROG_PASSWORD=p",
            ]
        );
        assert!(migration.sanitized.contains(r#""password": """#));
    }

    #[test]
    fn rejects_refresh_tokens_ssh_passphrases_and_multi_server_secrets() {
        let refresh = config_migration(
            r#"[{"serverId":"prod","url":"https://example.jfrog.io","refreshToken":"secret"}]"#,
        )
        .unwrap_err();
        assert!(refresh.contains("refreshToken"));

        let ssh = config_migration(
            r#"[{"serverId":"prod","url":"https://example.jfrog.io","sshPassphrase":"secret"}]"#,
        )
        .unwrap_err();
        assert!(ssh.contains("sshPassphrase"));

        let multi = config_migration(
            r#"[{"serverId":"one","url":"https://one.example","accessToken":"one"},{"serverId":"two","url":"https://two.example","accessToken":"two"}]"#,
        )
        .unwrap_err();
        assert!(multi.contains("multiple servers"));
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
    fn keys_default_path_and_parser_helpers_cover_edge_cases() {
        assert_eq!(keys(), &[JFROG_ENV_ASSIGNMENTS_KEY]);
        assert!(config_contains_secret("[{\"accessToken\":\"secret\"}]"));
        assert!(config_contains_secret("[{\"refreshToken\":\"secret\"}]"));
        assert!(!config_contains_secret("[{\"accessToken\":\"\"}]"));
        assert_eq!(
            json_string_field("[{\"password\":\"secret\"}]", "password"),
            Some("secret")
        );
        assert_eq!(json_string_field("[{\"password\":null}]", "password"), None);

        let previous_home = std::env::var_os("HOME");
        unsafe { std::env::remove_var("HOME") };
        let err = jfrog_config_path().unwrap_err();
        assert!(err.contains("HOME is not set"));
        match previous_home {
            Some(value) => unsafe { std::env::set_var("HOME", value) },
            None => unsafe { std::env::remove_var("HOME") },
        }
    }

    #[test]
    fn migrate_credentials_ignores_missing_and_secretless_files() {
        let temp = std::env::temp_dir();
        let missing = temp.join(format!("jfrog-missing-{}", std::process::id()));
        let blank = temp.join(format!("jfrog-blank-{}", std::process::id()));
        let store = TestCredentialStore::default();

        assert!(!migrate_credentials_file(&missing, &store).unwrap());
        fs::write(&blank, "[{\"serverId\":\"prod\",\"accessToken\":\"\"}]\n").unwrap();
        assert!(!migrate_credentials_file(&blank, &store).unwrap());
        assert!(store.values.borrow().is_empty());
        fs::remove_file(blank).unwrap();
    }

    #[test]
    fn migrate_credentials_preserves_file_when_store_fails() {
        let path = std::env::temp_dir().join(format!("jfrog-store-failure-{}", std::process::id()));
        let contents = "[{\"serverId\":\"prod\",\"url\":\"https://example.jfrog.io\",\"accessToken\":\"secret\"}]\n";
        fs::write(&path, contents).unwrap();

        let err = migrate_credentials_file(&path, &FailingCredentialStore).unwrap_err();

        assert!(err.contains("store failed"));
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);
        fs::remove_file(path).unwrap();
    }
}
