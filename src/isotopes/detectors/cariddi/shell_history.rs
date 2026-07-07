pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    super::reasons_matching("Shell history contains cariddi")
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    crate::isotopes::detectors::radioisotope::findings(
        "cariddi-shell-history",
        install_insecurity_reasons,
        home,
    )
}
