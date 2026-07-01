use std::io::Write;
use std::path::{Path, PathBuf};

const GH_CLI_PATH: &str = "/opt/homebrew/opt/gh-cli/bin/gh";
const INSTALL_COMMAND: &str = "brew install automic-vault/isotopes/gh-cli";
const MIGRATE_COMMAND: &str = "gh auth av-migrate";

pub(crate) fn run(stdout: &mut dyn Write) -> Result<(), String> {
    if !gh_cli_path().exists() {
        return Err(format!("gh-cli is not installed; run `{INSTALL_COMMAND}`"));
    }

    writeln!(stdout, "╭─ harden gh-cli").ok();
    writeln!(stdout, "│").ok();
    writeln!(
        stdout,
        "╰─ run `{MIGRATE_COMMAND}` to migrate GitHub credentials"
    )
    .ok();
    Ok(())
}

fn gh_cli_path() -> PathBuf {
    std::env::var_os("AUTOMIC_VAULT_TEST_GH_CLI_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(GH_CLI_PATH).to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn missing_gh_cli_tells_user_to_install_isotope() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let missing = temp_path("missing-gh-cli");
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_GH_CLI_PATH", &missing);
        }

        let err = run(&mut Vec::new()).unwrap_err();

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_GH_CLI_PATH");
        }
        assert_eq!(
            err,
            "gh-cli is not installed; run `brew install automic-vault/isotopes/gh-cli`"
        );
    }

    #[test]
    fn installed_gh_cli_prints_migration_command() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let gh = temp_path("gh");
        fs::write(&gh, "").unwrap();
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_GH_CLI_PATH", &gh);
        }
        let mut stdout = Vec::new();

        run(&mut stdout).unwrap();

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_GH_CLI_PATH");
        }
        let stdout = String::from_utf8(stdout).unwrap();
        assert!(stdout.contains("`gh auth av-migrate`"));
        let _ = fs::remove_file(gh);
    }

    fn temp_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("av-{label}-{}-{nanos}", std::process::id()))
    }
}
