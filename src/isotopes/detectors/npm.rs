#![allow(dead_code)]

use std::path::PathBuf;

const SOLUTION: &str = "Set `min-release-age=7` in the reported npm config file.";

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::js_release_age::policy_findings(
        "npm",
        home,
        npm_config_paths(home),
        super::js_release_age::npm_days,
        SOLUTION,
    )
}

fn npm_config_paths(home: &std::path::Path) -> Vec<PathBuf> {
    if let Some(path) = std::env::var_os("NPM_CONFIG_USERCONFIG").filter(|value| !value.is_empty())
    {
        return vec![PathBuf::from(path)];
    }
    vec![home.join(".npmrc")]
}
