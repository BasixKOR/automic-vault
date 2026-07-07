pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    super::reasons_matching(|reason| reason.contains("root-equivalent host access"))
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    crate::isotopes::detectors::radioisotope::findings(
        "docker-root-access",
        install_insecurity_reasons,
        home,
    )
}
