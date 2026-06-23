use std::{env, fs, path::PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    Ok(!install_insecurity_reasons()?.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = env::var_os("GH_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".config/gh"))
        .join("hosts.yml");
    let contents = fs::read_to_string(path).unwrap_or_default();
    Ok(contents.contains("oauth_token").then(|| "GitHub CLI hosts file contains plaintext credentials".to_string()).into_iter().collect())
}
