#![allow(dead_code)]

use std::path::PathBuf;

const SOLUTION: &str =
    "Set `minimumReleaseAge = 86400` under `[install]` in the reported Bun config file.";

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::js_release_age::policy_findings(
        "bun",
        home,
        bun_config_paths(home),
        super::js_release_age::bun_seconds,
        SOLUTION,
    )
}

fn bun_config_paths(home: &std::path::Path) -> Vec<PathBuf> {
    if let Some(path) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return vec![PathBuf::from(path).join(".bunfig.toml")];
    }
    vec![home.join(".bunfig.toml")]
}
