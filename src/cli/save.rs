use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use super::inject;

pub(crate) fn run(args: Vec<OsString>, stderr: &mut dyn Write) -> i32 {
    match run_inner(args) {
        Ok(()) => 0,
        Err(err) => {
            let _ = writeln!(stderr, "av save: {err}");
            1
        }
    }
}

fn run_inner(args: Vec<OsString>) -> Result<(), String> {
    let (key, project_directory) = parse_args(args)?;
    let key = key
        .to_str()
        .ok_or_else(|| "save key must be valid UTF-8".to_string())?;
    inject::validate_key_name(key)?;
    let value = read_secret_from_tty(key)?;
    save_value(key, &value, project_directory.as_deref())
}

fn parse_args(args: Vec<OsString>) -> Result<(OsString, Option<String>), String> {
    let mut key = None;
    let mut project_directory = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--project-directory" {
            let path = args.next().ok_or_else(usage)?;
            if project_directory.is_some() {
                return Err("--project-directory may be specified only once".into());
            }
            project_directory = Some(canonical_project_directory(Path::new(&path))?);
        } else if let Some(path) = arg
            .to_str()
            .and_then(|arg| arg.strip_prefix("--project-directory="))
        {
            if path.is_empty() || project_directory.is_some() {
                return Err(usage());
            }
            project_directory = Some(canonical_project_directory(Path::new(path))?);
        } else if key.replace(arg).is_some() {
            return Err(usage());
        }
    }
    Ok((key.ok_or_else(usage)?, project_directory))
}

fn usage() -> String {
    "usage: av save [--project-directory=DIR] KEY".into()
}

fn canonical_project_directory(path: &Path) -> Result<String, String> {
    let path = std::fs::canonicalize(path).map_err(|err| {
        format!(
            "failed to resolve project directory {}: {err}",
            path.display()
        )
    })?;
    let metadata = path.metadata().map_err(|err| {
        format!(
            "failed to inspect project directory {}: {err}",
            path.display()
        )
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "project directory is not a directory: {}",
            path.display()
        ));
    }
    let parent = path
        .parent()
        .ok_or("project directory cannot be a filesystem root")?;
    let parent_metadata = parent.metadata().map_err(|err| {
        format!(
            "failed to inspect project directory parent {}: {err}",
            parent.display()
        )
    })?;
    if parent == path || parent_metadata.dev() != metadata.dev() {
        return Err("project directory cannot be a filesystem root".into());
    }
    path.into_os_string()
        .into_string()
        .map_err(|_| "project directory must be valid UTF-8".into())
}

fn save_value(key: &str, value: &str, project_directory: Option<&str>) -> Result<(), String> {
    if value.is_empty() {
        return Err("empty key value".into());
    }
    match project_directory {
        Some(path) => crate::secrets::store_project_secret(key, value, path),
        None => crate::secrets::store_secret(key, value),
    }
}

fn read_secret_from_tty(key: &str) -> Result<String, String> {
    let mut tty = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .map_err(|err| format!("failed to open /dev/tty: {err}"))?;
    write!(tty, "Value for {key}: ").map_err(|err| format!("failed to prompt: {err}"))?;
    tty.flush()
        .map_err(|err| format!("failed to flush prompt: {err}"))?;

    let fd = tty.as_raw_fd();
    let mut original = unsafe { std::mem::zeroed::<libc::termios>() };
    if unsafe { libc::tcgetattr(fd, &mut original) } != 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let mut hidden = original;
    hidden.c_lflag &= !libc::ECHO;
    if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &hidden) } != 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let restore = EchoRestore { fd, original };

    let mut reader = BufReader::new(
        tty.try_clone()
            .map_err(|err| format!("failed to read from /dev/tty: {err}"))?,
    );
    let mut value = String::new();
    reader
        .read_line(&mut value)
        .map_err(|err| format!("failed to read key value: {err}"))?;
    drop(restore);
    writeln!(tty).ok();

    while value.ends_with('\n') || value.ends_with('\r') {
        value.pop();
    }
    Ok(value)
}

struct EchoRestore {
    fd: i32,
    original: libc::termios,
}

impl Drop for EchoRestore {
    fn drop(&mut self) {
        unsafe {
            libc::tcsetattr(self.fd, libc::TCSANOW, &self.original);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_value_to_test_keychain() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let dir = std::env::temp_dir().join(format!("av-save-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR", &dir);
        }

        save_value("SAVED_KEY", "secret", None).unwrap();

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR");
        }
        assert_eq!(
            std::fs::read_to_string(dir.join("SAVED_KEY")).unwrap(),
            "secret"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_empty_value() {
        assert_eq!(
            save_value("SAVED_KEY", "", None).unwrap_err(),
            "empty key value"
        );
    }

    #[test]
    fn parses_and_canonicalizes_project_directory() {
        let directory = std::env::temp_dir();
        let (key, project) = parse_args(vec![
            OsString::from(format!("--project-directory={}", directory.display())),
            OsString::from("SAVED_KEY"),
        ])
        .unwrap();
        assert_eq!(key, "SAVED_KEY");
        assert_eq!(
            project,
            Some(
                std::fs::canonicalize(directory)
                    .unwrap()
                    .display()
                    .to_string()
            )
        );
    }

    #[test]
    fn saves_project_value_separately_from_global_value() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let root = std::env::temp_dir().join(format!("av-project-save-{}", std::process::id()));
        let keychain = root.join("keychain");
        let project = root.join("project");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&project).unwrap();
        let project = std::fs::canonicalize(project).unwrap();
        unsafe { std::env::set_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR", &keychain) };

        save_value("SAVED_KEY", "global", None).unwrap();
        save_value("SAVED_KEY", "project", project.to_str()).unwrap();

        unsafe { std::env::remove_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR") };
        assert_eq!(
            std::fs::read_to_string(keychain.join("SAVED_KEY")).unwrap(),
            "global"
        );
        assert_eq!(
            std::fs::read_to_string(crate::secrets::test_project_secret_path(
                &keychain,
                project.to_str().unwrap(),
                "SAVED_KEY"
            ))
            .unwrap(),
            "project"
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
