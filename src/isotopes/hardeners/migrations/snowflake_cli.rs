#[cfg(all(target_os = "macos", not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const SNOWFLAKE_ENV_ASSIGNMENTS_KEY: &str = "SNOWFLAKE_ENV_ASSIGNMENTS";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[SNOWFLAKE_ENV_ASSIGNMENTS_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_default_configs(&KeychainCredentialStore).map(|_| ())
}

fn migrate_default_configs(store: &dyn CredentialStore) -> Result<bool, String> {
    let candidates = candidate_directories()?;
    let mut matches = Vec::new();

    for dir in candidates {
        let bundle = load_bundle(&dir)?;
        if bundle.has_sensitive_values() {
            matches.push(bundle);
        }
    }

    if matches.is_empty() {
        return Ok(false);
    }
    if matches.len() > 1 {
        let joined = matches
            .iter()
            .map(|bundle| bundle.dir.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "multiple Snowflake CLI config directories contain plaintext \
credentials; migrate them manually: {joined}"
        ));
    }

    migrate_bundle(matches.remove(0), store)
}

fn candidate_directories() -> Result<Vec<PathBuf>, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(vec![
        home.join(".snowflake"),
        home.join("Library/Application Support/snowflake"),
        home.join(".config/snowflake"),
    ])
}

#[derive(Debug)]
struct ConfigBundle {
    dir: PathBuf,
    config: Option<FileState>,
    connections: Option<FileState>,
}

#[derive(Debug)]
struct FileState {
    path: PathBuf,
    sanitized: String,
    changed: bool,
    assignments: Vec<String>,
}

impl ConfigBundle {
    fn has_sensitive_values(&self) -> bool {
        self.config.as_ref().is_some_and(|state| state.changed)
            || self.connections.as_ref().is_some_and(|state| state.changed)
    }

    fn assignments(&self) -> Vec<String> {
        let mut assignments = Vec::new();
        for state in [&self.config, &self.connections].into_iter().flatten() {
            for assignment in &state.assignments {
                if !assignments.iter().any(|existing| existing == assignment) {
                    assignments.push(assignment.clone());
                }
            }
        }
        assignments
    }
}

fn load_bundle(dir: &Path) -> Result<ConfigBundle, String> {
    Ok(ConfigBundle {
        dir: dir.to_path_buf(),
        config: load_file_state(&dir.join("config.toml"), ConfigFileKind::ConfigToml)?,
        connections: load_file_state(
            &dir.join("connections.toml"),
            ConfigFileKind::ConnectionsToml,
        )?,
    })
}

fn load_file_state(path: &Path, kind: ConfigFileKind) -> Result<Option<FileState>, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    let migration = file_migration(&contents, kind)?;
    Ok(Some(FileState {
        path: path.to_path_buf(),
        sanitized: migration.sanitized,
        changed: migration.changed,
        assignments: migration.assignments,
    }))
}

fn migrate_bundle(bundle: ConfigBundle, store: &dyn CredentialStore) -> Result<bool, String> {
    let assignments = bundle.assignments();
    if assignments.is_empty() {
        return Ok(false);
    }
    store.store_secret(SNOWFLAKE_ENV_ASSIGNMENTS_KEY, &assignments.join("\n"))?;

    if let Some(config) = bundle.config {
        if config.changed {
            fs::write(&config.path, config.sanitized)
                .map_err(|err| format!("failed to write {}: {err}", config.path.display()))?;
        }
    }
    if let Some(connections) = bundle.connections {
        if connections.changed {
            fs::write(&connections.path, connections.sanitized)
                .map_err(|err| format!("failed to write {}: {err}", connections.path.display()))?;
        }
    }

    Ok(true)
}

#[derive(Clone, Copy)]
enum ConfigFileKind {
    ConfigToml,
    ConnectionsToml,
}

#[derive(Debug)]
struct FileMigration {
    sanitized: String,
    changed: bool,
    assignments: Vec<String>,
}

fn file_migration(contents: &str, kind: ConfigFileKind) -> Result<FileMigration, String> {
    let mut changed = false;
    let mut output = Vec::new();
    let mut section = Section::Other;
    let mut assignments = Vec::new();

    for line in contents.lines() {
        if let Some(name) = section_name(line) {
            section = section_for_name(name, kind);
            output.push(line.to_string());
            continue;
        }

        let sanitized = migrate_line(line, &section, &mut changed, &mut assignments)?;
        output.push(sanitized);
    }

    if !changed {
        return Ok(FileMigration {
            sanitized: contents.to_string(),
            changed,
            assignments,
        });
    }

    let mut rendered = output.join("\n");
    if contents.ends_with('\n') {
        rendered.push('\n');
    }
    Ok(FileMigration {
        sanitized: rendered,
        changed,
        assignments,
    })
}

enum Section {
    Connection(String),
    Other,
}

fn section_name(line: &str) -> Option<&str> {
    line.trim()
        .strip_prefix('[')?
        .strip_suffix(']')
        .map(str::trim)
}

fn section_for_name(name: &str, kind: ConfigFileKind) -> Section {
    match kind {
        ConfigFileKind::ConfigToml => name
            .strip_prefix("connections.")
            .filter(|name| !name.is_empty())
            .map(|name| Section::Connection(name.to_string()))
            .unwrap_or(Section::Other),
        ConfigFileKind::ConnectionsToml => Section::Connection(name.to_string()),
    }
}

fn migrate_line(
    line: &str,
    section: &Section,
    changed: &mut bool,
    assignments: &mut Vec<String>,
) -> Result<String, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(line.to_string());
    }

    let Some((before_equals, after_equals)) = line.split_once('=') else {
        return Ok(line.to_string());
    };
    let key = before_equals.trim().to_ascii_lowercase();
    if key == "private_key_file_pwd" && toml_value_is_nonempty(after_equals) {
        return Err("Snowflake private_key_file_pwd configs must be migrated manually".to_string());
    }
    if key != "password" {
        return Ok(line.to_string());
    }

    let value = toml_string_value(after_equals).unwrap_or_else(|| after_equals.trim().to_string());
    if value.is_empty() {
        return Ok(line.to_string());
    }
    let Section::Connection(connection) = section else {
        return Err(
            "Snowflake password outside a connection section must be migrated manually".to_string(),
        );
    };
    let suffix = env_connection_suffix(connection)?;
    reject_env_line_breaks(&value)?;
    let assignment = format!("SNOWFLAKE_CONNECTIONS_{suffix}_PASSWORD={value}");
    if !assignments.iter().any(|existing| existing == &assignment) {
        assignments.push(assignment);
    }

    *changed = true;
    Ok(format!("{before_equals}= \"\""))
}

fn toml_value_is_nonempty(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && value != "\"\"" && value != "''"
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

fn env_connection_suffix(name: &str) -> Result<String, String> {
    if name.is_empty()
        || !name
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric())
    {
        return Err(format!(
            "Snowflake connection [{name}] cannot be represented as a safe environment variable"
        ));
    }
    Ok(name.to_ascii_uppercase())
}

fn reject_env_line_breaks(value: &str) -> Result<(), String> {
    if value.contains('\n') || value.contains('\r') {
        return Err("SNOWFLAKE password env vars cannot contain line breaks".to_string());
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
    fn blanks_password_fields_but_keeps_other_settings() {
        let contents = "\
[connections.default]\n\
user = \"jdoe\"\n\
password = \"secret\"\n\
warehouse = \"app\"\n";
        let migration = file_migration(contents, ConfigFileKind::ConfigToml).unwrap();

        assert!(migration.sanitized.contains("password = \"\""));
        assert!(migration.sanitized.contains("warehouse = \"app\""));
        assert_eq!(
            migration.assignments,
            vec!["SNOWFLAKE_CONNECTIONS_DEFAULT_PASSWORD=secret"]
        );
    }

    #[test]
    fn migrates_one_bundle_and_stores_both_files() {
        let temp = std::env::temp_dir().join(format!("snowflake-bundle-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        let config_path = temp.join("config.toml");
        let config_contents = "\
default_connection_name = \"default\"\n\
[connections.default]\n\
password = \"secret\"\n";
        fs::write(&config_path, config_contents).unwrap();
        let store = TestCredentialStore::default();
        let bundle = load_bundle(&temp).unwrap();

        migrate_bundle(bundle, &store).unwrap();

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                SNOWFLAKE_ENV_ASSIGNMENTS_KEY.to_string(),
                "SNOWFLAKE_CONNECTIONS_DEFAULT_PASSWORD=secret".to_string()
            )]
        );
        let sanitized = fs::read_to_string(&config_path).unwrap();
        assert!(sanitized.contains("password = \"\""));
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn rejects_private_key_passphrases_and_unsafe_connection_names() {
        let passphrase = file_migration(
            "[connections.default]\nprivate_key_file_pwd = \"secret\"\n",
            ConfigFileKind::ConfigToml,
        )
        .unwrap_err();
        assert!(passphrase.contains("private_key_file_pwd"));

        let unsafe_name = file_migration(
            "[connections.prod-west]\npassword = \"secret\"\n",
            ConfigFileKind::ConfigToml,
        )
        .unwrap_err();
        assert!(unsafe_name.contains("environment variable"));
    }

    #[test]
    fn maps_connections_toml_sections_to_connection_env_vars() {
        let migration = file_migration(
            "[prod]\npassword = \"secret\"\n",
            ConfigFileKind::ConnectionsToml,
        )
        .unwrap();

        assert_eq!(
            migration.assignments,
            vec!["SNOWFLAKE_CONNECTIONS_PROD_PASSWORD=secret"]
        );
        assert!(migration.sanitized.contains("password = \"\""));
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
