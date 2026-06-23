use std::{env, fs, path::PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    Ok(!install_insecurity_reasons()?.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = env::var_os("AWS_SHARED_CREDENTIALS_FILE")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".aws/credentials"));
    let contents = fs::read_to_string(path).unwrap_or_default();
    Ok(contents.contains("aws_secret_access_key").then(|| "AWS shared credentials file contains plaintext credentials".to_string()).into_iter().collect())
}
