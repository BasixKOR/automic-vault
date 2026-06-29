#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in cpan_config_paths()? {
        if path.exists() && cpan_config_contains_secret(&read_to_string(&path)?) {
            reasons.push(format!(
                "CPAN config contains plaintext credentials: {}",
                path.display()
            ));
        }
    }
    Ok(reasons)
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn cpan_config_paths() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    Ok(vec![
        home.join(".cpan/CPAN/MyConfig.pm"),
        home.join(".cpan/CPAN/Config.pm"),
        home.join(".cpan/CPAN/Config_local.pm"),
    ])
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn cpan_config_contains_secret(contents: &str) -> bool {
    contents.lines().any(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return false;
        }
        let lower = trimmed.to_ascii_lowercase();
        if ["proxy_pass", "proxy_user", "password", "passwd"]
            .iter()
            .any(|key| lower.contains(key))
            && quoted_or_arrow_value(trimmed).is_some_and(secret_value_is_real)
        {
            return true;
        }
        lower.contains("://") && url_contains_userinfo_secret(trimmed)
    })
}

fn quoted_or_arrow_value(line: &str) -> Option<&str> {
    let value = line
        .split_once("=>")
        .map(|(_, value)| value)
        .or_else(|| line.split_once('=').map(|(_, value)| value))?
        .trim()
        .trim_end_matches(',')
        .trim_end_matches(';')
        .trim();
    Some(value.trim_matches('"').trim_matches('\'').trim())
}

fn secret_value_is_real(value: &str) -> bool {
    value.len() >= 6
        && !value.contains("${")
        && !value.eq_ignore_ascii_case("secret")
        && !value.eq_ignore_ascii_case("password")
        && !value.eq_ignore_ascii_case("redacted")
}

fn url_contains_userinfo_secret(value: &str) -> bool {
    let Some(scheme_end) = value.find("://") else {
        return false;
    };
    let rest = &value[scheme_end + 3..];
    let Some(userinfo_end) = rest.find('@') else {
        return false;
    };
    let host_end = rest.find('/').unwrap_or(rest.len());
    userinfo_end < host_end
        && rest[..userinfo_end]
            .split_once(':')
            .is_some_and(|(_, password)| secret_value_is_real(password))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_proxy_passwords_and_userinfo_urls() {
        assert!(cpan_config_contains_secret(
            "'proxy_pass' => 'supersecret',\n"
        ));
        assert!(cpan_config_contains_secret(
            "urllist => [q[https://user:supersecret@example.com/]]\n"
        ));
        assert!(!cpan_config_contains_secret(
            "'proxy_pass' => 'redacted',\n"
        ));
    }

    #[test]
    fn top_level_detection_reports_config_file() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!("perl-detect-{}", std::process::id()));
        let config = home.join(".cpan/CPAN/MyConfig.pm");
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(config.parent().unwrap()).unwrap();
        std::fs::write(&config, "'proxy_pass' => 'supersecret',\n").unwrap();
        let previous_home = std::env::var_os("HOME");
        unsafe { std::env::set_var("HOME", &home) };

        let reasons = install_insecurity_reasons().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }
        assert_eq!(reasons.len(), 1);
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("perl", install_insecurity_reasons, home)
}
