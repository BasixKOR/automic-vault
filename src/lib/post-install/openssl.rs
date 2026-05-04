use std::fs;
use std::path::Path;

pub(super) fn supports(formula: &str) -> bool {
    formula == "openssl@3"
}

pub(super) fn post_install(prefix: &Path) -> Result<(), String> {
    let source_dir = prefix.join(super::super::OPENSSL_CA_CERTIFICATES_DIR);
    let target_dir = prefix.join(super::super::OPENSSL_CERT_PEM_DESTINATION_DIR);

    if path_exists(&source_dir) {
        move_ca_certificates_dir(&source_dir, &target_dir)?;
    }

    let source_cert_name = Path::new(super::super::OPENSSL_CA_CERTIFICATES_CERT)
        .file_name()
        .ok_or_else(|| {
            format!(
                "invalid OpenSSL cert source path {}",
                super::super::OPENSSL_CA_CERTIFICATES_CERT
            )
        })?;
    let source_cert = target_dir.join(source_cert_name);
    let target_cert = target_dir.join("cert.pem");
    if path_exists(&source_cert) {
        if path_exists(&target_cert) {
            super::super::remove_path(&target_cert)?;
        }
        fs::rename(&source_cert, &target_cert).map_err(|err| {
            format!(
                "failed to move {} to {}: {err}",
                source_cert.display(),
                target_cert.display()
            )
        })?;
    }

    Ok(())
}

fn move_ca_certificates_dir(source_dir: &Path, target_dir: &Path) -> Result<(), String> {
    if !path_exists(target_dir) {
        fs::rename(source_dir, target_dir).map_err(|err| {
            format!(
                "failed to move {} to {}: {err}",
                source_dir.display(),
                target_dir.display()
            )
        })?;
        return Ok(());
    }

    let entries = fs::read_dir(source_dir)
        .map_err(|err| format!("failed to read {}: {err}", source_dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("failed to read {}: {err}", source_dir.display()))?;
        super::super::merge_path_into(&entry.path(), &target_dir.join(entry.file_name()))?;
    }
    fs::remove_dir(source_dir)
        .map_err(|err| format!("failed to remove {}: {err}", source_dir.display()))
}

fn path_exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}
