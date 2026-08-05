use std::ffi::OsString;
use std::io::{IsTerminal, Write};

mod aws;
mod bless;
mod doctor;
mod inject;
mod list;
mod open;
mod save;
mod scan;
mod shell_secrets;

use crate::isotopes::hardeners;

const USAGE: &str = "\
usage:
  av <command> [options]

commands:
  $ av scan [--show-all|--json]           # audit secrets and configuration
  $ av doctor [<tool>] [--json]           # verify installed hardening
  $ av detectors --json                   # print detector metadata
  $ av hardeners --json                   # print hardener metadata
  $ av bless [--endorse-caller] <path>    # approve a script for secret access
  $ av inject +KEY... [--] <command>      # inject secrets into a command
  $ av inject -- <command>                # run an approved script
  $ av list                               # list saved secret names
  $ av save <key>                         # store a secret
  $ av harden <tool> [-y|--yes]           # harden a tool; migrate credentials
  $ av open [--secret-gate <id>]          # open the Automic Vault app

modes:
  $ av help                               # show this help
  $ av --version                          # print version

more:
  $ open https://www.automicvault.com/docs/";

const INSTALL_REVISION: u32 = 13;

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
    run_with_style(
        args,
        stdout,
        stderr,
        scan::Style::plain(),
        scan::Style::plain(),
    )
}

pub fn run_terminal<I>(args: I) -> i32
where
    I: IntoIterator<Item = OsString>,
{
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    let terminal = stdout.is_terminal();
    let color = terminal && color_enabled();
    run_with_style(
        args,
        &mut stdout,
        &mut stderr,
        scan::Style { color },
        scan::Style { color: terminal },
    )
}

pub fn run_scanner_terminal<I>(args: I) -> i32
where
    I: IntoIterator<Item = OsString>,
{
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    let terminal = stdout.is_terminal();
    run_scanner_with_style(
        args,
        &mut stdout,
        &mut stderr,
        scan::Style {
            color: terminal && color_enabled(),
        },
    )
}

fn run_scanner_with_style<I, W, E>(
    args: I,
    stdout: &mut W,
    stderr: &mut E,
    style: scan::Style,
) -> i32
where
    I: IntoIterator<Item = OsString>,
    W: Write,
    E: Write,
{
    let mut args = args.into_iter();
    let _program = args.next();
    match args.collect::<Vec<_>>().as_slice() {
        [] => scan::run(stdout, style, false),
        [arg] if arg == "--show-all" => scan::run(stdout, style, true),
        [arg] if arg == "--json" => scan::run_json(stdout),
        [arg] if arg == "--version" || arg == "-V" => {
            let _ = writeln!(stdout, "scanner {}", env!("CARGO_PKG_VERSION"));
            0
        }
        [arg] if arg == "--help" || arg == "-h" => {
            let _ = writeln!(stdout, "usage: scanner [--show-all|--json]");
            0
        }
        _ => {
            let _ = writeln!(stderr, "usage: scanner [--show-all|--json]");
            2
        }
    }
}

fn run_with_style<I, W, E>(
    args: I,
    stdout: &mut W,
    stderr: &mut E,
    style: scan::Style,
    help_style: scan::Style,
) -> i32
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
    if command == "help" || command == "--help" || command == "-h" {
        write_help(stdout, help_style);
        return 0;
    }
    if command == "--version" || command == "-V" {
        let _ = writeln!(stdout, "av {}", env!("CARGO_PKG_VERSION"));
        return 0;
    }
    if command == "__version" {
        let _ = writeln!(stdout, "{INSTALL_REVISION}");
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
        Some("__install-env-wrapper") if rest.len() == 1 => {
            let Some(target) = rest[0].to_str() else {
                let _ = writeln!(stderr, "av: invalid env-wrapper hardener name");
                return 2;
            };
            match hardeners::env_wrapper::install_target(target) {
                Ok(()) => 0,
                Err(err) => {
                    let _ = writeln!(stderr, "av: {err}");
                    1
                }
            }
        }
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
            let target = if target == "fly" {
                OsString::from("flyctl")
            } else {
                target
            };
            if target == "aws" {
                let result = hardeners::aws_cli::run_aws(stdout, yes);
                return finish_hardening(result, "aws", stdout, stderr);
            }
            if target == "gh" || target == "gh-cli" {
                return match hardeners::gh_cli::run(stdout, yes) {
                    Ok(()) => {
                        print_hardening_followup(stdout, "gh");
                        0
                    }
                    Err(hardeners::gh_cli::HardenError::GhCliNotInstalled) => {
                        print_isotope_install_guidance(
                            stderr,
                            style,
                            "gh",
                            "gh-cli",
                            hardeners::gh_cli::INSTALL_COMMAND,
                        );
                        1
                    }
                    Err(hardeners::gh_cli::HardenError::Other(err)) => {
                        let _ = writeln!(stderr, "av harden: {err}");
                        1
                    }
                };
            }
            if target == "stripe" || target == "stripe-cli" {
                return match hardeners::stripe_cli::run(stdout, yes) {
                    Ok(()) => {
                        print_hardening_followup(stdout, "stripe");
                        0
                    }
                    Err(hardeners::stripe_cli::HardenError::StripeCliNotInstalled) => {
                        print_isotope_install_guidance(
                            stderr,
                            style,
                            "stripe",
                            "stripe-cli",
                            hardeners::stripe_cli::INSTALL_COMMAND,
                        );
                        1
                    }
                    Err(hardeners::stripe_cli::HardenError::Other(err)) => {
                        let _ = writeln!(stderr, "av harden: {err}");
                        1
                    }
                };
            }
            if target == "brew" || target == "homebrew" {
                let result = hardeners::homebrew::run(stdout, yes);
                return finish_hardening(result, "brew", stdout, stderr);
            }
            if target == "sudo" {
                let result = hardeners::sudo::run(stdout, style.color);
                return finish_hardening(result, "sudo", stdout, stderr);
            }
            if target == "supabase" || target == "supabase-cli" {
                let result = hardeners::supabase::run(stdout, yes);
                return finish_hardening(result, "supabase", stdout, stderr);
            }
            if let Some(target) = target.to_str()
                && let Some(result) = hardeners::env_wrapper::run_target(target, stdout, yes)
            {
                return finish_hardening(result, target, stdout, stderr);
            }
            let _ = writeln!(stderr, "{USAGE}");
            2
        }
        Some("inject") => inject::run(rest, stdout, stderr, shebang_script),
        Some("aws") => aws::run(rest, stderr),
        Some("aws-credentials") if rest.is_empty() => aws::credentials(stdout, stderr),
        Some("list" | "ls") => list::run(rest, stdout, stderr),
        Some("bless") => bless::run(rest, stderr),
        Some("open") => {
            let Some(secret_gate) = parse_open_args(&rest) else {
                let _ = writeln!(stderr, "{USAGE}");
                return 2;
            };
            open::run(stderr, secret_gate.as_deref())
        }
        Some("save") => save::run(rest, stderr),
        _ => {
            let _ = writeln!(stderr, "{USAGE}");
            2
        }
    }
}

fn write_help(stdout: &mut dyn Write, style: scan::Style) {
    for line in USAGE.lines() {
        let (command, comment) = line.split_once('#').unwrap_or((line, ""));
        let command = command
            .chars()
            .map(|ch| {
                if matches!(ch, '[' | ']' | '<' | '>' | '|' | '$') {
                    style.paint("2", ch.to_string())
                } else {
                    ch.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("");
        if comment.is_empty() {
            let _ = writeln!(stdout, "{command}");
        } else {
            let _ = writeln!(
                stdout,
                "{command}{}",
                style.paint("2", format!("#{comment}"))
            );
        }
    }
}

fn print_isotope_install_guidance(
    stderr: &mut dyn Write,
    style: scan::Style,
    target: &str,
    package: &str,
    install_command: &str,
) {
    let _ = writeln!(stderr, "╭─ harden {target}");
    let _ = writeln!(stderr, "│");
    let _ = writeln!(
        stderr,
        "◆ {}",
        style.paint("33", &format!("{package} is not installed"))
    );
    let _ = writeln!(stderr, "│");
    let _ = writeln!(stderr, "├─ 1. run: `{install_command}`");
    let _ = writeln!(stderr, "╰─ 2. run: `av harden {target}` again");
}

fn print_hardening_followup(stdout: &mut dyn Write, target: &str) {
    let _ = writeln!(stdout, "◇ verify with `av doctor {target}`");
}

fn finish_hardening<W: Write, E: Write>(
    result: Result<(), String>,
    target: &str,
    stdout: &mut W,
    stderr: &mut E,
) -> i32 {
    match result {
        Ok(()) => {
            print_hardening_followup(stdout, target);
            0
        }
        Err(err) => {
            let _ = writeln!(stderr, "av harden: {err}");
            1
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

fn parse_open_args(args: &[OsString]) -> Option<Option<String>> {
    match args {
        [] => Some(None),
        [flag, id] if flag == "--secret-gate" => {
            let id = id.to_str()?;
            open::valid_secret_gate_id(id).then(|| Some(id.to_string()))
        }
        _ => None,
    }
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

    fn run_scanner_args(args: &[&str]) -> (i32, String, String) {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = run_scanner_with_style(
            args.iter().map(OsString::from),
            &mut stdout,
            &mut stderr,
            scan::Style::plain(),
        );
        (
            code,
            String::from_utf8(stdout).unwrap(),
            String::from_utf8(stderr).unwrap(),
        )
    }

    #[test]
    fn scanner_only_accepts_scan_options() {
        let (code, stdout, stderr) = run_scanner_args(&["scanner", "--help"]);
        assert_eq!(
            (code, stdout.as_str(), stderr.as_str()),
            (0, "usage: scanner [--show-all|--json]\n", "")
        );

        let (code, stdout, stderr) = run_scanner_args(&["scanner", "doctor"]);
        assert_eq!(
            (code, stdout.as_str(), stderr.as_str()),
            (2, "", "usage: scanner [--show-all|--json]\n")
        );
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
            "╭─ harden gh\n│\n◆ gh-cli is not installed\n│\n├─ 1. run: `brew install automic-vault/isotopes/gh-cli`\n╰─ 2. run: `av harden gh` again\n"
        );
    }

    #[test]
    fn harden_stripe_tells_user_to_install_isotope() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let missing =
            std::env::temp_dir().join(format!("av-missing-stripe-{}", std::process::id()));
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_STRIPE_CLI_PATH", &missing);
        }

        let (code, stdout, stderr) = run_args(&["av", "harden", "stripe"]);

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_STRIPE_CLI_PATH");
        }
        assert_eq!(code, 1);
        assert_eq!(stdout, "");
        assert_eq!(
            stderr,
            "╭─ harden stripe\n│\n◆ stripe-cli is not installed\n│\n├─ 1. run: `brew install automic-vault/isotopes/stripe-cli`\n╰─ 2. run: `av harden stripe` again\n"
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
    fn harden_fly_aliases_flyctl() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let targets = std::env::temp_dir().join(format!("av-cli-fly-{}", std::process::id()));
        std::fs::create_dir_all(&targets).unwrap();
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_ENV_WRAPPER_TARGET_DIR", &targets);
        }

        let (code, stdout, stderr) = run_args(&["av", "harden", "fly"]);

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_ENV_WRAPPER_TARGET_DIR");
        }
        std::fs::remove_dir_all(&targets).unwrap();
        assert_eq!(code, 1);
        assert_eq!(stdout, "");
        assert_eq!(
            stderr,
            format!(
                "av harden: flyctl is not installed at {}\n",
                targets.join("flyctl").display()
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
            "╭─ harden sudo\n│\n◇ enables biometric authentication for sudo\n│\n╰─ run:\n\n        echo 'auth sufficient pam_tid.so' | sudo tee -a /etc/pam.d/sudo_local >/dev/null\n◇ verify with `av doctor sudo`\n"
        );
        assert_eq!(stderr, "");
    }

    #[test]
    fn private_env_wrapper_installer_rejects_unknown_targets() {
        let (code, stdout, stderr) =
            run_args(&["av", "__install-env-wrapper", "definitely-not-a-hardener"]);

        assert_eq!(code, 1);
        assert_eq!(stdout, "");
        assert_eq!(stderr, "av: unknown hardener `definitely-not-a-hardener`\n");
    }

    #[test]
    fn private_env_wrapper_installer_requires_root() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        unsafe { std::env::set_var("AUTOMIC_VAULT_TEST_EUID", "501") };

        let (code, stdout, stderr) = run_args(&["av", "__install-env-wrapper", "doctl"]);

        unsafe { std::env::remove_var("AUTOMIC_VAULT_TEST_EUID") };
        assert_eq!(code, 1);
        assert_eq!(stdout, "");
        assert_eq!(stderr, "av: env-wrapper installation requires root\n");
    }

    #[test]
    fn private_env_wrapper_installer_rejects_test_path_overrides() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_ENV_WRAPPER_STUB_DIR", "/tmp/stubs");
        }

        let (code, stdout, stderr) = run_args(&["av", "__install-env-wrapper", "doctl"]);

        unsafe { std::env::remove_var("AUTOMIC_VAULT_TEST_ENV_WRAPPER_STUB_DIR") };
        assert_eq!(code, 1);
        assert_eq!(stdout, "");
        assert_eq!(
            stderr,
            "av: test path overrides are forbidden during privileged installation\n"
        );
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
    fn help_describes_commands_and_required_arguments() {
        for help in ["help", "--help"] {
            let (code, stdout, stderr) = run_args(&["av", help]);

            assert_eq!(code, 0);
            assert_eq!(stdout, format!("{USAGE}\n"));
            assert_eq!(stderr, "");
            assert!(stdout.contains("\ncommands:\n"));
            assert!(stdout.contains("$ av harden <tool> [-y|--yes]"));
            assert!(stdout.contains("$ av list"));
            assert!(stdout.contains("$ av open [--secret-gate <id>]"));
            assert!(stdout.contains("\nmodes:\n"));
            assert!(stdout.contains("$ av help"));
            assert!(!stdout.contains("__version"));
        }

        assert!(
            USAGE
                .lines()
                .filter_map(|line| line.find('#'))
                .all(|column| column == 42)
        );

        let mut stdout = Vec::new();
        write_help(&mut stdout, scan::Style { color: true });
        let stdout = String::from_utf8(stdout).unwrap();
        assert!(stdout.contains("  \x1b[2m$\x1b[0m av scan"));
        assert!(stdout.contains("\x1b[2m[\x1b[0m--show-all\x1b[2m|\x1b[0m--json\x1b[2m]\x1b[0m"));
        assert!(stdout.contains("\x1b[2m<\x1b[0mtool\x1b[2m>\x1b[0m"));
        assert!(stdout.contains("\x1b[2m# audit secrets and configuration\x1b[0m"));

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(
            run_with_style(
                ["av", "help"].map(OsString::from),
                &mut stdout,
                &mut stderr,
                scan::Style::plain(),
                scan::Style { color: true },
            ),
            0
        );
        assert!(stdout.windows(4).any(|bytes| bytes == b"\x1b[2m"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn secret_gate_open_arguments_are_strictly_parsed() {
        assert_eq!(parse_open_args(&[]), Some(None));
        assert_eq!(
            parse_open_args(&[OsString::from("--secret-gate"), OsString::from("aws")]),
            Some(Some("aws".to_string()))
        );
        assert_eq!(
            parse_open_args(&[OsString::from("--secret-gate"), OsString::from("../aws")]),
            None
        );
        assert_eq!(parse_open_args(&[OsString::from("--secret-gate")]), None);
    }

    #[test]
    fn versions_are_reported_separately() {
        let (code, stdout, stderr) = run_args(&["av", "--version"]);
        assert_eq!(
            (code, stdout.as_str(), stderr.as_str()),
            (0, concat!("av ", env!("CARGO_PKG_VERSION"), "\n"), "")
        );

        let (code, stdout, stderr) = run_args(&["av", "__version"]);
        assert_eq!(
            (code, stdout, stderr),
            (0, format!("{INSTALL_REVISION}\n"), String::new())
        );
    }

    #[test]
    fn detectors_json_is_supported() {
        let (code, stdout, stderr) = run_args(&["av", "detectors", "--json"]);

        assert_eq!(code, 0);
        assert!(stdout.contains(r#""detectors":["#));
        assert_eq!(stderr, "");
    }
}
