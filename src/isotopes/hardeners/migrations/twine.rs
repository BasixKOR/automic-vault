#[cfg(all(target_os = "macos", not(test), not(coverage)))]
use std::ffi::{CString, c_char};
use std::fs;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const TWINE_ENV_ASSIGNMENTS_KEY: &str = "TWINE_ENV_ASSIGNMENTS";

pub trait CredentialStore {
    fn store_secret(&self, key: &str, value: &str) -> Result<(), String>;
}

pub struct KeychainCredentialStore;

pub fn keys() -> &'static [&'static str] {
    &[TWINE_ENV_ASSIGNMENTS_KEY]
}

pub fn migrate_credentials() -> Result<(), String> {
    migrate_credentials_file(&pypirc_path()?, &KeychainCredentialStore).map(|_| ())
}

pub fn migrate_credentials_file(path: &Path, store: &dyn CredentialStore) -> Result<bool, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    let assignments = twine_env_assignments(&contents)?;
    if assignments.is_empty() {
        return Ok(false);
    }

    store.store_secret(TWINE_ENV_ASSIGNMENTS_KEY, &assignments.join("\n"))?;
    fs::write(path, sanitized_pypirc(&contents))
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(true)
}

fn pypirc_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".pypirc"))
}

fn sanitized_pypirc(contents: &str) -> String {
    let mut lines = Vec::new();
    for line in contents.lines() {
        lines.push(sanitize_line(line));
    }

    let mut rendered = lines.join("\n");
    if contents.ends_with('\n') {
        rendered.push('\n');
    }
    rendered
}

fn sanitize_line(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
        return line.to_string();
    }

    let Some((key, value)) = line.split_once('=') else {
        return line.to_string();
    };
    let normalized_key = key.trim().to_ascii_lowercase();
    if normalized_key == "password" && !value.trim().is_empty() {
        return format!("{}=", key);
    }
    if normalized_key == "repository" {
        let stripped = strip_url_userinfo(value.trim());
        if stripped != value.trim() {
            let prefix_len = line.len() - line.trim_start().len();
            let prefix = &line[..prefix_len];
            return format!("{prefix}{}= {stripped}", key.trim());
        }
    }

    line.to_string()
}

fn twine_env_assignments(contents: &str) -> Result<Vec<String>, String> {
    let mut credentials = Vec::new();
    for section in pypirc_sections(contents) {
        if let Some(credential) = section_credentials(&section)? {
            credentials.push(credential);
        }
    }

    match credentials.len() {
        0 => Ok(Vec::new()),
        1 => credentials
            .pop()
            .expect("one credential")
            .env_assignments()
            .map_err(|err| format!("Twine .pypirc cannot be migrated: {err}")),
        _ => Err(
            "Twine .pypirc contains credentials for multiple repositories; migrate them manually"
                .to_string(),
        ),
    }
}

#[derive(Default)]
struct PypircSection {
    name: String,
    username: Option<String>,
    password: Option<String>,
    repository: Option<String>,
    repository_userinfo: Option<RepositoryUserinfo>,
}

struct RepositoryUserinfo {
    username: Option<String>,
    password: Option<String>,
    sanitized_url: String,
}

struct TwineCredential {
    username: Option<String>,
    password: String,
    repository_url: String,
}

impl TwineCredential {
    fn env_assignments(self) -> Result<Vec<String>, String> {
        let mut assignments = Vec::new();
        if let Some(username) = self.username {
            reject_env_line_breaks("TWINE_USERNAME", &username)?;
            assignments.push(format!("TWINE_USERNAME={username}"));
        }
        reject_env_line_breaks("TWINE_PASSWORD", &self.password)?;
        assignments.push(format!("TWINE_PASSWORD={}", self.password));
        reject_env_line_breaks("TWINE_REPOSITORY_URL", &self.repository_url)?;
        assignments.push(format!("TWINE_REPOSITORY_URL={}", self.repository_url));
        Ok(assignments)
    }
}

fn pypirc_sections(contents: &str) -> Vec<PypircSection> {
    let mut sections = Vec::new();
    let mut current = PypircSection::default();

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if let Some(name) = section_name(trimmed) {
            if !current.name.is_empty() {
                sections.push(current);
            }
            current = PypircSection {
                name: name.to_string(),
                ..PypircSection::default()
            };
            continue;
        }

        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let value = value.trim().to_string();
        match key.trim().to_ascii_lowercase().as_str() {
            "username" if !value.is_empty() => current.username = Some(value),
            "password" if !value.is_empty() => current.password = Some(value),
            "repository" if !value.is_empty() => {
                current.repository_userinfo = repository_userinfo(&value);
                current.repository = Some(strip_url_userinfo(&value));
            }
            _ => {}
        }
    }

    if !current.name.is_empty() {
        sections.push(current);
    }
    sections
}

fn section_name(trimmed: &str) -> Option<&str> {
    trimmed.strip_prefix('[')?.strip_suffix(']').map(str::trim)
}

fn section_credentials(section: &PypircSection) -> Result<Option<TwineCredential>, String> {
    let mut password = section.password.clone();
    let mut username = section.username.clone();
    let mut repository_url = section.repository.clone();

    if let Some(userinfo) = &section.repository_userinfo {
        if let Some(url_username) = &userinfo.username {
            merge_optional(&mut username, url_username, "username", &section.name)?;
        }
        if let Some(url_password) = &userinfo.password {
            merge_optional(&mut password, url_password, "password", &section.name)?;
        }
        repository_url = Some(userinfo.sanitized_url.clone());
    }

    let Some(password) = password else {
        return Ok(None);
    };
    let repository_url = repository_url
        .or_else(|| default_repository_url(&section.name).map(str::to_string))
        .ok_or_else(|| {
            format!(
                "section [{}] has credentials but no repository URL; migrate it manually",
                section.name
            )
        })?;

    if username.is_none() && !is_default_pypi_repository(&repository_url) {
        return Err(format!(
            "section [{}] has non-PyPI credentials without a username; migrate it manually",
            section.name
        ));
    }

    Ok(Some(TwineCredential {
        username,
        password,
        repository_url,
    }))
}

fn merge_optional(
    target: &mut Option<String>,
    value: &str,
    label: &str,
    section_name: &str,
) -> Result<(), String> {
    if let Some(existing) = target {
        if existing != value {
            return Err(format!(
                "section [{section_name}] has conflicting {label} values; migrate it manually"
            ));
        }
        return Ok(());
    }
    *target = Some(value.to_string());
    Ok(())
}

fn default_repository_url(section_name: &str) -> Option<&'static str> {
    match section_name {
        "pypi" => Some("https://upload.pypi.org/legacy/"),
        "testpypi" => Some("https://test.pypi.org/legacy/"),
        _ => None,
    }
}

fn is_default_pypi_repository(repository_url: &str) -> bool {
    matches!(
        repository_url,
        "https://upload.pypi.org/legacy/" | "https://test.pypi.org/legacy/"
    )
}

fn reject_env_line_breaks(key: &str, value: &str) -> Result<(), String> {
    if value.contains('\n') || value.contains('\r') {
        return Err(format!("{key} cannot contain line breaks"));
    }
    Ok(())
}

fn repository_userinfo(value: &str) -> Option<RepositoryUserinfo> {
    let (scheme, rest) = value
        .strip_prefix("https://")
        .map(|rest| ("https://", rest))
        .or_else(|| value.strip_prefix("http://").map(|rest| ("http://", rest)))?;
    let userinfo_end = rest.find('@')?;
    let host_end = rest.find('/').unwrap_or(rest.len());
    if userinfo_end >= host_end {
        return None;
    }

    let userinfo = &rest[..userinfo_end];
    let (username, password) = if let Some((username, password)) = userinfo.split_once(':') {
        (
            (!username.is_empty()).then(|| username.to_string()),
            (!password.is_empty()).then(|| password.to_string()),
        )
    } else {
        ((!userinfo.is_empty()).then(|| userinfo.to_string()), None)
    };
    Some(RepositoryUserinfo {
        username,
        password,
        sanitized_url: format!("{scheme}{}", &rest[userinfo_end + 1..]),
    })
}

fn strip_url_userinfo(value: &str) -> String {
    let Some((scheme, rest)) = value
        .strip_prefix("https://")
        .map(|rest| ("https://", rest))
        .or_else(|| value.strip_prefix("http://").map(|rest| ("http://", rest)))
    else {
        return value.to_string();
    };
    let Some(userinfo_end) = rest.find('@') else {
        return value.to_string();
    };
    let host_end = rest.find('/').unwrap_or(rest.len());
    if userinfo_end >= host_end {
        return value.to_string();
    }
    format!("{scheme}{}", &rest[userinfo_end + 1..])
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
    Err(format!("failed to store isotope key {account}: {message}"))
}

#[cfg(any(not(target_os = "macos"), test, coverage))]
fn keychain_store_secret(_service: &str, _account: &str, _value: &str) -> Result<(), String> {
    Err("isotope keychain integration is only available on macOS".to_string())
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
    fn migrates_pypirc_and_sanitizes_local_copy() {
        let temp = std::env::temp_dir().join(format!(
            "{}-migrate-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        let path = temp.join(".pypirc");
        let contents = "[pypi]\nusername = __token__\npassword = fake-token\n";
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_credentials_file(&path, &store).unwrap());

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                TWINE_ENV_ASSIGNMENTS_KEY.to_string(),
                "TWINE_USERNAME=__token__\nTWINE_PASSWORD=fake-token\nTWINE_REPOSITORY_URL=https://upload.pypi.org/legacy/".to_string()
            )]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "[pypi]\nusername = __token__\npassword =\n"
        );
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn migrates_repository_url_userinfo() {
        let temp = std::env::temp_dir().join(format!(
            "{}-userinfo-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        let path = temp.join(".pypirc");
        let contents = "[internal]\nrepository = https://user:fake@example.invalid/simple/\n";
        fs::write(&path, contents).unwrap();
        let store = TestCredentialStore::default();

        assert!(migrate_credentials_file(&path, &store).unwrap());

        assert_eq!(
            store.values.borrow().as_slice(),
            &[(
                TWINE_ENV_ASSIGNMENTS_KEY.to_string(),
                "TWINE_USERNAME=user\nTWINE_PASSWORD=fake\nTWINE_REPOSITORY_URL=https://example.invalid/simple/".to_string()
            )]
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "[internal]\nrepository= https://example.invalid/simple/\n"
        );
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn rejects_multiple_secret_repositories() {
        let contents = "\
[pypi]
username = __token__
password = first-token
[internal]
username = user
password = second-token
repository = https://example.invalid/simple/
";

        let err = twine_env_assignments(contents).unwrap_err();

        assert!(err.contains("multiple repositories"));
    }

    #[test]
    fn rejects_custom_repository_without_url() {
        let contents = "[internal]\nusername = user\npassword = fake\n";

        let err = twine_env_assignments(contents).unwrap_err();

        assert!(err.contains("no repository URL"));
    }

    #[test]
    fn ignores_missing_and_secretless_files() {
        let temp = std::env::temp_dir().join(format!(
            "{}-missing-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        let store = TestCredentialStore::default();

        assert!(!migrate_credentials_file(&temp.join(".pypirc"), &store).unwrap());
        let path = temp.join("plain.pypirc");
        fs::write(
            &path,
            "[pypi]\nrepository = https://example.invalid/simple/\n",
        )
        .unwrap();
        assert!(!migrate_credentials_file(&path, &store).unwrap());
        assert!(store.values.borrow().is_empty());
        fs::remove_dir_all(temp).unwrap();
    }
}
