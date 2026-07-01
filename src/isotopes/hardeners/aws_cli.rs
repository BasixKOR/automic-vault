use std::fs;
use std::io::{self, IsTerminal, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const STUB_DIR: &str = "/usr/local/bin";
const STUB_MARKER: &str = "# Automic Vault hardened stub";
const AWS_STUB: &str = include_str!("aws");
const AWS_STUB_PATH: &str = "/usr/local/bin/aws";
const AWS_VAULT_PATH: &str = "/opt/homebrew/bin/aws-vault";

unsafe extern "C" {
    fn geteuid() -> u32;
}

pub(crate) fn run_aws(stdout: &mut dyn Write, yes: bool) -> Result<(), String> {
    let aws_vault = aws_vault_path();
    if !aws_vault.exists() {
        return Err("aws-vault is not installed; run `brew install aws-vault`".to_string());
    }

    writeln!(stdout, "╭─ harden aws").ok();
    writeln!(stdout, "│").ok();
    writeln!(stdout, "◆ This will use aws-vault for AWS credentials.").ok();
    writeln!(stdout, "│").ok();
    writeln!(stdout, "├─ run `aws-vault add ${{AWS_PROFILE:-default}}`").ok();
    writeln!(
        stdout,
        "├─ remove plaintext keys from ~/.aws/credentials manually"
    )
    .ok();

    if unsafe { geteuid() } != 0 {
        writeln!(stdout, "╰─ finish with `sudo av harden aws`").ok();
        return Ok(());
    }

    writeln!(stdout, "├─ write {AWS_STUB_PATH}").ok();
    writeln!(stdout, "│").ok();
    if !confirm(stdout, yes)? {
        writeln!(stdout, "╰─ cancelled").ok();
        return Ok(());
    }
    install_aws_stub(&aws_stub_path())?;
    writeln!(stdout, "╰─ wrote {AWS_STUB_PATH}").ok();
    Ok(())
}

pub(crate) fn run_stub_install(
    target: &Path,
    stdout: &mut dyn Write,
    yes: bool,
) -> Result<(), String> {
    if unsafe { geteuid() } != 0 {
        return Err("must be run as root, use: sudo av harden PATH".to_string());
    }
    if !target.is_absolute() {
        return Err("target path must be absolute".to_string());
    }
    if !target.exists() {
        return Err(format!("{} does not exist", target.display()));
    }

    if !Path::new("/usr/local/bin/av").exists() {
        return Err("/usr/local/bin/av is not installed".to_string());
    }
    let tool = target
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("tool");
    let stub = stub_path(target)?;
    if stub.exists() && !is_av_stub(&stub) {
        return Err(format!(
            "{} already exists and is not an av hardened stub",
            stub.display()
        ));
    }

    writeln!(stdout, "╭─ install stub {tool}").ok();
    writeln!(stdout, "│").ok();
    writeln!(stdout, "◆ This will:").ok();
    writeln!(stdout, "│  1. verify /usr/local/bin/av").ok();
    writeln!(stdout, "│  2. write {}", stub.display()).ok();
    writeln!(stdout, "│  3. point it at {}", target.display()).ok();
    writeln!(stdout, "│").ok();
    if !confirm(stdout, yes)? {
        writeln!(stdout, "╰─ cancelled").ok();
        return Ok(());
    }
    writeln!(stdout, "│").ok();
    writeln!(stdout, "├─ ✓ verified /usr/local/bin/av").ok();

    fs::create_dir_all(STUB_DIR).map_err(|err| format!("failed to create {STUB_DIR}: {err}"))?;
    if stub.exists() {
        fs::remove_file(&stub)
            .map_err(|err| format!("failed to replace {}: {err}", stub.display()))?;
    }
    fs::write(&stub, stub_script(target)?)
        .map_err(|err| format!("failed to install stub at {}: {err}", stub.display()))?;
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755))
        .map_err(|err| format!("failed to chmod {}: {err}", stub.display()))?;

    writeln!(stdout, "├─ ✓ wrote {}", stub.display()).ok();
    writeln!(stdout, "╰─ done").ok();
    Ok(())
}

fn confirm(stdout: &mut dyn Write, yes: bool) -> Result<bool, String> {
    if yes {
        writeln!(stdout, "◇ Continue? yes (--yes)").ok();
        return Ok(true);
    }

    write!(stdout, "◇ Continue? [y/N] ").ok();
    stdout
        .flush()
        .map_err(|err| format!("failed to flush prompt: {err}"))?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|err| format!("failed to read confirmation: {err}"))?;
    if !io::stdin().is_terminal() {
        writeln!(stdout).ok();
    }
    Ok(matches!(
        input.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn stub_path(target: &Path) -> Result<PathBuf, String> {
    let name = target
        .file_name()
        .ok_or_else(|| "target path must end in a file name".to_string())?;
    Ok(Path::new(STUB_DIR).join(name))
}

fn is_av_stub(path: &Path) -> bool {
    fs::read_to_string(path)
        .map(|contents| contents.lines().nth(1) == Some(STUB_MARKER))
        .unwrap_or(false)
}

fn stub_script(target: &Path) -> Result<String, String> {
    let name = target
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "target path must end in a UTF-8 file name".to_string())?;
    Ok(format!(
        "#!/bin/sh\n{STUB_MARKER}\nexec /usr/local/bin/av stub-exec '{}' '{}' \"$@\"\n",
        shell_quote(name),
        shell_quote(&target.display().to_string())
    ))
}

fn shell_quote(value: &str) -> String {
    value.replace('\'', "'\\''")
}

fn aws_vault_path() -> PathBuf {
    std::env::var_os("AUTOMIC_VAULT_TEST_AWS_VAULT_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(AWS_VAULT_PATH))
}

fn aws_stub_path() -> PathBuf {
    std::env::var_os("AUTOMIC_VAULT_TEST_AWS_STUB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(AWS_STUB_PATH))
}

fn install_aws_stub(path: &Path) -> Result<(), String> {
    fs::write(path, AWS_STUB)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .map_err(|err| format!("failed to chmod {}: {err}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn aws_stub_uses_aws_vault_profile_env() {
        let path = temp_path("aws-stub");
        install_aws_stub(&path).unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), AWS_STUB);
        assert!(AWS_STUB.contains("${AWS_PROFILE:-default}"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn missing_aws_vault_tells_user_to_install_it() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let missing = temp_path("missing-aws-vault");
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_AWS_VAULT_PATH", &missing);
        }

        let err = run_aws(&mut Vec::new(), true).unwrap_err();

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_AWS_VAULT_PATH");
        }
        assert_eq!(
            err,
            "aws-vault is not installed; run `brew install aws-vault`"
        );
    }

    fn temp_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("av-{label}-{}-{nanos}", std::process::id()))
    }
}
