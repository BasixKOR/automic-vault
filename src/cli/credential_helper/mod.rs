use std::ffi::OsStr;
use std::io::Write;

use super::stub;

const KEYCHAIN_SERVICE: &str = "com.automicvault.isotope";
const TOKEN_ENV: &str = "AUTOMIC_VAULT_CREDENTIAL_HELPER_TOKEN";
const AWS_ACCESS_KEY_ID: &str = "AWS_ACCESS_KEY_ID";
const AWS_SECRET_ACCESS_KEY: &str = "AWS_SECRET_ACCESS_KEY";

pub(crate) fn run(protocol: &OsStr, stdout: &mut dyn Write) -> Result<(), String> {
    if protocol != "aws" {
        return Err("unknown credential helper protocol".to_string());
    }
    let token = std::env::var(TOKEN_ENV)
        .map_err(|_| "missing AWS credential_process approval token".to_string())?;
    stub::broker_request(&format!("validate aws {token}\n"))?;

    let access_key_id = load_secret(AWS_ACCESS_KEY_ID)?;
    let secret_access_key = load_secret(AWS_SECRET_ACCESS_KEY)?;
    writeln!(
        stdout,
        "{{\"AccessKeyId\":{},\"SecretAccessKey\":{},\"Version\":1}}",
        json_string(&access_key_id),
        json_string(&secret_access_key)
    )
    .map_err(|err| format!("failed to write AWS credential_process response: {err}"))
}

pub(crate) fn load_secret(key: &str) -> Result<String, String> {
    load_secret_if_present(key)?.ok_or_else(|| format!("failed to load isotope key {key}: -25300"))
}

pub(crate) fn load_secret_if_present(key: &str) -> Result<Option<String>, String> {
    if let Some(dir) = std::env::var_os("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR") {
        let path = std::path::PathBuf::from(dir).join(key);
        return match std::fs::read_to_string(&path) {
            Ok(value) => Ok(Some(value)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(format!("failed to read {}: {err}", path.display())),
        };
    }
    if let Ok(value) = std::env::var(format!("AUTOMIC_VAULT_TEST_{key}")) {
        return Ok(Some(value));
    }
    keychain_load_secret_if_present(KEYCHAIN_SERVICE, key)
}

#[cfg(target_os = "macos")]
fn keychain_load_secret_if_present(service: &str, account: &str) -> Result<Option<String>, String> {
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
    }

    let service_cstr =
        CString::new(service).map_err(|_| "invalid keychain service name".to_string())?;
    let account_cstr =
        CString::new(account).map_err(|_| "invalid keychain account name".to_string())?;
    let mut len = 0u32;
    let mut data = std::ptr::null_mut();
    let status = unsafe {
        SecKeychainFindGenericPassword(
            std::ptr::null(),
            service.len() as u32,
            service_cstr.as_ptr(),
            account.len() as u32,
            account_cstr.as_ptr(),
            &mut len,
            &mut data,
            std::ptr::null_mut(),
        )
    };
    if status == -25300 {
        return Ok(None);
    }
    if status != 0 {
        return Err(format!("failed to load isotope key {account}: {status}"));
    }

    let bytes = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), len as usize) }.to_vec();
    unsafe {
        let _ = SecKeychainItemFreeContent(std::ptr::null(), data);
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| format!("isotope key {account} is not valid UTF-8"))
}

#[cfg(not(target_os = "macos"))]
fn keychain_load_secret_if_present(
    _service: &str,
    _account: &str,
) -> Result<Option<String>, String> {
    Err("keychain access is only available on macOS".to_string())
}

fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            value if value < ' ' => out.push_str(&format!("\\u{:04x}", value as u32)),
            value => out.push(value),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_escapes_secret_values() {
        assert_eq!(json_string("a\"b\\c\n"), r#""a\"b\\c\n""#);
    }
}
