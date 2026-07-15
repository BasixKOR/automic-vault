use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{HardenerDetection, SecretGateDescriptor, SecretGateRoute};

const SUPABASE_CLI_PATH: &str = "/opt/homebrew/opt/supabase-cli/bin/supabase";
const INSTALL_COMMAND: &str = "brew install automic-vault/isotopes/supabase-cli";
const VAULT_KEY: &str = "SUPABASE_ACCESS_TOKEN";
const KEYCHAIN_SERVICE: &str = "Supabase CLI";
const KEYCHAIN_ACCOUNTS: &[&str] = &["supabase", "access-token"];

pub(crate) fn run(stdout: &mut dyn Write, yes: bool) -> Result<(), String> {
    if !supabase_cli_path().exists() {
        return Err(format!(
            "supabase is not installed; run `{INSTALL_COMMAND}`"
        ));
    }

    let token_paths = supabase_token_paths()?;
    let mut tokens = read_plaintext_tokens(&token_paths)?;
    if tokens.is_empty() {
        tokens.extend(read_legacy_keychain_tokens());
    }
    tokens.sort();
    tokens.dedup();

    writeln!(stdout, "╭─ harden supabase").ok();
    writeln!(stdout, "│").ok();
    if tokens.is_empty() {
        writeln!(stdout, "╰─ no legacy Supabase credentials found").ok();
        return Ok(());
    }
    if tokens.len() > 1 {
        return Err(
            "multiple distinct Supabase access tokens found; remove the stale one and retry".into(),
        );
    }

    writeln!(
        stdout,
        "├─ migrate Supabase access token into Automic Vault"
    )
    .ok();
    writeln!(stdout, "├─ remove plaintext fallback access-token files").ok();
    writeln!(stdout, "│").ok();
    if !confirm(stdout, yes)? {
        writeln!(stdout, "╰─ cancelled").ok();
        return Ok(());
    }

    crate::secrets::store_secret(VAULT_KEY, &tokens[0])?;
    for path in &token_paths {
        remove_plaintext_token(path)?;
    }
    for account in KEYCHAIN_ACCOUNTS {
        delete_legacy_keychain_token(account)?;
    }
    writeln!(stdout, "╰─ migrated Supabase credentials").ok();
    Ok(())
}

pub(crate) fn detect() -> HardenerDetection {
    let path = supabase_cli_path();
    let exists = path.exists();
    let path = path.display().to_string();
    HardenerDetection::command(exists, "supabase", Some(path.clone()), path)
}

pub(crate) fn secret_gate() -> SecretGateDescriptor {
    SecretGateDescriptor {
        id: "supabase",
        key_patterns: vec![VAULT_KEY.to_string()],
        routes: vec![SecretGateRoute {
            operation: "keys",
            script_path: None,
            target_path: supabase_cli_path().display().to_string(),
            caller_identifiers: vec!["supabase", "supabase-go", "com.supabase.cli"],
            key_patterns: vec![VAULT_KEY.to_string()],
            replace_existing_env: true,
            allow_missing_keys: false,
        }],
    }
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

fn supabase_cli_path() -> PathBuf {
    crate::test_env_var("AUTOMIC_VAULT_TEST_SUPABASE_CLI_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(SUPABASE_CLI_PATH).to_path_buf())
}

fn supabase_token_paths() -> Result<Vec<PathBuf>, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    let mut paths = vec![home.join(".supabase/access-token")];
    if let Some(supabase_home) = std::env::var_os("SUPABASE_HOME").filter(|value| !value.is_empty())
    {
        paths.push(PathBuf::from(supabase_home).join("access-token"));
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn read_plaintext_tokens(paths: &[PathBuf]) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    for path in paths {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
            Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
        };
        let token = contents.trim();
        if is_supabase_access_token(token) {
            tokens.push(token.to_string());
        }
    }
    Ok(tokens)
}

fn read_legacy_keychain_tokens() -> Vec<String> {
    if let Some(token) = crate::test_env_string("AUTOMIC_VAULT_TEST_SUPABASE_LEGACY_TOKEN") {
        return vec![token];
    }
    KEYCHAIN_ACCOUNTS
        .iter()
        .filter_map(|account| security_find_generic_password(KEYCHAIN_SERVICE, account))
        .collect()
}

fn security_find_generic_password(service: &str, account: &str) -> Option<String> {
    let output = Command::new("/usr/bin/security")
        .args(["find-generic-password", "-s", service, "-a", account, "-w"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    is_supabase_access_token(&token).then_some(token)
}

fn remove_plaintext_token(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("failed to remove {}: {err}", path.display())),
    }
}

fn delete_legacy_keychain_token(account: &str) -> Result<(), String> {
    if crate::test_env_var("AUTOMIC_VAULT_TEST_SUPABASE_LEGACY_TOKEN").is_some() {
        return Ok(());
    }
    let output = Command::new("/usr/bin/security")
        .args([
            "delete-generic-password",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            account,
        ])
        .output()
        .map_err(|err| format!("failed to run security: {err}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("could not be found")
        || stderr.contains("The specified item could not be found")
    {
        return Ok(());
    }
    Err(format!(
        "failed to delete legacy Supabase keychain item: {}",
        stderr.trim()
    ))
}

fn is_supabase_access_token(value: &str) -> bool {
    let suffix = value
        .strip_prefix("sbp_oauth_")
        .or_else(|| value.strip_prefix("sbp_"));
    suffix.is_some_and(|rest| rest.len() == 40 && rest.chars().all(|ch| ch.is_ascii_hexdigit()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn harden_imports_plaintext_access_token() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let dir = temp_path("supabase-import");
        let home = dir.join("home");
        let keychain = dir.join("keychain");
        let supabase = dir.join("supabase");
        let token_dir = home.join(".supabase");
        fs::create_dir_all(&token_dir).unwrap();
        fs::write(&supabase, "").unwrap();
        fs::write(
            token_dir.join("access-token"),
            "sbp_0123456789abcdef0123456789abcdef01234567\n",
        )
        .unwrap();
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR", &keychain);
            std::env::set_var("AUTOMIC_VAULT_TEST_SUPABASE_CLI_PATH", &supabase);
        }

        run(&mut Vec::new(), true).unwrap();

        unsafe {
            std::env::remove_var("HOME");
            std::env::remove_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR");
            std::env::remove_var("AUTOMIC_VAULT_TEST_SUPABASE_CLI_PATH");
        }
        assert_eq!(
            fs::read_to_string(keychain.join(VAULT_KEY)).unwrap(),
            "sbp_0123456789abcdef0123456789abcdef01234567"
        );
        assert!(!token_dir.join("access-token").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn detect_reports_full_supabase_path() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let missing = temp_path("missing-supabase-cli-detect");
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_SUPABASE_CLI_PATH", &missing);
        }

        let detection = detect();

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_SUPABASE_CLI_PATH");
        }
        assert_eq!(detection.target_path, Some(missing.display().to_string()));
    }

    #[test]
    fn validates_supabase_access_tokens() {
        assert!(is_supabase_access_token(
            "sbp_0123456789abcdef0123456789abcdef01234567"
        ));
        assert!(is_supabase_access_token(
            "sbp_oauth_0123456789abcdef0123456789abcdef01234567"
        ));
        assert!(!is_supabase_access_token("sbp_short"));
        assert!(!is_supabase_access_token("not-a-token"));
    }

    fn temp_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("av-{label}-{}-{nanos}", std::process::id()))
    }
}
