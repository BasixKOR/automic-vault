#![allow(dead_code)]

use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICES: &[&str] = &["StripeCLI"];

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in candidate_config_paths()? {
        if !path.exists() {
            continue;
        }
        let contents = read_to_string(&path)?;
        if stripe_config_contains_plaintext_key(&contents) {
            reasons.push(format!(
                "Stripe CLI config contains plaintext API keys: {}",
                path.display()
            ));
        }
    }
    if super::gh_cli::keychain_services_allow_security_tool(
        &KEYCHAIN_SERVICES
            .iter()
            .map(|service| (*service).to_string())
            .collect::<Vec<_>>(),
    )? {
        reasons.push(
            "Stripe CLI keychain item allows non-interactive extraction by the security tool"
                .to_string(),
        );
    }
    Ok(reasons)
}

fn candidate_config_paths() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    let mut paths = vec![home.join(".config/stripe/config.toml")];
    if let Some(xdg_config_home) =
        std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty())
    {
        paths.push(PathBuf::from(xdg_config_home).join("stripe/config.toml"));
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn stripe_config_contains_plaintext_key(contents: &str) -> bool {
    contents.lines().any(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return false;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            return false;
        };
        let key = key.trim();
        let value = value.trim().trim_matches('"').trim_matches('\'');
        matches!(
            key,
            "test_mode_api_key" | "live_mode_api_key" | "api_key" | "secret_key"
        ) && stripe_key_is_plaintext(value)
    })
}

fn stripe_key_is_plaintext(value: &str) -> bool {
    let value = value.trim();
    value.len() >= 12
        && !value.contains('*')
        && (value.starts_with("sk_") || value.starts_with("rk_"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_plaintext_but_not_redacted_keys() {
        assert!(stripe_config_contains_plaintext_key(
            "[default]\ntest_mode_api_key = \"sk_test_123456789\"\n"
        ));
        assert!(stripe_config_contains_plaintext_key(
            "[default]\napi_key = \"rk_test_123456789\"\n"
        ));
        assert!(!stripe_config_contains_plaintext_key(
            "[default]\nlive_mode_api_key = \"sk_live_*********0000\"\n"
        ));
    }

    #[test]
    fn checks_the_upstream_stripe_cli_keychain_service() {
        assert_eq!(KEYCHAIN_SERVICES, ["StripeCLI"]);
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("stripe-cli", install_insecurity_reasons, home)
}
