#![allow(dead_code)]

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    let credentials_path = match std::env::var_os("AWS_SHARED_CREDENTIALS_FILE") {
        Some(path) if !path.is_empty() => std::path::PathBuf::from(path),
        _ => {
            let home = home.as_ref().ok_or_else(|| "HOME is not set".to_string())?;
            home.join(".aws/credentials")
        }
    };
    let config_path = match (std::env::var_os("AWS_CONFIG_FILE"), home.as_ref()) {
        (Some(path), _) if !path.is_empty() => Some(std::path::PathBuf::from(path)),
        (_, Some(home)) => Some(home.join(".aws/config")),
        _ => None,
    };

    if credentials_path.exists() && credentials_file_is_insecure(&credentials_path)? {
        reasons.push(format!(
            "AWS shared credentials file contains plaintext access keys: {}",
            credentials_path.display()
        ));
    }
    if let Some(config_path) = &config_path {
        if config_path.exists() && config_file_has_legacy_plugins(config_path)? {
            reasons.push(format!(
                "AWS CLI legacy plugins are configured: {}",
                config_path.display()
            ));
        }
    }

    if let Some(home) = home {
        let login_cache_path = home.join(".aws/login/cache");
        if login_cache_is_insecure(&login_cache_path)? {
            reasons.push(format!(
                "AWS login cache contains cached access credentials: {}",
                login_cache_path.display()
            ));
        }
    }

    Ok(reasons)
}

fn credentials_file_is_insecure(path: &std::path::Path) -> Result<bool, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let mut in_profile = false;

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }

        if trimmed.starts_with('[') {
            in_profile = trimmed
                .strip_prefix('[')
                .and_then(|section| section.strip_suffix(']'))
                .map(str::trim)
                .is_some_and(|section| !section.is_empty());
            continue;
        }

        if !in_profile {
            continue;
        }

        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if is_credentials_file_secret_key(key) && !value.trim().is_empty() {
            return Ok(true);
        }
    }

    Ok(false)
}

fn login_cache_is_insecure(path: &std::path::Path) -> Result<bool, String> {
    if !path.exists() {
        return Ok(false);
    }

    let entries = std::fs::read_dir(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|err| format!("failed to read entry in {}: {err}", path.display()))?;
        let file_type = entry
            .file_type()
            .map_err(|err| format!("failed to stat {}: {err}", entry.path().display()))?;
        if !file_type.is_file()
            || entry.path().extension().and_then(|value| value.to_str()) != Some("json")
        {
            continue;
        }

        let contents = std::fs::read_to_string(entry.path())
            .map_err(|err| format!("failed to read {}: {err}", entry.path().display()))?;
        if login_cache_file_is_insecure(&contents) {
            return Ok(true);
        }
    }

    Ok(false)
}

fn config_file_has_legacy_plugins(path: &std::path::Path) -> Result<bool, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let mut in_plugins_section = false;

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_plugins_section = trimmed
                .strip_prefix('[')
                .and_then(|section| section.strip_suffix(']'))
                .map(str::trim)
                == Some("plugins");
            continue;
        }

        if !in_plugins_section {
            continue;
        }

        let Some((_, value)) = trimmed.split_once('=') else {
            return Ok(true);
        };
        if !value.trim().is_empty() {
            return Ok(true);
        }
    }

    Ok(false)
}

fn login_cache_file_is_insecure(contents: &str) -> bool {
    contains_json_string_value(contents, "accessKeyId")
        || contains_json_string_value(contents, "secretAccessKey")
        || contains_json_string_value(contents, "AWS_ACCESS_KEY_ID")
        || contains_json_string_value(contents, "AWS_SECRET_ACCESS_KEY")
}

fn contains_json_string_value(contents: &str, key: &str) -> bool {
    let needle = format!("\"{key}\"");
    let mut remaining = contents;
    while let Some(key_index) = remaining.find(&needle) {
        let Some(after_key) = remaining[key_index..]
            .split_once(':')
            .map(|(_, value)| value)
        else {
            return false;
        };
        let value = after_key.trim_start();
        if value.starts_with('"') && value.get(1..).is_some_and(|value| !value.starts_with('"')) {
            return true;
        }
        if after_key.is_empty() {
            return false;
        }
        remaining = &after_key[1..];
    }
    false
}

fn is_credentials_file_secret_key(key: &str) -> bool {
    key == "aws_access_key_id" || key == "aws_secret_access_key"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_secret_access_key_in_credentials_file() {
        let contents = "[default]\naws_secret_access_key = secret\n";
        let path =
            std::env::temp_dir().join(format!("aws-cli-detect-credentials-{}", std::process::id()));
        std::fs::write(&path, contents).unwrap();

        let result = credentials_file_is_insecure(&path);

        std::fs::remove_file(path).unwrap();
        assert_eq!(result, Ok(true));
    }

    #[test]
    fn detects_secret_access_key_in_named_credentials_profile() {
        let contents = "[profile dev]\naws_secret_access_key = secret\n";
        let path = std::env::temp_dir().join(format!(
            "aws-cli-detect-profile-credentials-{}",
            std::process::id()
        ));
        std::fs::write(&path, contents).unwrap();

        let result = credentials_file_is_insecure(&path);

        std::fs::remove_file(path).unwrap();
        assert_eq!(result, Ok(true));
    }

    #[test]
    fn detects_login_cache_credentials() {
        let contents = r#"{"Credentials":{"accessKeyId":"AKIA","secretAccessKey":"secret"}}"#;

        assert!(login_cache_file_is_insecure(contents));
    }

    #[test]
    fn detects_legacy_plugins_in_config_file() {
        let path =
            std::env::temp_dir().join(format!("aws-cli-detect-config-{}", std::process::id()));
        std::fs::write(&path, "[plugins]\nexample = example.plugin\n").unwrap();

        let result = config_file_has_legacy_plugins(&path);

        std::fs::remove_file(path).unwrap();
        assert_eq!(result, Ok(true));
    }

    #[test]
    fn ignores_empty_legacy_plugin_section() {
        let path = std::env::temp_dir().join(format!(
            "aws-cli-detect-empty-config-{}",
            std::process::id()
        ));
        std::fs::write(&path, "[plugins]\n[default]\nregion = us-east-1\n").unwrap();

        let result = config_file_has_legacy_plugins(&path);

        std::fs::remove_file(path).unwrap();
        assert_eq!(result, Ok(false));
    }

    #[test]
    fn ignores_empty_login_cache_credentials() {
        let contents = r#"{"Credentials":{"accessKeyId":"","secretAccessKey":""}}"#;

        assert!(!login_cache_file_is_insecure(contents));
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_default_locations_are_missing() {
        let home = std::env::temp_dir().join(format!(
            "{}-detect-missing-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let xdg = home.join("xdg");
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&xdg).unwrap();

        let previous_home = std::env::var_os("HOME");
        let previous_xdg = std::env::var_os("XDG_CONFIG_HOME");
        let previous_config = std::env::var_os("AWS_CONFIG_FILE");
        let previous_credentials = std::env::var_os("AWS_SHARED_CREDENTIALS_FILE");
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("XDG_CONFIG_HOME", &xdg);
            std::env::remove_var("AWS_CONFIG_FILE");
            std::env::remove_var("AWS_SHARED_CREDENTIALS_FILE");
        }

        let result = install_is_insecure().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_xdg {
                Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
            match previous_config {
                Some(value) => std::env::set_var("AWS_CONFIG_FILE", value),
                None => std::env::remove_var("AWS_CONFIG_FILE"),
            }
            match previous_credentials {
                Some(value) => std::env::set_var("AWS_SHARED_CREDENTIALS_FILE", value),
                None => std::env::remove_var("AWS_SHARED_CREDENTIALS_FILE"),
            }
        }

        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("aws-cli", install_insecurity_reasons, home)
}
