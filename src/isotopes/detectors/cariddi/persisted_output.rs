pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    super::reasons_matching("cariddi default output can contain discovered secrets")
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    crate::isotopes::detectors::radioisotope::findings(
        "cariddi-persisted-output",
        install_insecurity_reasons,
        home,
    )
}
