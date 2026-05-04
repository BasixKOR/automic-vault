pub(crate) fn qualified_name(package: &str) -> String {
    format!("pip:{package}")
}

pub(crate) fn install_leaf_name(package: &str) -> String {
    package.to_string()
}
