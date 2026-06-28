use std::ffi::OsString;
use std::io::{IsTerminal, Write};
use std::path::Path;

const USAGE: &str = "Usage: av scan | av inject +KEY [--] COMMAND | av harden [--yes] aws | av harden [--yes] PATH | av credential-helper aws";

mod credential_helper;
mod harden;
mod inject;
mod isotopes;
mod scan;
mod shell_secrets;
mod stub;

pub(crate) fn bash_shell_secret_insecurity_reasons() -> Result<Vec<String>, String> {
    shell_secrets::bash_reasons()
}

pub(crate) fn zsh_shell_secret_insecurity_reasons() -> Result<Vec<String>, String> {
    shell_secrets::zsh_reasons()
}

#[cfg(test)]
pub fn global_test_env_lock() -> &'static std::sync::Mutex<()> {
    &tests::ENV_LOCK
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Finding {
    source: &'static str,
    homepage: &'static str,
    severity: &'static str,
    explanation: String,
    solution: String,
    affected: Vec<AffectedFile>,
    docs_url: &'static str,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AffectedFile {
    path: String,
    line: usize,
}

pub fn run<I, W, E>(args: I, stdout: &mut W, stderr: &mut E) -> i32
where
    I: IntoIterator<Item = OsString>,
    W: Write,
    E: Write,
{
    run_with_style(args, stdout, stderr, scan::Style::plain())
}

pub fn run_terminal<I>(args: I) -> i32
where
    I: IntoIterator<Item = OsString>,
{
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    let color = stdout.is_terminal() && color_enabled();
    run_with_style(args, &mut stdout, &mut stderr, scan::Style { color })
}

fn run_with_style<I, W, E>(args: I, stdout: &mut W, stderr: &mut E, style: scan::Style) -> i32
where
    I: IntoIterator<Item = OsString>,
    W: Write,
    E: Write,
{
    let mut args = args.into_iter();
    let _program = args.next();
    let Some(command) = args.next() else {
        let _ = writeln!(stderr, "{USAGE}");
        return 2;
    };
    let mut rest = args.collect::<Vec<_>>();

    let command = if let Some(words) = split_shebang_inject_arg(&command) {
        rest.splice(0..0, words.into_iter().skip(1));
        OsString::from("inject")
    } else {
        command
    };

    match command.to_str() {
        Some("scan") if rest.is_empty() => scan::run(stdout, style),
        Some("harden") => {
            let Some((target, yes)) = parse_harden_args(&rest) else {
                let _ = writeln!(stderr, "{USAGE}");
                return 2;
            };
            if target == "aws" {
                return match harden::run_aws(stdout, yes) {
                    Ok(()) => 0,
                    Err(err) => {
                        let _ = writeln!(stderr, "av harden: {err}");
                        1
                    }
                };
            }
            match harden::run_stub_install(Path::new(&target), stdout, yes) {
                Ok(()) => 0,
                Err(err) => {
                    let _ = writeln!(stderr, "av harden: {err}");
                    1
                }
            }
        }
        Some("credential-helper") if rest.len() == 1 => {
            let protocol = &rest[0];
            match credential_helper::run(&protocol, stdout) {
                Ok(()) => 0,
                Err(err) => {
                    let _ = writeln!(stderr, "av credential-helper: {err}");
                    1
                }
            }
        }
        Some("inject") => inject::run(rest, stdout, stderr),
        Some("stub-exec") if rest.len() >= 2 => {
            let tool = &rest[0];
            let target = &rest[1];
            match stub::run(tool, target, rest.iter().skip(2).cloned()) {
                Ok(()) => 0,
                Err(err) => {
                    let _ = writeln!(stderr, "av stub: {err}");
                    1
                }
            }
        }
        _ => {
            let _ = writeln!(stderr, "{USAGE}");
            2
        }
    }
}

fn parse_harden_args(args: &[OsString]) -> Option<(OsString, bool)> {
    let mut yes = false;
    let mut target = None;
    for arg in args {
        if arg == "--yes" || arg == "-y" {
            yes = true;
        } else if target.is_none() {
            target = Some(arg.clone());
        } else {
            return None;
        }
    }
    target.map(|target| (target, yes))
}

fn split_shebang_inject_arg(value: &OsString) -> Option<Vec<OsString>> {
    let value = value.to_str()?;
    if value == "inject" || !value.starts_with("inject ") {
        return None;
    }
    Some(value.split_whitespace().map(OsString::from).collect())
}

fn color_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none()
        && std::env::var_os("TERM").is_none_or(|term| term != "dumb")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    pub(crate) static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn run_args(args: &[&str]) -> (i32, String, String) {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = run(args.iter().map(OsString::from), &mut stdout, &mut stderr);
        (
            code,
            String::from_utf8(stdout).unwrap(),
            String::from_utf8(stderr).unwrap(),
        )
    }

    #[test]
    fn scan_prints_clean_report() {
        let (code, stdout, stderr) = run_args(&["av", "scan"]);

        assert_eq!(code, 0);
        assert!(stdout.starts_with("╭─ system exposure audit\n"));
        assert_eq!(stderr, "");
    }

    #[test]
    fn only_scan_is_supported() {
        for args in [&["av"][..], &["av", "harden"], &["av", "scan", "--json"]] {
            let (code, stdout, stderr) = run_args(args);
            assert_eq!(code, 2);
            assert_eq!(stdout, "");
            assert_eq!(stderr, format!("{USAGE}\n"));
        }
    }
}
