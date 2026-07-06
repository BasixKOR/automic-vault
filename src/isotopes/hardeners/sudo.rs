use std::io::Write;

const ENABLE_TOUCH_ID_COMMAND: &str = "grep -Eq '^[[:space:]]*auth[[:space:]].*pam_tid\\.so' /etc/pam.d/sudo_local 2>/dev/null || echo 'auth sufficient pam_tid.so' | sudo tee -a /etc/pam.d/sudo_local >/dev/null";

pub(crate) fn run(stdout: &mut dyn Write) -> Result<(), String> {
    writeln!(stdout, "╭─ harden sudo").ok();
    writeln!(stdout, "│").ok();
    writeln!(
        stdout,
        "╰─ run `{ENABLE_TOUCH_ID_COMMAND}` to enable Touch ID for sudo"
    )
    .ok();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_touch_id_command() {
        let mut stdout = Vec::new();

        run(&mut stdout).unwrap();

        let stdout = String::from_utf8(stdout).unwrap();
        assert!(stdout.contains("pam_tid\\.so"));
        assert!(stdout.contains("/etc/pam.d/sudo_local"));
    }
}
