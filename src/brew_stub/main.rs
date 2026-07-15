use std::ffi::{CString, OsString};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

const MARKER: &str = "AUTOMIC_VAULT_BREW_STUB_V1";
const TARGET: &str = "/opt/homebrew/bin/brew";
const APPROVAL_SERVICE: &str = "com.automicvault.av2.approval";

#[derive(Debug, PartialEq, Eq)]
struct AuthorizationRequest {
    target: String,
    args: Vec<String>,
    cwd: String,
}

fn main() {
    if std::env::args().any(|arg| arg == "--automic-vault-brew-stub-marker") {
        println!("{MARKER}");
        return;
    }

    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(err) => fail(format!("failed to read current directory: {err}")),
    };
    let mut command = approved_command(args, std::env::vars_os(), &cwd, xpc_authorize)
        .unwrap_or_else(|err| fail(err));
    let err = command.exec();
    fail(format!("failed to exec {TARGET}: {err}"));
}

fn fail(message: String) -> ! {
    eprintln!("av-brew-stub: {message}");
    std::process::exit(1);
}

fn approved_command<I, F>(
    args: Vec<OsString>,
    source_env: I,
    cwd: &Path,
    approve: F,
) -> Result<Command, String>
where
    I: IntoIterator<Item = (OsString, OsString)>,
    F: FnOnce(&AuthorizationRequest) -> Result<(), String>,
{
    let request = authorization_request(&args, cwd)?;
    approve(&request)?;
    let mut command = Command::new(TARGET);
    command.args(args).env_clear().envs(stub_env(source_env));
    Ok(command)
}

fn authorization_request(args: &[OsString], cwd: &Path) -> Result<AuthorizationRequest, String> {
    let args = args
        .iter()
        .map(|arg| {
            arg.to_str()
                .map(str::to_string)
                .ok_or_else(|| "brew arguments must be valid UTF-8".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let cwd = cwd
        .to_str()
        .ok_or_else(|| "current directory must be valid UTF-8".to_string())?;
    Ok(AuthorizationRequest {
        target: TARGET.to_string(),
        args,
        cwd: cwd.to_string(),
    })
}

#[cfg(target_os = "macos")]
fn xpc_authorize(request: &AuthorizationRequest) -> Result<(), String> {
    use std::os::raw::{c_char, c_int, c_void};

    type XpcObject = *mut c_void;

    unsafe extern "C" {
        static _xpc_type_error: u8;
        static _xpc_error_key_description: *const c_char;

        fn xpc_connection_create_mach_service(
            name: *const c_char,
            targetq: *mut c_void,
            flags: u64,
        ) -> XpcObject;
        fn xpc_connection_activate(connection: XpcObject);
        fn xpc_connection_cancel(connection: XpcObject);
        fn xpc_connection_send_message_with_reply_sync(
            connection: XpcObject,
            message: XpcObject,
        ) -> XpcObject;
        fn xpc_dictionary_create_empty() -> XpcObject;
        fn xpc_dictionary_set_bool(xdict: XpcObject, key: *const c_char, value: bool);
        fn xpc_dictionary_get_bool(xdict: XpcObject, key: *const c_char) -> bool;
        fn xpc_dictionary_set_string(xdict: XpcObject, key: *const c_char, value: *const c_char);
        fn xpc_dictionary_get_string(xdict: XpcObject, key: *const c_char) -> *const c_char;
        fn xpc_dictionary_set_value(xdict: XpcObject, key: *const c_char, value: XpcObject);
        fn xpc_array_create_empty() -> XpcObject;
        fn xpc_array_append_value(xarray: XpcObject, value: XpcObject);
        fn xpc_string_create(string: *const c_char) -> XpcObject;
        fn xpc_get_type(object: XpcObject) -> *const c_void;
        fn xpc_release(object: XpcObject);
        fn xpc_connection_set_peer_code_signing_requirement(
            connection: XpcObject,
            requirement: *const c_char,
        ) -> c_int;
        fn av_xpc_connection_set_empty_event_handler(connection: XpcObject);
    }

    unsafe fn set_string(dict: XpcObject, key: &[u8], value: &str) -> Result<(), String> {
        let value = CString::new(value).map_err(|_| "XPC field contains NUL".to_string())?;
        unsafe { xpc_dictionary_set_string(dict, key.as_ptr().cast(), value.as_ptr()) };
        Ok(())
    }

    unsafe fn string_array(values: &[String]) -> Result<XpcObject, String> {
        let array = unsafe { xpc_array_create_empty() };
        if array.is_null() {
            return Err("failed to create approval XPC array".into());
        }
        for value in values {
            let value =
                CString::new(value.as_str()).map_err(|_| "XPC array contains NUL".to_string())?;
            let string = unsafe { xpc_string_create(value.as_ptr()) };
            unsafe {
                xpc_array_append_value(array, string);
                xpc_release(string);
            }
        }
        Ok(array)
    }

    let service = CString::new(APPROVAL_SERVICE).unwrap();
    let connection =
        unsafe { xpc_connection_create_mach_service(service.as_ptr(), std::ptr::null_mut(), 0) };
    if connection.is_null() {
        return Err("failed to create approval XPC connection".into());
    }

    let menu_requirement = CString::new(av::MENU_HELPER_CODE_SIGNING_REQUIREMENT).unwrap();
    if unsafe {
        xpc_connection_set_peer_code_signing_requirement(connection, menu_requirement.as_ptr())
    } != 0
    {
        unsafe { xpc_release(connection) };
        return Err("failed to configure approval XPC signing requirement".into());
    }

    unsafe {
        av_xpc_connection_set_empty_event_handler(connection);
        xpc_connection_activate(connection);
    }

    let message = unsafe { xpc_dictionary_create_empty() };
    if message.is_null() {
        unsafe {
            xpc_connection_cancel(connection);
            xpc_release(connection);
        }
        return Err("failed to create approval XPC message".into());
    }

    let empty = unsafe { string_array(&[]) }?;
    let args = unsafe { string_array(&request.args) }?;
    unsafe {
        set_string(message, b"op\0", "authorize")?;
        set_string(message, b"target\0", &request.target)?;
        set_string(message, b"cwd\0", &request.cwd)?;
        set_string(message, b"tool\0", "brew")?;
        xpc_dictionary_set_bool(message, b"replace_existing_env\0".as_ptr().cast(), false);
        xpc_dictionary_set_bool(message, b"allow_missing_keys\0".as_ptr().cast(), false);
        xpc_dictionary_set_value(message, b"keys\0".as_ptr().cast(), empty);
        xpc_dictionary_set_value(message, b"args\0".as_ptr().cast(), args);
        xpc_dictionary_set_value(message, b"env_conflicts\0".as_ptr().cast(), empty);
        xpc_release(empty);
        xpc_release(args);
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
            let error = xpc_dictionary_get_string(reply, _xpc_error_key_description);
            let error = if error.is_null() {
                "approval XPC connection failed".into()
            } else {
                std::ffi::CStr::from_ptr(error)
                    .to_string_lossy()
                    .into_owned()
            };
            if error == "Connection invalid" {
                Err("Automic Vault approval service is not running; open the menu bar app".into())
            } else {
                Err(error)
            }
        } else if xpc_dictionary_get_bool(reply, b"ok\0".as_ptr().cast()) {
            Ok(())
        } else {
            let error = xpc_dictionary_get_string(reply, b"error\0".as_ptr().cast());
            Err(if error.is_null() {
                "brew authorization denied".into()
            } else {
                std::ffi::CStr::from_ptr(error)
                    .to_string_lossy()
                    .into_owned()
            })
        }
    };
    unsafe { xpc_release(reply) };
    result
}

#[cfg(not(target_os = "macos"))]
fn xpc_authorize(_request: &AuthorizationRequest) -> Result<(), String> {
    Err("menu bar approval is only available on macOS".into())
}

fn stub_env<I>(source: I) -> Vec<(OsString, OsString)>
where
    I: IntoIterator<Item = (OsString, OsString)>,
{
    let mut env = vec![
        ("HOME".into(), "/opt/homebrew/var/automic".into()),
        ("USER".into(), "automic".into()),
        ("LOGNAME".into(), "automic".into()),
        (
            "PATH".into(),
            "/opt/homebrew/bin:/opt/homebrew/sbin:/usr/bin:/bin:/usr/sbin:/sbin".into(),
        ),
        ("TMPDIR".into(), "/opt/homebrew/var/automic/tmp".into()),
        (
            "HOMEBREW_CACHE".into(),
            "/opt/homebrew/var/automic/cache".into(),
        ),
        ("AUTOMIC_VAULT_BREW_STUB".into(), MARKER.into()),
    ];

    for (key, value) in source {
        let Some(key_str) = key.to_str() else {
            continue;
        };
        if key_str == "TERM"
            || key_str == "LANG"
            || key_str == "NO_COLOR"
            || key_str.starts_with("LC_")
        {
            env.push((key, value));
        }
    }
    env
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorization_request_keeps_exact_args_and_cwd() {
        let request = authorization_request(
            &[
                "install".into(),
                "--cask".into(),
                "Visual Studio Code".into(),
            ],
            Path::new("/tmp/a project"),
        )
        .unwrap();

        assert_eq!(request.target, TARGET);
        assert_eq!(request.args, ["install", "--cask", "Visual Studio Code"]);
        assert_eq!(request.cwd, "/tmp/a project");
    }

    #[test]
    fn denial_prevents_command_creation() {
        let result = approved_command(
            vec!["install".into(), "ack".into()],
            [],
            Path::new("/tmp"),
            |_| Err("denied".into()),
        );

        assert_eq!(result.unwrap_err().to_string(), "denied");
    }

    #[test]
    fn approved_command_has_sanitized_env() {
        let command = approved_command(
            vec!["info".into(), "ack".into()],
            [
                ("TERM".into(), "xterm-256color".into()),
                ("SECRET".into(), "nope".into()),
            ],
            Path::new("/tmp"),
            |_| Ok(()),
        )
        .unwrap();
        let env = command
            .get_envs()
            .map(|(key, value)| (key.to_owned(), value.unwrap().to_owned()))
            .collect::<Vec<_>>();

        assert!(env.contains(&("HOME".into(), "/opt/homebrew/var/automic".into())));
        assert!(env.contains(&("TERM".into(), "xterm-256color".into())));
        assert!(!env.iter().any(|(key, _)| key == "SECRET"));
    }

    #[test]
    fn stub_env_keeps_only_safe_user_env() {
        let env = stub_env([
            ("TERM".into(), "xterm-256color".into()),
            ("LANG".into(), "en_US.UTF-8".into()),
            ("LC_ALL".into(), "C".into()),
            ("NO_COLOR".into(), "1".into()),
            ("HOMEBREW_PREFIX".into(), "/tmp/bad".into()),
            ("PATH".into(), "/tmp/bad".into()),
        ]);

        assert!(env.contains(&("HOME".into(), "/opt/homebrew/var/automic".into())));
        assert!(env.contains(&("USER".into(), "automic".into())));
        assert!(env.contains(&("LOGNAME".into(), "automic".into())));
        assert!(env.contains(&("TERM".into(), "xterm-256color".into())));
        assert!(env.contains(&("LC_ALL".into(), "C".into())));
        assert!(!env.contains(&("HOMEBREW_PREFIX".into(), "/tmp/bad".into())));
        assert!(!env.contains(&("PATH".into(), "/tmp/bad".into())));
    }
}
