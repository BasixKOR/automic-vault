#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for config in nuget_configs()? {
        if config.path.exists() && config_has_secrets(&read_to_string(&config.path)?) {
            reasons.push(format!(
                "NuGet user config contains package credentials: {}",
                config.path.display()
            ));
        }
    }
    Ok(reasons)
}

fn nuget_configs() -> Result<Vec<NuGetConfig>, String> {
    let home = user_home()?;
    let mono_config_root = if let Some(path) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(path).join("NuGet")
    } else {
        home.join(".config/NuGet")
    };

    Ok(vec![
        NuGetConfig {
            path: mono_config_root.join("NuGet.Config"),
        },
        NuGetConfig {
            path: home.join(".nuget/NuGet/NuGet.Config"),
        },
    ])
}

struct NuGetConfig {
    path: PathBuf,
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn config_has_secrets(contents: &str) -> bool {
    let lower = contents.to_ascii_lowercase();
    has_configured_api_key(&lower)
        || has_package_source_password(&lower)
        || has_proxy_password(&lower)
        || has_client_certificate_password(&lower)
}

fn has_configured_api_key(lower: &str) -> bool {
    lower.contains("<apikeys")
        && lower.contains("<add")
        && lower.contains("value=")
        && !lower.contains("value=\"\"")
}

fn has_package_source_password(lower: &str) -> bool {
    lower.contains("<packagesourcecredentials")
        && (lower.contains("key=\"password\"")
            || lower.contains("key=\"cleartextpassword\"")
            || lower.contains("key='password'")
            || lower.contains("key='cleartextpassword'"))
}

fn has_proxy_password(lower: &str) -> bool {
    lower.contains("http_proxy.password")
        && lower.contains("<add")
        && lower.contains("value=")
        && !lower.contains("value=\"\"")
}

fn has_client_certificate_password(lower: &str) -> bool {
    (lower.contains("password=") || lower.contains("cleartextpassword="))
        && (lower.contains("<clientcertificate") || lower.contains("<certificate"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_package_source_credentials() {
        assert!(config_has_secrets(
            r#"<configuration>
  <packageSourceCredentials>
    <private>
      <add key="Username" value="me" />
      <add key="ClearTextPassword" value="secret" />
    </private>
  </packageSourceCredentials>
</configuration>"#
        ));
    }

    #[test]
    fn detects_api_keys() {
        assert!(config_has_secrets(
            r#"<configuration><apikeys><add key="https://api.nuget.org/v3/index.json" value="secret" /></apikeys></configuration>"#
        ));
    }

    #[test]
    fn detects_proxy_passwords() {
        assert!(config_has_secrets(
            r#"<configuration><config><add key="http_proxy.password" value="secret" /></config></configuration>"#
        ));
    }

    #[test]
    fn ignores_plain_sources() {
        assert!(!config_has_secrets(
            r#"<configuration><packageSources><add key="nuget.org" value="https://api.nuget.org/v3/index.json" /></packageSources></configuration>"#
        ));
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
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("XDG_CONFIG_HOME", &xdg);
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
        }

        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("nuget", install_insecurity_reasons, home)
}
