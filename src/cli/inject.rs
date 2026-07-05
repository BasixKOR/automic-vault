use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{CString, OsString};
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::credential_helper;

const USAGE: &str = "\
Usage: av inject [--replace-existing-env] [--allow-missing-keys] +KEY [+KEY...] [--] COMMAND [args...]

Injects named Keychain secrets into COMMAND's environment.";

const APPROVAL_SERVICE: &str = "com.automicvault.av2.approval";

#[derive(Debug, PartialEq, Eq)]
struct Options {
    replace_existing_env: bool,
    allow_missing_keys: bool,
    keys: Vec<String>,
    target: OsString,
    args: Vec<OsString>,
    shebang_script: Option<OsString>,
}

#[derive(Debug, PartialEq, Eq)]
struct ApprovalRequest {
    keys: Vec<String>,
    target: String,
    args: Vec<String>,
    cwd: String,
    replace_existing_env: bool,
    allow_missing_keys: bool,
    env_conflicts: Vec<String>,
    shebang_script: Option<String>,
}

unsafe extern "C" {
    fn geteuid() -> u32;
}

pub(crate) fn run(
    args: Vec<OsString>,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    shebang_script: Option<OsString>,
) -> i32 {
    match dispatch(args, stdout, shebang_script) {
        Ok(Some(options)) => exec(options, stderr),
        Ok(None) => 0,
        Err(err) => {
            let _ = writeln!(stderr, "av inject: {err}");
            1
        }
    }
}

fn dispatch(
    args: Vec<OsString>,
    stdout: &mut dyn Write,
    shebang_script: Option<OsString>,
) -> Result<Option<Options>, String> {
    if args
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        writeln!(stdout, "{USAGE}").ok();
        return Ok(None);
    }
    if args
        .first()
        .is_some_and(|arg| arg == "--version" || arg == "-V")
    {
        writeln!(stdout, "av inject {}", env!("CARGO_PKG_VERSION")).ok();
        return Ok(None);
    }
    match parse(args) {
        Ok(mut options) => {
            options.shebang_script = shebang_script.or_else(|| infer_shebang_script(&options));
            Ok(Some(options))
        }
        Err(err) => {
            if err.starts_with("missing ") {
                writeln!(stdout, "{USAGE}").ok();
            }
            Err(err)
        }
    }
}

fn parse(args: Vec<OsString>) -> Result<Options, String> {
    let mut replace_existing_env = false;
    let mut allow_missing_keys = false;
    let mut keys = Vec::new();
    let mut seen_keys = BTreeSet::new();
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        if arg == "--replace-existing-env" {
            replace_existing_env = true;
            continue;
        }
        if arg == "--allow-missing-keys" {
            allow_missing_keys = true;
            continue;
        }
        if arg == "--allow-existing-env" {
            return Err("--allow-existing-env has been replaced by --replace-existing-env".into());
        }
        if arg == "--force" {
            return Err("--force has been replaced by --replace-existing-env".into());
        }
        if arg == "--import" || arg == "--migrate" {
            return Err("credential import and migration are no longer supported".into());
        }

        let value = arg
            .to_str()
            .ok_or_else(|| "inject arguments must be valid UTF-8".to_string())?;
        if let Some(key) = value.strip_prefix('+') {
            validate_key_name(key)?;
            if !seen_keys.insert(key.to_string()) {
                return Err(format!("duplicate key requested: {key}"));
            }
            keys.push(key.to_string());
            continue;
        }

        if arg == "--" {
            if keys.is_empty() {
                return Err("at least one +KEY must be provided before the target".into());
            }
            let target = iter
                .next()
                .ok_or_else(|| "missing target command".to_string())?;
            keys.sort();
            return Ok(Options {
                replace_existing_env,
                allow_missing_keys,
                keys,
                target,
                args: iter.collect(),
                shebang_script: None,
            });
        }

        if keys.is_empty() {
            return Err("at least one +KEY must be provided before the target".into());
        }
        keys.sort();
        return Ok(Options {
            replace_existing_env,
            allow_missing_keys,
            keys,
            target: arg,
            args: iter.collect(),
            shebang_script: None,
        });
    }

    if keys.is_empty() {
        Err("missing key and target command".into())
    } else {
        Err("missing target command".into())
    }
}

fn exec(options: Options, stderr: &mut dyn Write) -> i32 {
    if unsafe { geteuid() } == 0 {
        let _ = writeln!(stderr, "av inject: must not be run as root");
        return 1;
    }

    let (target, env) = match prepare_injection(&options, stderr, approve_injection, build_env) {
        Ok(prepared) => prepared,
        Err(err) => {
            let _ = writeln!(stderr, "av inject: {err}");
            return 1;
        }
    };

    let err = Command::new(&target)
        .args(&options.args)
        .env_clear()
        .envs(env)
        .exec();
    let _ = writeln!(
        stderr,
        "av inject: failed to execute {}: {err}",
        target.display()
    );
    1
}

fn prepare_injection<A, B>(
    options: &Options,
    stderr: &mut dyn Write,
    approve: A,
    build: B,
) -> Result<(PathBuf, BTreeMap<OsString, OsString>), String>
where
    A: FnOnce(&ApprovalRequest) -> Result<(), String>,
    B: FnOnce(&Options, &mut dyn Write) -> Result<BTreeMap<OsString, OsString>, String>,
{
    let target = resolve_target(&options.target)?;
    let request = approval_request(options, &target)?;
    approve(&request)?;
    let env = build(options, stderr)?;
    Ok((target, env))
}

fn approval_request(options: &Options, target: &Path) -> Result<ApprovalRequest, String> {
    let current_env = std::env::vars_os().collect::<BTreeMap<_, _>>();
    let env_conflicts = options
        .keys
        .iter()
        .filter(|key| current_env.contains_key(std::ffi::OsStr::new(key.as_str())))
        .cloned()
        .collect();
    Ok(ApprovalRequest {
        keys: options.keys.clone(),
        target: target.display().to_string(),
        args: options.args.iter().map(os_display).collect(),
        cwd: std::env::current_dir()
            .map_err(|err| format!("failed to read current directory: {err}"))?
            .display()
            .to_string(),
        replace_existing_env: options.replace_existing_env,
        allow_missing_keys: options.allow_missing_keys,
        env_conflicts,
        shebang_script: options.shebang_script.as_ref().map(os_display),
    })
}

fn os_display(value: &OsString) -> String {
    value.to_string_lossy().into_owned()
}

fn infer_shebang_script(options: &Options) -> Option<OsString> {
    let script = options.args.first()?;
    let first_line = std::fs::read_to_string(script)
        .ok()?
        .lines()
        .next()?
        .to_string();
    if first_line.starts_with("#!") && first_line.contains("av inject") {
        Some(script.clone())
    } else {
        None
    }
}

fn build_env(
    options: &Options,
    stderr: &mut dyn Write,
) -> Result<BTreeMap<OsString, OsString>, String> {
    let mut env = std::env::vars_os().collect::<BTreeMap<_, _>>();
    for key in &options.keys {
        if env.contains_key(std::ffi::OsStr::new(key)) && !options.replace_existing_env {
            writeln!(
                stderr,
                "av inject: warning: environment variable {key} is already set; leaving existing value unchanged (replace with: --replace-existing-env)"
            )
            .ok();
            continue;
        }
        match credential_helper::load_secret_if_present(key)? {
            Some(value) => {
                env.insert(OsString::from(key), OsString::from(value));
            }
            None if options.allow_missing_keys => {}
            None => return Err(format!("failed to load isotope key {key}: -25300")),
        }
    }
    Ok(env)
}

fn resolve_target(target: &OsString) -> Result<PathBuf, String> {
    let path = Path::new(target);
    if path.components().count() > 1 {
        if !path.is_absolute() {
            return Err("target binary path must be absolute".into());
        }
        return Ok(path.to_path_buf());
    }
    let paths = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&paths) {
        let candidate = dir.join(path);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "target command not found on PATH: {}",
        path.display()
    ))
}

fn validate_key_name(key: &str) -> Result<(), String> {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return Err("empty isotope key name".into());
    };
    if !(first == '_' || first.is_ascii_alphabetic())
        || chars.any(|ch| !(ch == '_' || ch.is_ascii_alphanumeric()))
    {
        return Err(format!("invalid isotope key name: {key}"));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn approve_injection(request: &ApprovalRequest) -> Result<(), String> {
    if std::env::var_os("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR").is_some() {
        return Ok(());
    }
    xpc_approve_injection(request)
}

#[cfg(not(target_os = "macos"))]
fn approve_injection(_request: &ApprovalRequest) -> Result<(), String> {
    Err("menu bar approval is only available on macOS".into())
}

#[cfg(target_os = "macos")]
fn xpc_approve_injection(request: &ApprovalRequest) -> Result<(), String> {
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
        let value =
            CString::new(value).map_err(|_| format!("XPC field contains NUL: {value:?}"))?;
        unsafe { xpc_dictionary_set_string(dict, key.as_ptr().cast(), value.as_ptr()) };
        Ok(())
    }

    unsafe fn string_array(values: &[String]) -> Result<XpcObject, String> {
        let array = unsafe { xpc_array_create_empty() };
        for value in values {
            let value = CString::new(value.as_str())
                .map_err(|_| format!("XPC array contains NUL: {value:?}"))?;
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

    let menu_requirement = CString::new(r#"identifier "com.automicvault.menubar-helper""#).unwrap();
    let requirement_status = unsafe {
        xpc_connection_set_peer_code_signing_requirement(connection, menu_requirement.as_ptr())
    };
    if requirement_status != 0 {
        unsafe { xpc_release(connection) };
        return Err("failed to configure approval XPC signing requirement".into());
    }

    unsafe {
        av_xpc_connection_set_empty_event_handler(connection);
        xpc_connection_activate(connection);
    }

    let message = unsafe { xpc_dictionary_create_empty() };
    if message.is_null() {
        unsafe { xpc_connection_cancel(connection) };
        unsafe { xpc_release(connection) };
        return Err("failed to create approval XPC message".into());
    }

    unsafe {
        set_string(message, b"op\0", "inject")?;
        set_string(message, b"target\0", &request.target)?;
        set_string(message, b"cwd\0", &request.cwd)?;
        if let Some(script) = &request.shebang_script {
            set_string(message, b"shebang_script\0", script)?;
        }
        xpc_dictionary_set_bool(
            message,
            b"replace_existing_env\0".as_ptr().cast(),
            request.replace_existing_env,
        );
        xpc_dictionary_set_bool(
            message,
            b"allow_missing_keys\0".as_ptr().cast(),
            request.allow_missing_keys,
        );

        let keys = string_array(&request.keys)?;
        xpc_dictionary_set_value(message, b"keys\0".as_ptr().cast(), keys);
        xpc_release(keys);

        let args = string_array(&request.args)?;
        xpc_dictionary_set_value(message, b"args\0".as_ptr().cast(), args);
        xpc_release(args);

        let conflicts = string_array(&request.env_conflicts)?;
        xpc_dictionary_set_value(message, b"env_conflicts\0".as_ptr().cast(), conflicts);
        xpc_release(conflicts);
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
                "injection denied".into()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn os(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    fn temp_script(contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "av-inject-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn parses_old_and_separator_forms() {
        assert_eq!(
            parse(os(&["+B", "+A", "/bin/echo", "hi"])).unwrap(),
            Options {
                replace_existing_env: false,
                allow_missing_keys: false,
                keys: vec!["A".into(), "B".into()],
                target: "/bin/echo".into(),
                args: os(&["hi"]),
                shebang_script: None,
            }
        );
        assert_eq!(
            parse(os(&["--allow-missing-keys", "+A", "--", "env"]))
                .unwrap()
                .target,
            OsString::from("env")
        );
    }

    #[test]
    fn denied_approval_does_not_load_secrets() {
        let options = Options {
            replace_existing_env: false,
            allow_missing_keys: false,
            keys: vec!["SOME_SECRET".into()],
            target: "/bin/echo".into(),
            args: os(&["hi"]),
            shebang_script: None,
        };
        let mut stderr = Vec::new();
        let err = prepare_injection(
            &options,
            &mut stderr,
            |_| Err("user denied injection".into()),
            |_, _| panic!("approval denial must happen before loading secrets"),
        )
        .unwrap_err();

        assert_eq!(err, "user denied injection");
    }

    #[test]
    fn builds_approval_request_without_secret_values() {
        let options = Options {
            replace_existing_env: true,
            allow_missing_keys: true,
            keys: vec!["A".into(), "B".into()],
            target: "/bin/echo".into(),
            args: os(&["hi"]),
            shebang_script: Some("/tmp/tool".into()),
        };
        let request = approval_request(&options, Path::new("/bin/echo")).unwrap();

        assert_eq!(request.keys, ["A", "B"]);
        assert_eq!(request.target, "/bin/echo");
        assert_eq!(request.args, ["hi"]);
        assert!(request.replace_existing_env);
        assert!(request.allow_missing_keys);
        assert_eq!(request.shebang_script.as_deref(), Some("/tmp/tool"));
    }

    #[test]
    fn infers_split_shebang_script_argument() {
        let script = temp_script("#!/usr/local/bin/av inject +A /bin/zsh\n");
        let options = Options {
            replace_existing_env: false,
            allow_missing_keys: false,
            keys: vec!["A".into()],
            target: "/bin/zsh".into(),
            args: vec![script.clone().into_os_string()],
            shebang_script: None,
        };

        assert_eq!(
            infer_shebang_script(&options),
            Some(script.into_os_string())
        );
    }
}
