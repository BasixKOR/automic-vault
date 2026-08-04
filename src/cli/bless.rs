use std::ffi::OsString;
use std::io::Write;

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
    let [path] = args.as_slice() else {
        return Err("usage: av bless PATH".into());
    };
    let path = std::fs::canonicalize(path)
        .map_err(|err| format!("failed to resolve {}: {err}", path.to_string_lossy()))?;
    if !path.is_file() {
        return Err(format!("not a regular file: {}", path.display()));
    }
    let path = path
        .to_str()
        .ok_or_else(|| "script path must be valid UTF-8".to_string())?;
    crate::secrets::bless_script(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bless_requires_exactly_one_path() {
        assert_eq!(run_inner(vec![]).unwrap_err(), "usage: av bless PATH");
        assert_eq!(
            run_inner(vec!["a".into(), "b".into()]).unwrap_err(),
            "usage: av bless PATH"
        );
    }

    #[test]
    fn already_blessed_is_reported_on_stderr() {
        let mut stderr = Vec::new();

        assert_eq!(finish(Ok(true), &mut stderr), 0);
        assert_eq!(stderr, b"already blessed\n");
    }
}
