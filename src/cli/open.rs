use std::io::Write;
use std::process::Command;

pub(super) fn run<W: Write>(stderr: &mut W, secret_gate: Option<&str>) -> i32 {
    if let Some(secret_gate) = secret_gate {
        let launch_status = run_open(
            stderr,
            &[
                "-a",
                "Automic Vault",
                "--args",
                "--open-main-window",
                "--secret-gate",
                secret_gate,
            ],
        );
        if launch_status != 0 {
            return launch_status;
        }
        run_open(stderr, &[&secret_gate_url(secret_gate)])
    } else {
        run_open(
            stderr,
            &["-a", "Automic Vault", "--args", "--open-main-window"],
        )
    }
}

fn run_open<W: Write>(stderr: &mut W, arguments: &[&str]) -> i32 {
    match Command::new("/usr/bin/open").args(arguments).output() {
        Ok(output) if output.status.success() => 0,
        Ok(output) => {
            let _ = stderr.write_all(&output.stderr);
            output.status.code().unwrap_or(1)
        }
        Err(err) => {
            let _ = writeln!(stderr, "av open: {err}");
            1
        }
    }
}

pub(super) fn valid_secret_gate_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn secret_gate_url(id: &str) -> String {
    debug_assert!(valid_secret_gate_id(id));
    format!("automic-vault://secret-gate/{id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_gate_deep_links_accept_only_path_safe_ids() {
        assert_eq!(
            secret_gate_url("aws-cli"),
            "automic-vault://secret-gate/aws-cli"
        );
        assert!(valid_secret_gate_id("aws_cli.v2"));
        assert!(!valid_secret_gate_id(""));
        assert!(!valid_secret_gate_id("../aws"));
        assert!(!valid_secret_gate_id("aws/token"));
        assert!(!valid_secret_gate_id("aws token"));
    }
}
