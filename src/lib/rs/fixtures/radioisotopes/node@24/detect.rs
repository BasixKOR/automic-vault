use std::{env, fs, path::PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    Ok(!install_insecurity_reasons()?.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = env::var_os("NPM_CONFIG_USERCONFIG")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".npmrc"));
    Ok(fs::read_to_string(path).unwrap_or_default().contains("_authToken").then(|| "npm user config contains plaintext credentials".to_string()).into_iter().collect())
}
