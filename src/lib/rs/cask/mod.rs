pub(crate) const QUALIFIER: &str = "cask:";

pub(crate) fn qualified_name(cask: &str) -> String {
    format!("{QUALIFIER}{cask}")
}
