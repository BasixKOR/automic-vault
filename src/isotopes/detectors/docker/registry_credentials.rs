pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    super::reasons_matching(|reason| {
        reason.contains("inline registry credentials")
            || reason.contains("legacy config contains registry credentials")
    })
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    crate::isotopes::detectors::radioisotope::findings(
        "docker-registry-credentials",
        install_insecurity_reasons,
        home,
    )
}
