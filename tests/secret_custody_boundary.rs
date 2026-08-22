const RUST_SECRET_CLIENT: &str = include_str!("../src/secrets.rs");
const APPROVAL_SERVICE: &str = include_str!("../src/menu-helper/Sources/MenubarHelper/main.swift");
const APPROVAL_OPERATIONS: &str =
    include_str!("../src/menu-helper/Sources/MenubarHelperCore/ApprovalServiceOperation.swift");
const ARGOCD_MIGRATION: &str = include_str!("../src/isotopes/hardeners/migrations/argocd.rs");

#[test]
fn gate_client_has_no_generic_secret_load_protocol() {
    assert!(!RUST_SECRET_CLIENT.contains("xpc_request(\"load\""));
    assert!(!RUST_SECRET_CLIENT.contains("fn load_secret"));
    assert!(!APPROVAL_SERVICE.contains("handleLoad"));
    assert!(!APPROVAL_SERVICE.contains("case \"load\""));
    assert!(!APPROVAL_OPERATIONS.contains("case load"));
}

#[test]
fn argocd_migration_does_not_retrieve_legacy_vault_secrets() {
    assert!(!ARGOCD_MIGRATION.contains("ARGOCD_CONFIG_YAML"));
    assert!(!ARGOCD_MIGRATION.contains("load_secret"));
    assert!(!ARGOCD_MIGRATION.contains("isotope_copy_generic_password_json"));
}

#[test]
fn gpg_credential_selection_includes_the_fallback_launcher() {
    let handle_inject = APPROVAL_SERVICE
        .split_once("private func handleInject(")
        .unwrap()
        .1;
    let fallback = handle_inject
        .find("if launchers.isEmpty, let launcher = launcherIdentity")
        .unwrap();
    let selection = handle_inject
        .find("if parsedRequest.op == \"gpg-sign\"")
        .unwrap();

    assert!(fallback < selection);
}
