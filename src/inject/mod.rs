use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::credential_helper;

const USAGE: &str = "\
Usage: av inject [--replace-existing-env] [--allow-missing-keys] +KEY [+KEY...] [--] COMMAND [args...]

Injects named Keychain secrets into COMMAND's environment.";

#[derive(Debug, PartialEq, Eq)]
struct Options {
    replace_existing_env: bool,
    allow_missing_keys: bool,
    keys: Vec<String>,
    target: OsString,
    args: Vec<OsString>,
}

unsafe extern "C" {
    fn geteuid() -> u32;
}

pub(crate) fn run(args: Vec<OsString>, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match dispatch(args, stdout) {
        Ok(Some(options)) => exec(options, stderr),
        Ok(None) => 0,
        Err(err) => {
            let _ = writeln!(stderr, "av inject: {err}");
            1
        }
    }
}

fn dispatch(args: Vec<OsString>, stdout: &mut dyn Write) -> Result<Option<Options>, String> {
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
        Ok(options) => Ok(Some(options)),
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

    let target = match resolve_target(&options.target) {
        Ok(target) => target,
        Err(err) => {
            let _ = writeln!(stderr, "av inject: {err}");
            return 1;
        }
    };
    let env = match build_env(&options, stderr) {
        Ok(env) => env,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn os(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
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
            }
        );
        assert_eq!(
            parse(os(&["--allow-missing-keys", "+A", "--", "env"]))
                .unwrap()
                .target,
            OsString::from("env")
        );
    }
}
