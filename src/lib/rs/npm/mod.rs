use std::path::PathBuf;

pub(crate) fn qualified_name(package: &str) -> String {
    format!("npm:{package}")
}

pub(crate) fn install_relative_path(package: &str) -> PathBuf {
    if let Some(scoped) = package.strip_prefix('@') {
        if let Some((scope, name)) = scoped.split_once('/') {
            return PathBuf::from(format!("@{scope}")).join(name);
        }
    }

    PathBuf::from(package)
}

pub(crate) fn install_leaf_name(package: &str) -> String {
    install_relative_path(package)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(package)
        .to_string()
}

pub(crate) fn executable_name(package: &str) -> String {
    install_leaf_name(package)
}
