pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    crate::isotopes::detectors::radioisotope::findings(
        "pnpm-auth-token",
        super::install_insecurity_reasons,
        home,
    )
}
