pub(crate) const QUALIFIER: &str = "brew:";

pub(crate) fn qualified_name(formula: &str) -> String {
    format!("{QUALIFIER}{formula}")
}
