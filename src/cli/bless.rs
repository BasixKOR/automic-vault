use std::ffi::OsString;
use std::io::Write;

const USAGE: &str = "usage: av bless [--endorse-launcher] PATH";

pub(crate) fn run(args: Vec<OsString>, stderr: &mut dyn Write) -> i32 {
    finish(run_inner(args), stderr)
}

fn finish(result: Result<bool, String>, stderr: &mut dyn Write) -> i32 {
    match result {
        Ok(already_blessed) => {
            if already_blessed {
                let _ = writeln!(stderr, "already blessed");
            }
            0
        }
        Err(err) => {
            let _ = writeln!(stderr, "av bless: {err}");
            1
        }
    }
}

fn run_inner(args: Vec<OsString>) -> Result<bool, String> {
    let (path, endorse_launcher) = parse_args(&args)?;
    let path = std::fs::canonicalize(path)
        .map_err(|err| format!("failed to resolve {}: {err}", path.to_string_lossy()))?;
    if !path.is_file() {
        return Err(format!("not a regular file: {}", path.display()));
    }
    let path = path
        .to_str()
        .ok_or_else(|| "script path must be valid UTF-8".to_string())?;
    crate::secrets::bless_script(path, endorse_launcher)
}

fn parse_args(args: &[OsString]) -> Result<(&OsString, bool), String> {
    let (path, endorse_launcher) = match args {
        [path] if !path.to_string_lossy().starts_with('-') => (path, false),
        [flag, path] if flag == "--endorse-launcher" || flag == "--endorse-caller" => (path, true),
        _ => return Err(USAGE.into()),
    };
    Ok((path, endorse_launcher))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bless_arguments_make_launcher_endorsement_explicit() {
        assert_eq!(parse_args(&[]).unwrap_err(), USAGE);
        assert_eq!(parse_args(&["a".into(), "b".into()]).unwrap_err(), USAGE);
        assert_eq!(parse_args(&["--unknown".into()]).unwrap_err(), USAGE);

        let args = ["script".into()];
        assert_eq!(parse_args(&args).unwrap(), (&args[0], false));
        let args = ["--endorse-launcher".into(), "script".into()];
        assert_eq!(parse_args(&args).unwrap(), (&args[1], true));
    }

    #[test]
    fn bless_accepts_legacy_caller_endorsement_flag() {
        let args = ["--endorse-caller".into(), "script".into()];
        assert_eq!(parse_args(&args).unwrap(), (&args[1], true));
    }

    #[test]
    fn already_blessed_is_reported_on_stderr() {
        let mut stderr = Vec::new();

        assert_eq!(finish(Ok(true), &mut stderr), 0);
        assert_eq!(stderr, b"already blessed\n");
    }
}
