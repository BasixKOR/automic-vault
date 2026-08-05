use std::ffi::{CStr, CString, OsString};
use std::fs::File;
use std::io::{Seek, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

const REAL_AWS: &str = "/opt/homebrew/bin/aws";
const AWS_ACCESS_KEY_ID: &str = "AWS_ACCESS_KEY_ID";
const AWS_SECRET_ACCESS_KEY: &str = "AWS_SECRET_ACCESS_KEY";

pub(crate) fn run(mut args: Vec<OsString>, stderr: &mut dyn Write) -> i32 {
    if args.first().is_some_and(is_stub_arg) {
        args.remove(0);
    }
    if unsafe { libc::geteuid() } == 0 {
        let _ = writeln!(
            stderr,
            "aws: Automic Vault's AWS launcher must not run as root"
        );
        return 1;
    }
    match launch(args) {
        Ok(never) => never,
        Err(error) => {
            let _ = writeln!(stderr, "aws: {error}");
            1
        }
    }
}

fn is_stub_arg(arg: &OsString) -> bool {
    PathBuf::from(arg).is_absolute()
        && std::fs::read_to_string(arg)
            .is_ok_and(|contents| contents == "#!/usr/local/bin/av aws\n")
}

pub(crate) fn credentials(stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match xpc_request("aws-credentials", |_| Ok(())) {
        Ok(value) => {
            let _ = writeln!(stdout, "{value}");
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "aws credential helper: {error}");
            1
        }
    }
}

fn launch(args: Vec<OsString>) -> Result<i32, String> {
    let profile = selected_profile(&args)?;
    let config_path = std::env::var_os("AWS_CONFIG_FILE")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".aws/config")))
        .ok_or_else(|| "HOME and AWS_CONFIG_FILE are not set".to_string())?;
    let config = match std::fs::read(&config_path) {
        Ok(config) => config,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(format!("failed to read {}: {error}", config_path.display())),
    };
    let target = real_aws_path();
    if !target.is_file() {
        return Err(format!(
            "Homebrew AWS CLI is not installed at {}",
            target.display()
        ));
    }
    let cwd = std::env::current_dir()
        .map_err(|error| format!("failed to read current directory: {error}"))?;
    let words = args
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let generated_config = xpc_request("inject", |message| unsafe {
        xpc_set_string(message, "target", &target.to_string_lossy())?;
        xpc_set_string(message, "cwd", &cwd.to_string_lossy())?;
        xpc_set_string(message, "tool", "aws")?;
        xpc_set_string(message, "aws_profile", &profile)?;
        xpc_set_data(message, "aws_config", &config);
        xpc_set_bool(message, "replace_existing_env", false);
        xpc_set_bool(message, "allow_missing_keys", false);
        xpc_set_array(
            message,
            "keys",
            &[AWS_ACCESS_KEY_ID.into(), AWS_SECRET_ACCESS_KEY.into()],
        )?;
        xpc_set_array(message, "args", &words)?;
        xpc_set_array(message, "env_conflicts", &[])?;
        Ok(())
    })?;
    let config_file = inherited_file(generated_config.as_bytes())?;
    let config_fd = format!("/dev/fd/{}", config_file.as_raw_fd());
    let mut command = Command::new(&target);
    command.args(&args).env_clear();
    for (key, value) in std::env::vars_os() {
        if safe_environment_key(&key) {
            command.env(key, value);
        }
    }
    command
        .env("HOME", "/var/empty")
        .env("AWS_PROFILE", &profile)
        .env("AWS_CONFIG_FILE", config_fd)
        .env("AWS_SHARED_CREDENTIALS_FILE", "/dev/null")
        .env("AWS_EC2_METADATA_DISABLED", "true")
        .env("AWS_PAGER", "")
        .env("PAGER", "cat")
        .env("AWS_CLI_AUTO_PROMPT", "off");
    let error = command.exec();
    drop(config_file);
    Err(format!("failed to execute {}: {error}", target.display()))
}

fn selected_profile(args: &[OsString]) -> Result<String, String> {
    let mut profile = std::env::var("AWS_PROFILE")
        .or_else(|_| std::env::var("AWS_DEFAULT_PROFILE"))
        .unwrap_or_else(|_| "default".into());
    let mut index = 0;
    while index < args.len() {
        let value = args[index]
            .to_str()
            .ok_or_else(|| "AWS arguments must be valid UTF-8".to_string())?;
        if value == "--profile" {
            index += 1;
            profile = args
                .get(index)
                .and_then(|arg| arg.to_str())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "--profile requires a value".to_string())?
                .into();
        } else if let Some(value) = value.strip_prefix("--profile=") {
            if value.is_empty() {
                return Err("--profile requires a value".into());
            }
            profile = value.into();
        }
        index += 1;
    }
    if profile.contains(['\n', '\r', ']']) {
        return Err("invalid AWS profile name".into());
    }
    Ok(profile)
}

fn safe_environment_key(key: &std::ffi::OsStr) -> bool {
    let key = key.to_string_lossy();
    key == "TERM"
        || key == "COLORTERM"
        || key == "TMPDIR"
        || key == "HTTP_PROXY"
        || key == "HTTPS_PROXY"
        || key == "NO_PROXY"
        || key == "http_proxy"
        || key == "https_proxy"
        || key == "no_proxy"
        || key == "SSL_CERT_FILE"
        || key == "SSL_CERT_DIR"
        || key.starts_with("LC_")
        || key == "LANG"
}

fn real_aws_path() -> PathBuf {
    crate::test_env_var("AUTOMIC_VAULT_TEST_REAL_AWS_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(REAL_AWS))
}

fn inherited_file(contents: &[u8]) -> Result<File, String> {
    let mut template = CString::new("/tmp/automic-vault-aws-config.XXXXXX")
        .unwrap()
        .into_bytes_with_nul();
    let descriptor = unsafe { libc::mkstemp(template.as_mut_ptr().cast()) };
    if descriptor == -1 {
        return Err(format!(
            "failed to create AWS config: {}",
            std::io::Error::last_os_error()
        ));
    }
    let mut file = unsafe { File::from_raw_fd(descriptor) };
    if unsafe { libc::unlink(template.as_ptr().cast()) } == -1 {
        return Err(format!(
            "failed to unlink AWS config: {}",
            std::io::Error::last_os_error()
        ));
    }
    file.write_all(contents)
        .and_then(|()| file.rewind())
        .map_err(|error| format!("failed to write AWS config: {error}"))?;
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFD, 0) } == -1 {
        return Err(format!(
            "failed to preserve AWS config: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(file)
}

#[cfg(target_os = "macos")]
type XpcObject = *mut std::ffi::c_void;

#[cfg(target_os = "macos")]
fn xpc_request(
    operation: &str,
    configure: impl FnOnce(XpcObject) -> Result<(), String>,
) -> Result<String, String> {
    use std::ffi::{c_char, c_int, c_void};
    unsafe extern "C" {
        static _xpc_type_error: u8;
        static _xpc_error_key_description: *const c_char;
        fn xpc_connection_create_mach_service(
            name: *const c_char,
            queue: *mut c_void,
            flags: u64,
        ) -> XpcObject;
        fn xpc_connection_activate(connection: XpcObject);
        fn xpc_connection_cancel(connection: XpcObject);
        fn xpc_connection_send_message_with_reply_sync(
            connection: XpcObject,
            message: XpcObject,
        ) -> XpcObject;
        fn xpc_dictionary_create_empty() -> XpcObject;
        fn xpc_dictionary_get_bool(object: XpcObject, key: *const c_char) -> bool;
        fn xpc_dictionary_get_string(object: XpcObject, key: *const c_char) -> *const c_char;
        fn xpc_get_type(object: XpcObject) -> *const c_void;
        fn xpc_release(object: XpcObject);
        fn xpc_connection_set_peer_code_signing_requirement(
            connection: XpcObject,
            requirement: *const c_char,
        ) -> c_int;
        fn av_xpc_connection_set_empty_event_handler(connection: XpcObject);
    }
    let service = c"com.automicvault.av2.approval";
    let connection =
        unsafe { xpc_connection_create_mach_service(service.as_ptr(), std::ptr::null_mut(), 0) };
    if connection.is_null() {
        return Err("failed to create approval XPC connection".into());
    }
    let requirement = CString::new(crate::MENU_HELPER_CODE_SIGNING_REQUIREMENT).unwrap();
    if unsafe { xpc_connection_set_peer_code_signing_requirement(connection, requirement.as_ptr()) }
        != 0
    {
        unsafe { xpc_release(connection) };
        return Err("failed to configure approval XPC signing requirement".into());
    }
    unsafe {
        av_xpc_connection_set_empty_event_handler(connection);
        xpc_connection_activate(connection);
    }
    let message = unsafe { xpc_dictionary_create_empty() };
    unsafe {
        xpc_set_string(message, "op", operation)?;
    }
    if let Err(error) = configure(message) {
        unsafe {
            xpc_release(message);
            xpc_connection_cancel(connection);
            xpc_release(connection);
        }
        return Err(error);
    }
    let reply = unsafe { xpc_connection_send_message_with_reply_sync(connection, message) };
    unsafe {
        xpc_release(message);
        xpc_connection_cancel(connection);
        xpc_release(connection);
    }
    if reply.is_null() {
        return Err("Automic Vault approval did not reply".into());
    }
    let result = unsafe {
        if xpc_get_type(reply) == std::ptr::addr_of!(_xpc_type_error).cast() {
            let value = xpc_dictionary_get_string(reply, _xpc_error_key_description);
            Err(if value.is_null() {
                "approval XPC connection failed".into()
            } else {
                CStr::from_ptr(value).to_string_lossy().into_owned()
            })
        } else if !xpc_dictionary_get_bool(reply, c"ok".as_ptr()) {
            let value = xpc_dictionary_get_string(reply, c"error".as_ptr());
            Err(if value.is_null() {
                "request denied".into()
            } else {
                CStr::from_ptr(value).to_string_lossy().into_owned()
            })
        } else {
            let value = xpc_dictionary_get_string(reply, c"value".as_ptr());
            if value.is_null() {
                Err("Automic Vault returned no AWS value".into())
            } else {
                Ok(CStr::from_ptr(value).to_string_lossy().into_owned())
            }
        }
    };
    unsafe { xpc_release(reply) };
    result
}

#[cfg(not(target_os = "macos"))]
fn xpc_request(
    _operation: &str,
    _configure: impl FnOnce(*mut std::ffi::c_void) -> Result<(), String>,
) -> Result<String, String> {
    Err("AWS credential helper is only available on macOS".into())
}

#[cfg(target_os = "macos")]
unsafe fn xpc_set_string(object: XpcObject, key: &str, value: &str) -> Result<(), String> {
    unsafe extern "C" {
        fn xpc_dictionary_set_string(object: XpcObject, key: *const i8, value: *const i8);
    }
    let key = CString::new(key).unwrap();
    let value = CString::new(value).map_err(|_| "XPC field contains NUL".to_string())?;
    unsafe { xpc_dictionary_set_string(object, key.as_ptr(), value.as_ptr()) };
    Ok(())
}

#[cfg(target_os = "macos")]
unsafe fn xpc_set_bool(object: XpcObject, key: &str, value: bool) {
    unsafe extern "C" {
        fn xpc_dictionary_set_bool(object: XpcObject, key: *const i8, value: bool);
    }
    unsafe { xpc_dictionary_set_bool(object, CString::new(key).unwrap().as_ptr(), value) };
}

#[cfg(target_os = "macos")]
unsafe fn xpc_set_data(object: XpcObject, key: &str, value: &[u8]) {
    unsafe extern "C" {
        fn xpc_dictionary_set_data(
            object: XpcObject,
            key: *const i8,
            value: *const std::ffi::c_void,
            length: usize,
        );
    }
    unsafe {
        xpc_dictionary_set_data(
            object,
            CString::new(key).unwrap().as_ptr(),
            value.as_ptr().cast(),
            value.len(),
        )
    };
}

#[cfg(target_os = "macos")]
unsafe fn xpc_set_array(object: XpcObject, key: &str, values: &[String]) -> Result<(), String> {
    unsafe extern "C" {
        fn xpc_array_create_empty() -> XpcObject;
        fn xpc_array_set_string(array: XpcObject, index: usize, value: *const i8);
        fn xpc_dictionary_set_value(object: XpcObject, key: *const i8, value: XpcObject);
        fn xpc_release(object: XpcObject);
    }
    let array = unsafe { xpc_array_create_empty() };
    for value in values {
        let value =
            CString::new(value.as_str()).map_err(|_| "XPC array contains NUL".to_string())?;
        unsafe { xpc_array_set_string(array, usize::MAX, value.as_ptr()) };
    }
    unsafe {
        xpc_dictionary_set_value(object, CString::new(key).unwrap().as_ptr(), array);
        xpc_release(array);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_precedence_matches_aws() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        unsafe { std::env::set_var("AWS_PROFILE", "env") };
        assert_eq!(
            selected_profile(&["s3".into(), "ls".into()]).unwrap(),
            "env"
        );
        assert_eq!(
            selected_profile(&["--profile".into(), "dev".into(), "s3".into()]).unwrap(),
            "dev"
        );
        assert_eq!(
            selected_profile(&["s3".into(), "--profile=prod".into()]).unwrap(),
            "prod"
        );
        unsafe { std::env::remove_var("AWS_PROFILE") };
    }

    #[test]
    fn ambient_aws_environment_is_not_forwarded() {
        assert!(!safe_environment_key(std::ffi::OsStr::new(
            "AWS_ACCESS_KEY_ID"
        )));
        assert!(!safe_environment_key(std::ffi::OsStr::new("AWS_DATA_PATH")));
        assert!(safe_environment_key(std::ffi::OsStr::new("HTTPS_PROXY")));
    }
}
