use std::ffi::OsString;
use std::io::{IsTerminal, Write};

mod doctor;
mod inject;
mod open;
mod save;
mod scan;
mod shell_secrets;

use crate::isotopes::hardeners;

const USAGE: &str = "\
Usage:
  av scan [--show-all | --json]
  av doctor [COMMAND] [--json]
  av detectors --json
  av hardeners --json
  av inject +KEY [--] COMMAND
  av save KEY
  av harden";

pub(crate) fn bash_shell_secret_insecurity_reasons() -> Result<Vec<String>, String> {
    shell_secrets::bash_reasons()
}

pub(crate) fn zsh_shell_secret_insecurity_reasons() -> Result<Vec<String>, String> {
    shell_secrets::zsh_reasons()
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
    if command == "--help" || command == "-h" {
        let _ = writeln!(stdout, "{USAGE}");
        return 0;
    }
    let mut rest = args.collect::<Vec<_>>();

    let mut shebang_script = None;
    let command = if let Some(words) = split_shebang_inject_arg(&command) {
        shebang_script = rest.first().cloned();
        rest.splice(0..0, words.into_iter().skip(1));
        OsString::from("inject")
    } else {
        command
    };

    match command.to_str() {
        Some("scan") if rest.is_empty() => scan::run(stdout, style, false),
        Some("scan") if rest == [OsString::from("--show-all")] => scan::run(stdout, style, true),
        Some("scan") if rest == [OsString::from("--json")] => scan::run_json(stdout),
        Some("detectors") if rest == [OsString::from("--json")] => scan::run_detectors_json(stdout),
        Some("hardeners") if rest == [OsString::from("--json")] => scan::run_hardeners_json(stdout),
        Some("doctor") => {
            let Some((selector, json)) = parse_doctor_args(&rest) else {
                let _ = writeln!(stderr, "{USAGE}");
                return 2;
            };
            match doctor::run(stdout, selector.as_deref(), json, style) {
                Ok(code) => code,
                Err(err) => {
                    let _ = writeln!(stderr, "av doctor: {err}");
                    2
                }
            }
        }
        Some("harden") => {
            let Some((target, yes)) = parse_harden_args(&rest) else {
                let _ = writeln!(stderr, "{USAGE}");
                return 2;
            };
            if target == "aws" {
                return match hardeners::aws_cli::run_aws(stdout, yes) {
                    Ok(()) => 0,
                    Err(err) => {
                        let _ = writeln!(stderr, "av harden: {err}");
                        1
                    }
                };
            }
            if target == "gh" || target == "gh-cli" {
                return match hardeners::gh_cli::run(stdout, yes) {
                    Ok(()) => 0,
                    Err(err) => {
                        let _ = writeln!(stderr, "av harden: {err}");
                        1
                    }
                };
            }
            if target == "brew" || target == "homebrew" {
                return match hardeners::homebrew::run(stdout, yes) {
                    Ok(()) => 0,
                    Err(err) => {
                        let _ = writeln!(stderr, "av harden: {err}");
                        1
                    }
                };
            }
            if target == "sudo" {
                return match hardeners::sudo::run(stdout, style.color) {
                    Ok(()) => 0,
                    Err(err) => {
                        let _ = writeln!(stderr, "av harden: {err}");
                        1
                    }
                };
            }
            if target == "supabase" || target == "supabase-cli" {
                return match hardeners::supabase::run(stdout, yes) {
                    Ok(()) => 0,
                    Err(err) => {
                        let _ = writeln!(stderr, "av harden: {err}");
                        1
                    }
                };
            }
            if let Some(result) = target
                .to_str()
                .and_then(|target| hardeners::env_wrapper::run_target(target, stdout, yes))
            {
                return match result {
                    Ok(()) => 0,
                    Err(err) => {
                        let _ = writeln!(stderr, "av harden: {err}");
                        1
                    }
                };
            }
            let _ = writeln!(stderr, "{USAGE}");
            2
        }
        Some("inject") => inject::run(rest, stdout, stderr, shebang_script),
        Some("open") if rest.is_empty() => open::run(stderr),
        Some("save") => save::run(rest, stderr),
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

fn parse_doctor_args(args: &[OsString]) -> Option<(Option<String>, bool)> {
    let mut selector = None;
    let mut json = false;
    for arg in args {
        if arg == "--json" {
            if json {
                return None;
            }
            json = true;
        } else if selector.is_none() {
            selector = Some(arg.to_str()?.to_string());
        } else {
            return None;
        }
    }
    Some((selector, json))
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
pub(crate) static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

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
    fn scan_show_all_is_supported() {
        let (code, _, stderr) = run_args(&["av", "scan", "--show-all"]);

        assert_eq!(code, 0);
        assert_eq!(stderr, "");
    }

    #[test]
    fn harden_gh_tells_user_to_install_isotope() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let missing = std::env::temp_dir().join(format!("av-missing-gh-{}", std::process::id()));
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_GH_CLI_PATH", &missing);
        }

        let (code, stdout, stderr) = run_args(&["av", "harden", "gh"]);

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_GH_CLI_PATH");
        }
        assert_eq!(code, 1);
        assert_eq!(stdout, "");
        assert_eq!(
            stderr,
            "av harden: gh-cli is not installed; run `brew install automic-vault/isotopes/gh-cli`\n"
        );
    }

    #[test]
    fn harden_brew_is_routed() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let missing = std::env::temp_dir().join(format!("av-missing-brew-{}", std::process::id()));
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_BREW_TARGET", &missing);
            std::env::set_var("AUTOMIC_VAULT_TEST_EUID", "0");
        }

        let (code, stdout, stderr) = run_args(&["av", "harden", "brew"]);

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_BREW_TARGET");
            std::env::remove_var("AUTOMIC_VAULT_TEST_EUID");
        }
        assert_eq!(code, 1);
        assert_eq!(stdout, "");
        assert_eq!(
            stderr,
            format!(
                "av harden: Homebrew is not installed at {}\n",
                missing.display()
            )
        );
    }

    #[test]
    fn harden_sudo_prints_touch_id_command() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let pam = std::env::temp_dir().join(format!("av-cli-sudo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pam);
        std::fs::create_dir_all(&pam).unwrap();
        std::fs::write(pam.join("sudo_local"), "#auth sufficient pam_tid.so\n").unwrap();
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_SUDO_PAM_DIR", &pam);
        }

        let (code, stdout, stderr) = run_args(&["av", "harden", "sudo"]);

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_SUDO_PAM_DIR");
        }
        let _ = std::fs::remove_dir_all(pam);
        assert_eq!(code, 0);
        assert_eq!(
            stdout,
            "╭─ harden sudo\n│\n◇ enables biometric authentication for sudo\n│\n╰─ run:\n\n        echo 'auth sufficient pam_tid.so' | sudo tee -a /etc/pam.d/sudo_local >/dev/null\n"
        );
        assert_eq!(stderr, "");
    }

    #[test]
    fn only_scan_is_supported() {
        for args in [&["av"][..], &["av", "harden"], &["av", "scan", "--bad"]] {
            let (code, stdout, stderr) = run_args(args);
            assert_eq!(code, 2);
            assert_eq!(stdout, "");
            assert_eq!(stderr, format!("{USAGE}\n"));
        }
    }

    #[test]
    fn help_prints_one_command_per_line() {
        let (code, stdout, stderr) = run_args(&["av", "--help"]);

        assert_eq!(code, 0);
        assert_eq!(stdout, format!("{USAGE}\n"));
        assert_eq!(stderr, "");
        assert!(stdout.contains("\n  av harden\n"));
        assert!(!stdout.contains("av harden [--yes]"));
        assert!(!stdout.contains("av open"));
    }

    #[test]
    fn detectors_json_is_supported() {
        let (code, stdout, stderr) = run_args(&["av", "detectors", "--json"]);

        assert_eq!(code, 0);
        assert!(stdout.contains(r#""detectors":["#));
        assert_eq!(stderr, "");
    }
}
