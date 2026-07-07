pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    super::reasons_matching("Secretlint report may contain persisted secret findings")
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    crate::isotopes::detectors::radioisotope::findings(
        "secretlint-persisted-report",
        install_insecurity_reasons,
        home,
    )
}
