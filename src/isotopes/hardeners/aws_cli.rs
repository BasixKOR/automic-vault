use std::fs;
use std::io::{self, IsTerminal, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const STUB_DIR: &str = "/usr/local/bin";
const STUB_MARKER: &str = "# Automic Vault hardened stub";

unsafe extern "C" {
    fn geteuid() -> u32;
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
