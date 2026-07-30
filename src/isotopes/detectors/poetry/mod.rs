#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in candidate_auth_files()? {
        if path.exists() && poetry_auth_contains_secret(&read_to_string(&path)?) {
            reasons.push(format!(
                "Poetry auth.toml contains plaintext repository credentials: {}",
                path.display()
            ));
        }
    }
    reasons.sort();
    reasons.dedup();
    Ok(reasons)
}

fn candidate_auth_files() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    let mut paths = vec![
        home.join(".config/pypoetry/auth.toml"),
        home.join("Library/Application Support/pypoetry/auth.toml"),
        home.join("Library/Preferences/pypoetry/auth.toml"),
    ];
    if let Some(config_home) = xdg_config_home() {
        paths.push(config_home.join("pypoetry/auth.toml"));
    }
    Ok(paths)
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn xdg_config_home() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn poetry_auth_contains_secret(contents: &str) -> bool {
    let mut in_secret_table = false;
    let mut in_pypi_token_table = false;
    for line in contents.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            let header = &line[1..line.len() - 1];
            in_secret_table = header.contains("pypi-token") || header.contains("http-basic");
            // `poetry config pypi-token.<repo> <token>` writes a flat
            // `[pypi-token]` table keyed by repository name, e.g.
            // `[pypi-token]\npypi = "..."`, unlike `[http-basic.<repo>]`
            // which nests `password`/`token` keys per repo.
            in_pypi_token_table = header == "pypi-token";
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().trim_matches('"').trim_matches('\'');
        if in_pypi_token_table && secret_value(trim_quotes(value.trim())) {
            return true;
        }
        if in_secret_table
            && matches!(key, "password" | "token")
            && secret_value(trim_quotes(value.trim()))
        {
            return true;
        }
        if key.contains("pypi-token") && secret_value(trim_quotes(value.trim())) {
            return true;
        }
    }
    false
}

fn trim_quotes(value: &str) -> &str {
    value.trim_matches('"').trim_matches('\'')
}

fn secret_value(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && !value.contains("${") && !value.eq_ignore_ascii_case("changeme")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_poetry_auth_toml_secrets() {
        assert!(poetry_auth_contains_secret(
            "[pypi-token.pypi]\ntoken = \"pypi-secret\"\n"
        ));
        assert!(poetry_auth_contains_secret(
            "[http-basic.private]\nusername = \"u\"\npassword = \"p\"\n"
        ));
        assert!(!poetry_auth_contains_secret(
            "[http-basic.private]\nusername = \"u\"\n"
        ));
    }

    #[test]
    fn detects_pypi_token_in_the_flat_table_shape_poetry_actually_writes() {
        // `poetry config pypi-token.pypi <token>` writes auth.toml via
        // Config.auth_config_source.add_property("pypi-token.pypi", token),
        // which serializes as a flat table keyed by repository name, not a
        // dotted subtable with a `token =` key.
        assert!(poetry_auth_contains_secret(
            "[pypi-token]\npypi = \"pypi-AgEIcHlwaS5vcmc\"\n"
        ));
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("poetry", install_insecurity_reasons, home)
}
