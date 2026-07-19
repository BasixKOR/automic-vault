use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{HardenerDetection, SecretGateDescriptor, SecretGateRoute};

const GH_CLI_PATH: &str = "/opt/homebrew/opt/gh-cli/bin/gh";
pub(crate) const INSTALL_COMMAND: &str = "brew install automic-vault/isotopes/gh-cli";

#[derive(Debug)]
pub(crate) enum HardenError {
    GhCliNotInstalled,
    Other(String),
}

impl From<String> for HardenError {
    fn from(error: String) -> Self {
        Self::Other(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GhCredential {
    host: String,
    user: Option<String>,
    token: String,
}

pub(crate) fn run(stdout: &mut dyn Write, yes: bool) -> Result<(), HardenError> {
    if !gh_cli_path().exists() {
        return Err(HardenError::GhCliNotInstalled);
    }

    let hosts_paths = gh_hosts_paths()?;
    let mut credentials = Vec::new();
    for path in &hosts_paths {
        credentials.extend(read_plaintext_credentials(path)?);
    }
    if credentials.is_empty() {
        credentials.extend(read_legacy_keychain_credentials(&hosts_paths));
    }

    writeln!(stdout, "╭─ harden gh").ok();
    writeln!(stdout, "│").ok();
    if credentials.is_empty() {
        writeln!(stdout, "╰─ no legacy gh credentials found").ok();
        super::write_secret_gate_notice(stdout, "gh");
        return Ok(());
    }

    writeln!(
        stdout,
        "├─ migrate {} GitHub token(s) into Automic Vault",
        credentials.len()
    )
    .ok();
    writeln!(
        stdout,
        "├─ remove plaintext oauth_token entries from hosts.yml"
    )
    .ok();
    writeln!(stdout, "│").ok();
    if !confirm(stdout, yes)? {
        writeln!(stdout, "╰─ cancelled").ok();
        return Ok(());
    }

    for credential in &credentials {
        store_gh_credential(credential)?;
    }
    for path in &hosts_paths {
        remove_plaintext_tokens(path)?;
    }
    for credential in &credentials {
        delete_legacy_keychain_tokens(&credential.host, credential.user.as_deref())?;
    }
    writeln!(stdout, "╰─ migrated gh credentials").ok();
    super::write_secret_gate_notice(stdout, "gh");
    Ok(())
}

pub(crate) fn detect() -> HardenerDetection {
    let path = gh_cli_path();
    let exists = path.exists();
    let path = path.display().to_string();
    HardenerDetection::command(exists, "gh", Some(path.clone()), path)
}

pub(crate) fn secret_gate() -> SecretGateDescriptor {
    SecretGateDescriptor {
        id: "gh",
        key_patterns: vec!["GH_TOKEN_*".to_string()],
        routes: vec![SecretGateRoute {
            operation: "keys",
            script_path: None,
            target_path: gh_cli_path().display().to_string(),
            caller_identifiers: vec!["gh", "com.github.cli"],
            key_patterns: vec!["GH_TOKEN_*".to_string()],
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

fn gh_cli_path() -> PathBuf {
    crate::test_env_var("AUTOMIC_VAULT_TEST_GH_CLI_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(GH_CLI_PATH).to_path_buf())
}

fn gh_hosts_paths() -> Result<Vec<PathBuf>, String> {
    if let Some(config_dir) = std::env::var_os("GH_CONFIG_DIR").filter(|value| !value.is_empty()) {
        return Ok(vec![PathBuf::from(config_dir).join("hosts.yml")]);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    let mut paths = Vec::new();
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty())
    {
        paths.push(PathBuf::from(config_home).join("gh/hosts.yml"));
    }
    paths.push(home.join(".config/gh/hosts.yml"));
    Ok(paths)
}

fn read_plaintext_credentials(path: &Path) -> Result<Vec<GhCredential>, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    Ok(parse_hosts_credentials(&contents))
}

fn parse_hosts_credentials(contents: &str) -> Vec<GhCredential> {
    let mut credentials = Vec::new();
    let mut host = None::<String>;
    let mut active_user = None::<String>;
    let mut user_context = None::<String>;

    for line in contents.lines() {
        let indent = line.chars().take_while(|ch| ch.is_whitespace()).count();
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if indent == 0 {
            host = trimmed.strip_suffix(':').map(str::to_string);
            active_user = None;
            user_context = None;
            continue;
        }
        if indent <= 4 {
            user_context = None;
        }
        if indent == 4 {
            if let Some(value) = yaml_string_value(trimmed, "user") {
                active_user = Some(value.to_string());
                continue;
            }
        }
        if indent >= 8 && trimmed.ends_with(':') {
            user_context = trimmed.strip_suffix(':').map(str::to_string);
            continue;
        }
        if let Some(token) = yaml_string_value(trimmed, "oauth_token") {
            if token.is_empty() || token == "null" {
                continue;
            }
            let Some(host) = &host else { continue };
            credentials.push(GhCredential {
                host: host.clone(),
                user: user_context.clone().or_else(|| active_user.clone()),
                token: token.to_string(),
            });
        }
    }
    credentials
}

fn yaml_string_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let (got_key, value) = line.split_once(':')?;
    if got_key.trim() != key {
        return None;
    }
    Some(value.trim().trim_matches('"').trim_matches('\''))
}

fn read_legacy_keychain_credentials(hosts_paths: &[PathBuf]) -> Vec<GhCredential> {
    let hosts = configured_hosts(hosts_paths);
    let mut credentials = Vec::new();
    for (host, user) in hosts {
        if let Some(token) = legacy_token(&host, user.as_deref()) {
            credentials.push(GhCredential { host, user, token });
        }
    }
    credentials
}

fn configured_hosts(hosts_paths: &[PathBuf]) -> Vec<(String, Option<String>)> {
    let mut hosts = vec![("github.com".to_string(), None)];
    for path in hosts_paths {
        let Ok(contents) = fs::read_to_string(path) else {
            continue;
        };
        let mut host = None::<String>;
        let mut active_user = None::<String>;
        for line in contents.lines() {
            let indent = line.chars().take_while(|ch| ch.is_whitespace()).count();
            let trimmed = line.trim();
            if indent == 0 {
                if let Some(previous) = host.take() {
                    hosts.push((previous, active_user.take()));
                }
                host = trimmed.strip_suffix(':').map(str::to_string);
            } else if indent == 4 {
                if let Some(value) = yaml_string_value(trimmed, "user") {
                    active_user = Some(value.to_string());
                }
            }
        }
        if let Some(previous) = host {
            hosts.push((previous, active_user));
        }
    }
    hosts.sort();
    hosts.dedup();
    hosts
}

fn legacy_token(host: &str, user: Option<&str>) -> Option<String> {
    if let Some(token) = crate::test_env_string("AUTOMIC_VAULT_TEST_GH_LEGACY_TOKEN") {
        return Some(token);
    }
    let service = format!("gh:{host}");
    if let Some(user) = user {
        if let Some(token) = security_find_generic_password(&service, Some(user)) {
            return Some(token);
        }
    }
    security_find_generic_password(&service, Some(""))
        .or_else(|| security_find_generic_password(&service, None))
}

fn security_find_generic_password(service: &str, account: Option<&str>) -> Option<String> {
    let mut command = Command::new("/usr/bin/security");
    command.args(["find-generic-password", "-s", service]);
    if let Some(account) = account {
        command.args(["-a", account]);
    }
    command.arg("-w");
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!token.is_empty()).then_some(token)
}

fn store_gh_credential(credential: &GhCredential) -> Result<(), String> {
    crate::secrets::store_secret(&vault_key(&credential.host, None), &credential.token)?;
    if let Some(user) = &credential.user {
        crate::secrets::store_secret(&vault_key(&credential.host, Some(user)), &credential.token)?;
    }
    Ok(())
}

fn remove_plaintext_tokens(path: &Path) -> Result<(), String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    let cleaned = contents
        .split_inclusive('\n')
        .filter(|line| yaml_string_value(line.trim(), "oauth_token").is_none())
        .collect::<String>();
    if cleaned != contents {
        fs::write(path, cleaned)
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    }
    Ok(())
}

fn delete_legacy_keychain_tokens(host: &str, user: Option<&str>) -> Result<(), String> {
    if crate::test_env_var("AUTOMIC_VAULT_TEST_GH_LEGACY_TOKEN").is_some() {
        return Ok(());
    }
    let service = format!("gh:{host}");
    if let Some(user) = user {
        security_delete_generic_password(&service, Some(user))?;
    }
    security_delete_generic_password(&service, Some(""))
}

fn security_delete_generic_password(service: &str, account: Option<&str>) -> Result<(), String> {
    let mut command = Command::new("/usr/bin/security");
    command.args(["delete-generic-password", "-s", service]);
    if let Some(account) = account {
        command.args(["-a", account]);
    }
    let output = command
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
        "failed to delete legacy gh keychain item: {}",
        stderr.trim()
    ))
}

fn vault_key(host: &str, user: Option<&str>) -> String {
    let mut key = format!("GH_TOKEN_{}", sanitize_key_part(host));
    if let Some(user) = user {
        key.push('_');
        key.push_str(&sanitize_key_part(user));
    }
    key
}

fn sanitize_key_part(value: &str) -> String {
    let mut out = String::new();
    let mut last_underscore = false;
    for ch in value.chars().flat_map(char::to_uppercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_underscore = false;
        } else if !last_underscore {
            out.push('_');
            last_underscore = true;
        }
    }
    out.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn missing_gh_cli_tells_user_to_install_isotope() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let missing = temp_path("missing-gh-cli");
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_GH_CLI_PATH", &missing);
        }

        let err = run(&mut Vec::new(), true).err().unwrap();

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_GH_CLI_PATH");
        }
        assert!(matches!(err, HardenError::GhCliNotInstalled));
    }

    #[test]
    fn detect_reports_full_gh_path() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let missing = temp_path("missing-gh-cli-detect");
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_GH_CLI_PATH", &missing);
        }

        let detection = detect();

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_GH_CLI_PATH");
        }
        assert_eq!(detection.target_path, Some(missing.display().to_string()));
    }

    #[test]
    fn parses_plaintext_hosts_tokens() {
        assert_eq!(
            parse_hosts_credentials(
                "github.com:\n    user: monalisa\n    oauth_token: ghp_secret\n    users:\n        hubot:\n            oauth_token: gho_bot\n"
            ),
            vec![
                GhCredential {
                    host: "github.com".into(),
                    user: Some("monalisa".into()),
                    token: "ghp_secret".into(),
                },
                GhCredential {
                    host: "github.com".into(),
                    user: Some("hubot".into()),
                    token: "gho_bot".into(),
                }
            ]
        );
    }

    #[test]
    fn harden_imports_plaintext_hosts_tokens() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let dir = temp_path("gh-import");
        let config = dir.join("config");
        let keychain = dir.join("keychain");
        let gh = dir.join("gh");
        fs::create_dir_all(&config).unwrap();
        fs::write(&gh, "").unwrap();
        fs::write(
            config.join("hosts.yml"),
            "github.com:\n    user: monalisa\n    oauth_token: ghp_secret\n",
        )
        .unwrap();
        unsafe {
            std::env::set_var("GH_CONFIG_DIR", &config);
            std::env::set_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR", &keychain);
            std::env::set_var("AUTOMIC_VAULT_TEST_GH_CLI_PATH", &gh);
        }

        run(&mut Vec::new(), true).unwrap();

        unsafe {
            std::env::remove_var("GH_CONFIG_DIR");
            std::env::remove_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR");
            std::env::remove_var("AUTOMIC_VAULT_TEST_GH_CLI_PATH");
        }
        assert_eq!(
            fs::read_to_string(keychain.join("GH_TOKEN_GITHUB_COM")).unwrap(),
            "ghp_secret"
        );
        assert_eq!(
            fs::read_to_string(keychain.join("GH_TOKEN_GITHUB_COM_MONALISA")).unwrap(),
            "ghp_secret"
        );
        assert!(
            !fs::read_to_string(config.join("hosts.yml"))
                .unwrap()
                .contains("oauth_token")
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn vault_key_matches_gh_isotope() {
        assert_eq!(vault_key("github.com", None), "GH_TOKEN_GITHUB_COM");
        assert_eq!(
            vault_key("github.com", Some("mona-lisa")),
            "GH_TOKEN_GITHUB_COM_MONA_LISA"
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
