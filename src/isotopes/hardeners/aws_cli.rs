use std::fs;
use std::io::{self, IsTerminal, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const AWS_ACCESS_KEY_ID: &str = "AWS_ACCESS_KEY_ID";
const AWS_SECRET_ACCESS_KEY: &str = "AWS_SECRET_ACCESS_KEY";
const STUB_DIR: &str = "/usr/local/bin";
const STUB_MARKER: &str = "# Automic Vault hardened stub";
const AWS_STUB: &str = include_str!("aws");
const AWS_STUB_PATH: &str = "/usr/local/bin/aws";
const AWS_VAULT_PATH: &str = "/opt/homebrew/bin/aws-vault";

unsafe extern "C" {
    fn geteuid() -> u32;
}

pub(crate) fn run_aws(stdout: &mut dyn Write, yes: bool) -> Result<(), String> {
    let aws_vault = aws_vault_path();
    if !aws_vault.exists() {
        return Err("aws-vault is not installed; run `brew install aws-vault`".to_string());
    }
    let profile = std::env::var("AWS_PROFILE").unwrap_or_else(|_| "default".to_string());
    let credentials_path = aws_credentials_path()?;
    let credentials = read_aws_credentials(&credentials_path, &profile)?;
    let is_root = unsafe { geteuid() } == 0;
    let has_test_keychain = std::env::var_os("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR").is_some();
    if is_root && credentials.is_some() && !has_test_keychain {
        return Err(
            "run `av harden aws` without sudo first to import keys into your login keychain"
                .to_string(),
        );
    }

    writeln!(stdout, "╭─ harden aws").ok();
    writeln!(stdout, "│").ok();
    writeln!(stdout, "◆ This will use aws-vault for AWS credentials.").ok();
    writeln!(stdout, "│").ok();
    if credentials.is_some() {
        writeln!(
            stdout,
            "├─ import {profile} keys from {} into the login keychain",
            credentials_path.display()
        )
        .ok();
        writeln!(
            stdout,
            "├─ delete {profile} plaintext keys from {}",
            credentials_path.display()
        )
        .ok();
    } else {
        writeln!(
            stdout,
            "├─ no {profile} plaintext keys found in {}",
            credentials_path.display()
        )
        .ok();
    }

    if let Some(credentials) = credentials {
        writeln!(stdout, "│").ok();
        if !confirm(stdout, yes)? {
            writeln!(stdout, "╰─ cancelled").ok();
            return Ok(());
        }
        import_aws_credentials(&credentials)?;
        delete_aws_credentials(&credentials_path, &profile)?;
        writeln!(stdout, "├─ imported keys").ok();
        writeln!(stdout, "├─ deleted plaintext keys").ok();
    }

    if !is_root {
        writeln!(stdout, "╰─ finish with `sudo av harden aws`").ok();
        return Ok(());
    }

    writeln!(stdout, "├─ write {AWS_STUB_PATH}").ok();
    writeln!(stdout, "│").ok();
    if !confirm(stdout, yes)? {
        writeln!(stdout, "╰─ cancelled").ok();
        return Ok(());
    }
    install_aws_stub(&aws_stub_path())?;
    writeln!(stdout, "╰─ wrote {AWS_STUB_PATH}").ok();
    Ok(())
}

pub(crate) fn run_stub_install(
    target: &Path,
    stdout: &mut dyn Write,
    yes: bool,
) -> Result<(), String> {
    if unsafe { geteuid() } != 0 {
        return Err("must be run as root, use: sudo av harden PATH".to_string());
    }
    if !target.is_absolute() {
        return Err("target path must be absolute".to_string());
    }
    if !target.exists() {
        return Err(format!("{} does not exist", target.display()));
    }

    if !Path::new("/usr/local/bin/av").exists() {
        return Err("/usr/local/bin/av is not installed".to_string());
    }
    let tool = target
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("tool");
    let stub = stub_path(target)?;
    if stub.exists() && !is_av_stub(&stub) {
        return Err(format!(
            "{} already exists and is not an av hardened stub",
            stub.display()
        ));
    }

    writeln!(stdout, "╭─ install stub {tool}").ok();
    writeln!(stdout, "│").ok();
    writeln!(stdout, "◆ This will:").ok();
    writeln!(stdout, "│  1. verify /usr/local/bin/av").ok();
    writeln!(stdout, "│  2. write {}", stub.display()).ok();
    writeln!(stdout, "│  3. point it at {}", target.display()).ok();
    writeln!(stdout, "│").ok();
    if !confirm(stdout, yes)? {
        writeln!(stdout, "╰─ cancelled").ok();
        return Ok(());
    }
    writeln!(stdout, "│").ok();
    writeln!(stdout, "├─ ✓ verified /usr/local/bin/av").ok();

    fs::create_dir_all(STUB_DIR).map_err(|err| format!("failed to create {STUB_DIR}: {err}"))?;
    if stub.exists() {
        fs::remove_file(&stub)
            .map_err(|err| format!("failed to replace {}: {err}", stub.display()))?;
    }
    fs::write(&stub, stub_script(target)?)
        .map_err(|err| format!("failed to install stub at {}: {err}", stub.display()))?;
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755))
        .map_err(|err| format!("failed to chmod {}: {err}", stub.display()))?;

    writeln!(stdout, "├─ ✓ wrote {}", stub.display()).ok();
    writeln!(stdout, "╰─ done").ok();
    Ok(())
}

fn confirm(stdout: &mut dyn Write, yes: bool) -> Result<bool, String> {
    if yes {
        writeln!(stdout, "◇ Continue? yes (--yes)").ok();
        return Ok(true);
    }

    write!(stdout, "◇ Continue? [y/N] ").ok();
    stdout
        .flush()
        .map_err(|err| format!("failed to flush prompt: {err}"))?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|err| format!("failed to read confirmation: {err}"))?;
    if !io::stdin().is_terminal() {
        writeln!(stdout).ok();
    }
    Ok(matches!(
        input.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn stub_path(target: &Path) -> Result<PathBuf, String> {
    let name = target
        .file_name()
        .ok_or_else(|| "target path must end in a file name".to_string())?;
    Ok(Path::new(STUB_DIR).join(name))
}

fn is_av_stub(path: &Path) -> bool {
    fs::read_to_string(path)
        .map(|contents| contents.lines().nth(1) == Some(STUB_MARKER))
        .unwrap_or(false)
}

fn stub_script(target: &Path) -> Result<String, String> {
    let name = target
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "target path must end in a UTF-8 file name".to_string())?;
    Ok(format!(
        "#!/bin/sh\n{STUB_MARKER}\nexec /usr/local/bin/av stub-exec '{}' '{}' \"$@\"\n",
        shell_quote(name),
        shell_quote(&target.display().to_string())
    ))
}

fn shell_quote(value: &str) -> String {
    value.replace('\'', "'\\''")
}

fn aws_vault_path() -> PathBuf {
    std::env::var_os("AUTOMIC_VAULT_TEST_AWS_VAULT_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(AWS_VAULT_PATH))
}

fn aws_stub_path() -> PathBuf {
    std::env::var_os("AUTOMIC_VAULT_TEST_AWS_STUB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(AWS_STUB_PATH))
}

fn install_aws_stub(path: &Path) -> Result<(), String> {
    fs::write(path, AWS_STUB)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .map_err(|err| format!("failed to chmod {}: {err}", path.display()))
}

#[derive(Debug, PartialEq, Eq)]
struct AwsCredentials {
    access_key_id: String,
    secret_access_key: String,
}

fn aws_credentials_path() -> Result<PathBuf, String> {
    if let Some(path) =
        std::env::var_os("AWS_SHARED_CREDENTIALS_FILE").filter(|path| !path.is_empty())
    {
        return Ok(PathBuf::from(path));
    }
    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
    Ok(PathBuf::from(home).join(".aws/credentials"))
}

fn read_aws_credentials(path: &Path, profile: &str) -> Result<Option<AwsCredentials>, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    parse_aws_credentials(&contents, profile)
}

fn parse_aws_credentials(contents: &str, profile: &str) -> Result<Option<AwsCredentials>, String> {
    let mut in_profile = false;
    let mut access_key_id = None;
    let mut secret_access_key = None;

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if let Some(section) = section_name(trimmed) {
            in_profile = section == profile;
            continue;
        }
        if !in_profile {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "aws_access_key_id" if !value.is_empty() => access_key_id = Some(value.to_string()),
            "aws_secret_access_key" if !value.is_empty() => {
                secret_access_key = Some(value.to_string())
            }
            _ => {}
        }
    }

    match (access_key_id, secret_access_key) {
        (Some(access_key_id), Some(secret_access_key)) => Ok(Some(AwsCredentials {
            access_key_id,
            secret_access_key,
        })),
        (None, None) => Ok(None),
        _ => Err(format!(
            "AWS shared credentials file has incomplete AWS keys for profile {profile}"
        )),
    }
}

fn delete_aws_credentials(path: &Path, profile: &str) -> Result<(), String> {
    let contents = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let cleaned = remove_aws_credentials(&contents, profile);
    fs::write(path, cleaned).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn remove_aws_credentials(contents: &str, profile: &str) -> String {
    let mut in_profile = false;
    let mut out = String::new();
    for line in contents.split_inclusive('\n') {
        let trimmed = line.trim();
        if let Some(section) = section_name(trimmed) {
            in_profile = section == profile;
        }
        if in_profile
            && trimmed.split_once('=').is_some_and(|(key, _)| {
                matches!(key.trim(), "aws_access_key_id" | "aws_secret_access_key")
            })
        {
            continue;
        }
        out.push_str(line);
    }
    out
}

fn section_name(line: &str) -> Option<&str> {
    line.strip_prefix('[')
        .and_then(|line| line.strip_suffix(']'))
        .map(str::trim)
}

fn import_aws_credentials(credentials: &AwsCredentials) -> Result<(), String> {
    store_keychain_secret(AWS_ACCESS_KEY_ID, &credentials.access_key_id)?;
    store_keychain_secret(AWS_SECRET_ACCESS_KEY, &credentials.secret_access_key)
}

fn store_keychain_secret(account: &str, value: &str) -> Result<(), String> {
    if let Some(dir) = std::env::var_os("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR") {
        fs::create_dir_all(&dir)
            .map_err(|err| format!("failed to create test keychain dir: {err}"))?;
        let path = PathBuf::from(dir).join(account);
        return fs::write(&path, value)
            .map_err(|err| format!("failed to write {}: {err}", path.display()));
    }
    keychain_store_secret(KEYCHAIN_SERVICE, account, value)
}

#[cfg(target_os = "macos")]
fn keychain_store_secret(service: &str, account: &str, value: &str) -> Result<(), String> {
    use std::ffi::{CString, c_void};

    #[link(name = "Security", kind = "framework")]
    unsafe extern "C" {
        fn SecKeychainAddGenericPassword(
            keychain: *const c_void,
            service_name_length: u32,
            service_name: *const i8,
            account_name_length: u32,
            account_name: *const i8,
            password_length: u32,
            password_data: *const c_void,
            item_ref: *mut *mut c_void,
        ) -> i32;
        fn SecKeychainFindGenericPassword(
            keychain_or_array: *const c_void,
            service_name_length: u32,
            service_name: *const i8,
            account_name_length: u32,
            account_name: *const i8,
            password_length: *mut u32,
            password_data: *mut *mut c_void,
            item_ref: *mut *mut c_void,
        ) -> i32;
        fn SecKeychainItemModifyAttributesAndData(
            item_ref: *mut c_void,
            attr_list: *const c_void,
            length: u32,
            data: *const c_void,
        ) -> i32;
        fn SecKeychainItemFreeContent(attr_list: *const c_void, data: *mut c_void) -> i32;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFRelease(cf: *const c_void);
    }

    let service_cstr =
        CString::new(service).map_err(|_| "invalid keychain service name".to_string())?;
    let account_cstr =
        CString::new(account).map_err(|_| "invalid keychain account name".to_string())?;
    let mut item_ref = std::ptr::null_mut();
    let status = unsafe {
        SecKeychainAddGenericPassword(
            std::ptr::null(),
            service.len() as u32,
            service_cstr.as_ptr(),
            account.len() as u32,
            account_cstr.as_ptr(),
            value.len() as u32,
            value.as_ptr().cast(),
            &mut item_ref,
        )
    };
    if status == 0 {
        if !item_ref.is_null() {
            unsafe { CFRelease(item_ref.cast()) };
        }
        return Ok(());
    }
    if status != -25299 {
        return Err(format!("failed to store isotope key {account}: {status}"));
    }

    let mut old_len = 0u32;
    let mut old_data = std::ptr::null_mut();
    let status = unsafe {
        SecKeychainFindGenericPassword(
            std::ptr::null(),
            service.len() as u32,
            service_cstr.as_ptr(),
            account.len() as u32,
            account_cstr.as_ptr(),
            &mut old_len,
            &mut old_data,
            &mut item_ref,
        )
    };
    if status != 0 {
        return Err(format!("failed to find isotope key {account}: {status}"));
    }
    if !old_data.is_null() {
        unsafe {
            let _ = SecKeychainItemFreeContent(std::ptr::null(), old_data);
        }
    }
    let status = unsafe {
        SecKeychainItemModifyAttributesAndData(
            item_ref,
            std::ptr::null(),
            value.len() as u32,
            value.as_ptr().cast(),
        )
    };
    if !item_ref.is_null() {
        unsafe { CFRelease(item_ref.cast()) };
    }
    if status == 0 {
        Ok(())
    } else {
        Err(format!("failed to update isotope key {account}: {status}"))
    }
}

#[cfg(not(target_os = "macos"))]
fn keychain_store_secret(_service: &str, _account: &str, _value: &str) -> Result<(), String> {
    Err("keychain access is only available on macOS".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn aws_stub_uses_aws_vault_profile_env() {
        let path = temp_path("aws-stub");
        install_aws_stub(&path).unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), AWS_STUB);
        assert!(AWS_STUB.contains("${AWS_PROFILE:-default}"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn missing_aws_vault_tells_user_to_install_it() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let missing = temp_path("missing-aws-vault");
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_AWS_VAULT_PATH", &missing);
        }

        let err = run_aws(&mut Vec::new(), true).unwrap_err();

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_AWS_VAULT_PATH");
        }
        assert_eq!(
            err,
            "aws-vault is not installed; run `brew install aws-vault`"
        );
    }

    #[test]
    fn parses_profile_credentials() {
        assert_eq!(
            parse_aws_credentials(
                "[default]\naws_access_key_id = AKIA\naws_secret_access_key= secret\n",
                "default"
            )
            .unwrap(),
            Some(AwsCredentials {
                access_key_id: "AKIA".to_string(),
                secret_access_key: "secret".to_string()
            })
        );
    }

    #[test]
    fn removes_only_selected_profile_keys() {
        assert_eq!(
            remove_aws_credentials(
                "[default]\naws_access_key_id = AKIA\nregion = us-east-1\naws_secret_access_key = secret\n[dev]\naws_access_key_id = keep\n",
                "default"
            ),
            "[default]\nregion = us-east-1\n[dev]\naws_access_key_id = keep\n"
        );
    }

    #[test]
    fn harden_imports_keys_and_deletes_plaintext_credentials() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let dir = temp_path("aws-import");
        let credentials_path = dir.join("credentials");
        let keychain_dir = dir.join("keychain");
        let aws_vault = dir.join("aws-vault");
        let aws_stub = dir.join("aws");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&aws_vault, "").unwrap();
        fs::write(
            &credentials_path,
            "[default]\naws_access_key_id = AKIA\nregion = us-east-1\naws_secret_access_key = secret\n",
        )
        .unwrap();
        unsafe {
            std::env::set_var("AWS_SHARED_CREDENTIALS_FILE", &credentials_path);
            std::env::remove_var("AWS_PROFILE");
            std::env::set_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR", &keychain_dir);
            std::env::set_var("AUTOMIC_VAULT_TEST_AWS_VAULT_PATH", &aws_vault);
            std::env::set_var("AUTOMIC_VAULT_TEST_AWS_STUB_PATH", &aws_stub);
        }

        run_aws(&mut Vec::new(), true).unwrap();

        unsafe {
            std::env::remove_var("AWS_SHARED_CREDENTIALS_FILE");
            std::env::remove_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR");
            std::env::remove_var("AUTOMIC_VAULT_TEST_AWS_VAULT_PATH");
            std::env::remove_var("AUTOMIC_VAULT_TEST_AWS_STUB_PATH");
        }
        assert_eq!(
            fs::read_to_string(keychain_dir.join(AWS_ACCESS_KEY_ID)).unwrap(),
            "AKIA"
        );
        assert_eq!(
            fs::read_to_string(keychain_dir.join(AWS_SECRET_ACCESS_KEY)).unwrap(),
            "secret"
        );
        assert_eq!(
            fs::read_to_string(credentials_path).unwrap(),
            "[default]\nregion = us-east-1\n"
        );
        let _ = fs::remove_dir_all(dir);
    }

    fn temp_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("av-{label}-{}-{nanos}", std::process::id()))
    }
}
