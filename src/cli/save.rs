use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::os::fd::AsRawFd;

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
    let [key] = args.as_slice() else {
        return Err("usage: av save KEY".into());
    };
    let key = key
        .to_str()
        .ok_or_else(|| "save key must be valid UTF-8".to_string())?;
    inject::validate_key_name(key)?;
    let value = read_secret_from_tty(key)?;
    save_value(key, &value)
}

fn save_value(key: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("empty key value".into());
    }
    crate::secrets::store_secret(key, value)
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

        save_value("SAVED_KEY", "secret").unwrap();

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
        assert_eq!(save_value("SAVED_KEY", "").unwrap_err(), "empty key value");
    }
}
