use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const STUB_DIR: &str = "/usr/local/bin";
const STUB_MARKER: &str = "# Automic Vault hardened stub";
const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const AWS_ACCESS_KEY_ID_FILE_KEY: &str = "aws_access_key_id";
const AWS_SECRET_ACCESS_KEY_FILE_KEY: &str = "aws_secret_access_key";
const AWS_ACCESS_KEY_ID_ENV_KEY: &str = "AWS_ACCESS_KEY_ID";
const AWS_SECRET_ACCESS_KEY_ENV_KEY: &str = "AWS_SECRET_ACCESS_KEY";
const AWS_CREDENTIAL_PROCESS: &str = "/usr/local/bin/av credential-helper aws";

unsafe extern "C" {
    fn geteuid() -> u32;
}

pub(crate) fn run_aws(stdout: &mut dyn Write) -> Result<(), String> {
    if is_root() {
        return Err("run `av harden aws` as your normal user".to_string());
    }

    let home = home_dir()?;
    let credentials_path = home.join(".aws/credentials");
    let config_path = home.join(".aws/config");
    let target = std::env::var_os("AUTOMIC_VAULT_TEST_AWS_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/opt/homebrew/bin/aws"));

    writeln!(stdout, "╭─ harden aws").ok();
    let contents = fs::read_to_string(&credentials_path)
        .map_err(|_| format!("no AWS credentials found at {}", credentials_path.display()))?;
    let credentials = default_aws_credentials(&contents)
        .ok_or_else(|| "no default AWS access key pair found".to_string())?;

    store_secret(AWS_ACCESS_KEY_ID_ENV_KEY, &credentials.access_key_id)?;
    store_secret(
        AWS_SECRET_ACCESS_KEY_ENV_KEY,
        &credentials.secret_access_key,
    )?;
    writeln!(stdout, "├─ ✓ moved credentials to Keychain").ok();

    ensure_credential_process(&config_path)?;
    writeln!(stdout, "├─ ✓ configured {}", config_path.display()).ok();

    fs::write(&credentials_path, remove_default_aws_key_lines(&contents))
        .map_err(|err| format!("failed to write {}: {err}", credentials_path.display()))?;
    writeln!(stdout, "├─ ✓ removed plaintext keys").ok();
    writeln!(stdout, "│").ok();
    writeln!(stdout, "╰─ finish with").ok();
    writeln!(stdout, "   sudo av harden {}", target.display()).ok();
    Ok(())
}

pub(crate) fn run_stub_install(target: &Path, stdout: &mut dyn Write) -> Result<(), String> {
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
    writeln!(stdout, "╭─ install stub {tool}").ok();
    writeln!(stdout, "├─ ✓ verified /usr/local/bin/av").ok();

    let stub = stub_path(target)?;
    if stub.exists() && !is_av_stub(&stub) {
        return Err(format!(
            "{} already exists and is not an av hardened stub",
            stub.display()
        ));
    }

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

fn is_root() -> bool {
    (unsafe { geteuid() }) == 0
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

struct AwsCredentials {
    access_key_id: String,
    secret_access_key: String,
}

fn default_aws_credentials(contents: &str) -> Option<AwsCredentials> {
    let mut in_default = false;
    let mut access_key_id = None;
    let mut secret_access_key = None;

    for line in contents.lines() {
        let trimmed = line.trim();
        if let Some(section) = section_name(trimmed) {
            in_default = section == "default";
            continue;
        }
        if !in_default {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        match key.trim() {
            AWS_ACCESS_KEY_ID_FILE_KEY if !value.trim().is_empty() => {
                access_key_id = Some(value.trim().to_string())
            }
            AWS_SECRET_ACCESS_KEY_FILE_KEY if !value.trim().is_empty() => {
                secret_access_key = Some(value.trim().to_string())
            }
            _ => {}
        }
    }

    Some(AwsCredentials {
        access_key_id: access_key_id?,
        secret_access_key: secret_access_key?,
    })
}

fn section_name(line: &str) -> Option<&str> {
    line.strip_prefix('[')
        .and_then(|line| line.strip_suffix(']'))
        .map(str::trim)
}

fn remove_default_aws_key_lines(contents: &str) -> String {
    let mut output = String::new();
    let mut in_default = false;
    for line in contents.lines() {
        let trimmed = line.trim();
        if let Some(section) = section_name(trimmed) {
            in_default = section == "default";
            push_line(&mut output, line);
            continue;
        }
        if in_default
            && trimmed.split_once('=').is_some_and(|(key, _)| {
                matches!(
                    key.trim(),
                    AWS_ACCESS_KEY_ID_FILE_KEY | AWS_SECRET_ACCESS_KEY_FILE_KEY
                )
            })
        {
            continue;
        }
        push_line(&mut output, line);
    }
    output
}

fn ensure_credential_process(path: &Path) -> Result<(), String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
            }
            fs::write(
                path,
                format!("[default]\ncredential_process = {AWS_CREDENTIAL_PROCESS}\n"),
            )
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
            return Ok(());
        }
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    fs::write(path, upsert_credential_process(&contents))
        .map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn upsert_credential_process(contents: &str) -> String {
    let mut output = String::new();
    let mut in_default = false;
    let mut saw_default = false;
    let mut wrote = false;

    for line in contents.lines() {
        let trimmed = line.trim();
        if let Some(section) = section_name(trimmed) {
            if in_default && !wrote {
                push_credential_process(&mut output);
                wrote = true;
            }
            in_default = section == "default";
            saw_default |= in_default;
            push_line(&mut output, line);
            continue;
        }
        if in_default
            && trimmed
                .split_once('=')
                .is_some_and(|(key, _)| key.trim() == "credential_process")
        {
            if !wrote {
                push_credential_process(&mut output);
                wrote = true;
            }
            continue;
        }
        push_line(&mut output, line);
    }
    if in_default && !wrote {
        push_credential_process(&mut output);
    } else if !saw_default {
        if !output.is_empty() {
            output.push('\n');
        }
        push_line(&mut output, "[default]");
        push_credential_process(&mut output);
    }
    output
}

fn push_credential_process(output: &mut String) {
    push_line(
        output,
        &format!("credential_process = {AWS_CREDENTIAL_PROCESS}"),
    );
}

fn push_line(output: &mut String, line: &str) {
    output.push_str(line);
    output.push('\n');
}

fn store_secret(key: &str, value: &str) -> Result<(), String> {
    if let Some(dir) = std::env::var_os("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR") {
        let path = PathBuf::from(dir).join(key);
        fs::write(&path, value).map_err(|err| format!("failed to write {}: {err}", path.display()))
    } else {
        keychain_store_secret(KEYCHAIN_SERVICE, key, value)
    }
}

#[cfg(target_os = "macos")]
fn keychain_store_secret(service: &str, account: &str, value: &str) -> Result<(), String> {
    use std::ffi::{CString, c_void};

    #[link(name = "Security", kind = "framework")]
    unsafe extern "C" {
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
        fn SecKeychainItemFreeContent(attr_list: *const c_void, data: *mut c_void) -> i32;
        fn SecKeychainItemModifyContent(
            item_ref: *mut c_void,
            attr_list: *const c_void,
            length: u32,
            data: *const c_void,
        ) -> i32;
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
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFRelease(value: *const c_void);
    }

    let service_cstr =
        CString::new(service).map_err(|_| "invalid keychain service name".to_string())?;
    let account_cstr =
        CString::new(account).map_err(|_| "invalid keychain account name".to_string())?;

    let mut len = 0u32;
    let mut data = std::ptr::null_mut();
    let mut item = std::ptr::null_mut();
    let find_status = unsafe {
        SecKeychainFindGenericPassword(
            std::ptr::null(),
            service.len() as u32,
            service_cstr.as_ptr(),
            account.len() as u32,
            account_cstr.as_ptr(),
            &mut len,
            &mut data,
            &mut item,
        )
    };
    if !data.is_null() {
        unsafe {
            let _ = SecKeychainItemFreeContent(std::ptr::null(), data);
        }
    }
    if find_status == 0 {
        let status = unsafe {
            SecKeychainItemModifyContent(
                item,
                std::ptr::null(),
                value.len() as u32,
                value.as_ptr().cast(),
            )
        };
        if !item.is_null() {
            unsafe { CFRelease(item.cast()) };
        }
        return if status == 0 {
            Ok(())
        } else {
            Err(format!("failed to update isotope key {account}: {status}"))
        };
    }
    if find_status != -25300 {
        return Err(format!(
            "failed to load isotope key {account}: {find_status}"
        ));
    }

    let add_status = unsafe {
        SecKeychainAddGenericPassword(
            std::ptr::null(),
            service.len() as u32,
            service_cstr.as_ptr(),
            account.len() as u32,
            account_cstr.as_ptr(),
            value.len() as u32,
            value.as_ptr().cast(),
            std::ptr::null_mut(),
        )
    };
    if add_status == 0 {
        Ok(())
    } else {
        Err(format!(
            "failed to store isotope key {account}: {add_status}"
        ))
    }
}

#[cfg(not(target_os = "macos"))]
fn keychain_store_secret(_service: &str, _account: &str, _value: &str) -> Result<(), String> {
    Err("keychain access is only available on macOS".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn migrates_aws_default_keys_and_configures_credential_process() {
        let home = temp_home("aws-harden");
        let keychain = home.join("keychain");
        let aws_dir = home.join(".aws");
        fs::create_dir_all(&aws_dir).unwrap();
        fs::create_dir_all(&keychain).unwrap();
        fs::write(
            aws_dir.join("credentials"),
            "[default]\naws_access_key_id = AKIA\naws_secret_access_key = secret\nregion = us\n",
        )
        .unwrap();

        let _guard = crate::tests::ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR", &keychain);
            std::env::set_var("AUTOMIC_VAULT_TEST_AWS_PATH", "/tmp/aws");
        }
        let mut stdout = Vec::new();
        run_aws(&mut stdout).unwrap();
        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR");
            std::env::remove_var("AUTOMIC_VAULT_TEST_AWS_PATH");
        }

        assert_eq!(
            fs::read_to_string(keychain.join(AWS_ACCESS_KEY_ID_ENV_KEY)).unwrap(),
            "AKIA"
        );
        assert_eq!(
            fs::read_to_string(keychain.join(AWS_SECRET_ACCESS_KEY_ENV_KEY)).unwrap(),
            "secret"
        );
        let credentials = fs::read_to_string(aws_dir.join("credentials")).unwrap();
        assert!(!credentials.contains("aws_access_key_id"));
        assert!(!credentials.contains("aws_secret_access_key"));
        assert!(credentials.contains("region = us"));
        assert!(
            fs::read_to_string(aws_dir.join("config"))
                .unwrap()
                .contains("credential_process = /usr/local/bin/av credential-helper aws")
        );
        assert!(
            String::from_utf8(stdout)
                .unwrap()
                .contains("sudo av harden /tmp/aws")
        );

        let _ = fs::remove_dir_all(home);
    }

    fn temp_home(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "av-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
