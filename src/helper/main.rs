use std::ffi::{CStr, CString, c_char, c_void};

use nucleus::{
    HelperCommand, PackageSpec, check_for_updates, execute_helper_command,
    refresh_remote_combined_data, verify_helper_codesign_identity,
};

unsafe extern "C" {
    fn nuke_helper_run_service();
}

type ProgressCallback = extern "C" fn(*mut c_void, *const c_char);

fn main() {
    sanitize_environment();
    if let Err(err) = verify_helper_codesign_identity() {
        eprintln!("{err}");
        std::process::exit(1);
    }
    unsafe { nuke_helper_run_service() };
}

#[unsafe(no_mangle)]
pub extern "C" fn nuke_helper_install(
    packages_json: *const c_char,
    context: *mut c_void,
    progress_callback: Option<ProgressCallback>,
) -> *mut c_char {
    execute_command(
        HelperCommand::Install {
            packages: parse_packages(packages_json),
        },
        context,
        progress_callback,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn nuke_helper_update(
    packages_json: *const c_char,
    context: *mut c_void,
    progress_callback: Option<ProgressCallback>,
) -> *mut c_char {
    execute_command(
        HelperCommand::Update {
            packages: parse_packages(packages_json),
        },
        context,
        progress_callback,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn nuke_helper_uninstall(
    packages_json: *const c_char,
    context: *mut c_void,
    progress_callback: Option<ProgressCallback>,
) -> *mut c_char {
    execute_command(
        HelperCommand::Uninstall {
            packages: parse_packages(packages_json),
        },
        context,
        progress_callback,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn nuke_helper_update_all(
    context: *mut c_void,
    progress_callback: Option<ProgressCallback>,
) -> *mut c_char {
    execute_command(HelperCommand::UpdateAll, context, progress_callback)
}

#[unsafe(no_mangle)]
pub extern "C" fn nuke_helper_install_av(
    source_path: *const c_char,
    caller_path: *const c_char,
    context: *mut c_void,
    progress_callback: Option<ProgressCallback>,
) -> *mut c_char {
    let source_path = c_string(source_path).unwrap_or_default();
    let caller_path = c_string(caller_path).unwrap_or_default();
    execute_command(
        HelperCommand::InstallAv {
            source_path,
            caller_path,
        },
        context,
        progress_callback,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn nuke_helper_install_isotope_root(
    isotope_name: *const c_char,
    context: *mut c_void,
    progress_callback: Option<ProgressCallback>,
) -> *mut c_char {
    let isotope_name = c_string(isotope_name).unwrap_or_default();
    execute_command(
        HelperCommand::InstallIsotopeRoot { isotope_name },
        context,
        progress_callback,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn nuke_helper_convert_radioisotope(
    isotope_name: *const c_char,
    context: *mut c_void,
    progress_callback: Option<ProgressCallback>,
) -> *mut c_char {
    let isotope_name = c_string(isotope_name).unwrap_or_default();
    execute_command(
        HelperCommand::ConvertRadioisotope { isotope_name },
        context,
        progress_callback,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn nuke_helper_install_isotope_stubs(
    isotope_name: *const c_char,
    context: *mut c_void,
    progress_callback: Option<ProgressCallback>,
) -> *mut c_char {
    let isotope_name = c_string(isotope_name).unwrap_or_default();
    execute_command(
        HelperCommand::InstallIsotopeStubs { isotope_name },
        context,
        progress_callback,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn nuke_helper_remember_isotope_always_allow(
    executable_path: *const c_char,
    script_path: *const c_char,
    keys_json: *const c_char,
    context: *mut c_void,
    progress_callback: Option<ProgressCallback>,
) -> *mut c_char {
    let executable_path = c_string(executable_path).unwrap_or_default();
    let script_path = c_string(script_path).unwrap_or_default();
    let keys = parse_string_array(keys_json);
    execute_command(
        HelperCommand::RememberIsotopeAlwaysAllow {
            executable_path,
            script_path: if script_path.is_empty() {
                None
            } else {
                Some(script_path)
            },
            keys,
        },
        context,
        progress_callback,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn nuke_helper_check_for_updates() -> bool {
    if verify_helper_codesign_identity().is_err() {
        return false;
    }
    let _ = refresh_remote_combined_data();
    check_for_updates().unwrap_or(false)
}

#[unsafe(no_mangle)]
pub extern "C" fn nuke_helper_refresh_remote_database() -> bool {
    if verify_helper_codesign_identity().is_err() {
        return false;
    }
    refresh_remote_combined_data().unwrap_or(false)
}

#[unsafe(no_mangle)]
pub extern "C" fn nuke_helper_free_string(value: *mut c_char) {
    if value.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(value);
    }
}

fn execute_command(
    command: HelperCommand,
    context: *mut c_void,
    progress_callback: Option<ProgressCallback>,
) -> *mut c_char {
    if let Err(err) = verify_helper_codesign_identity() {
        return encode_error(err);
    }
    let context = context as usize;
    let result = execute_helper_command(command, move |event| {
        let Some(progress_callback) = progress_callback else {
            return;
        };
        let event_json = match serde_json::to_string(&event) {
            Ok(event_json) => event_json,
            Err(_) => return,
        };
        if let Ok(c_string) = CString::new(event_json) {
            progress_callback(context as *mut c_void, c_string.as_ptr());
        }
    });

    match serde_json::to_string(&result) {
        Ok(json) => string_into_raw(json),
        Err(err) => string_into_raw(format!(
            r#"{{"Err":"failed to encode helper result: {err}"}}"#
        )),
    }
}

fn parse_packages(packages_json: *const c_char) -> Vec<PackageSpec> {
    let Ok(packages_json) = c_string(packages_json) else {
        return Vec::new();
    };
    serde_json::from_str(&packages_json).unwrap_or_default()
}

fn parse_string_array(values_json: *const c_char) -> Vec<String> {
    let Ok(values_json) = c_string(values_json) else {
        return Vec::new();
    };
    serde_json::from_str(&values_json).unwrap_or_default()
}

fn c_string(value: *const c_char) -> Result<String, std::str::Utf8Error> {
    if value.is_null() {
        return Ok(String::new());
    }
    unsafe { CStr::from_ptr(value) }.to_str().map(str::to_owned)
}

fn string_into_raw(value: String) -> *mut c_char {
    CString::new(value).unwrap().into_raw()
}

fn encode_error(message: String) -> *mut c_char {
    match serde_json::to_string(&Err::<HelperCommandSuccessWire, _>(message)) {
        Ok(json) => string_into_raw(json),
        Err(_) => string_into_raw(r#"{"Err":"helper identity check failed"}"#.to_string()),
    }
}

#[derive(serde::Serialize)]
struct HelperCommandSuccessWire {
    message: String,
    processed_packages: Vec<String>,
}

fn sanitize_environment() {
    for key in ["PKG_ALLOW", "PACKAGE_MAGINAT0R_LVL", "HOMEBREW_PREFIX"] {
        unsafe { std::env::remove_var(key) };
    }
}
