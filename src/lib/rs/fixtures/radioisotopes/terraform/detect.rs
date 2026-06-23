use std::{env, fs, path::PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    Ok(!install_insecurity_reasons()?.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".terraform.d/credentials.tfrc.json");
    Ok(fs::read_to_string(path).unwrap_or_default().contains("token").then(|| "Terraform credentials file contains plaintext credentials".to_string()).into_iter().collect())
}
