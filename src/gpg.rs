use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

use pgp::composed::{ArmorOptions, Deserializable, DetachedSignature, SignedSecretKey};
use pgp::crypto::hash::HashAlgorithm;
use pgp::packet::{Signature, SignatureType};
use pgp::types::{KeyDetails, Password};
use zeroize::Zeroizing;

const HELP: &str = "\
av-gpg: Git signing adapter for Automic Vault

Configure Git from Automic Vault Settings. Signing requests are sent to the
bundled av executable; verification and other GPG operations use gpg.
";

pub fn run_git_program() -> i32 {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print!("{HELP}");
        return 0;
    }
    if is_sign_request(&args) {
        delegate_signing(&args)
    } else {
        delegate_gpg(&args)
    }
}

fn is_sign_request(args: &[OsString]) -> bool {
    args.iter().any(|arg| {
        let Some(arg) = arg.to_str() else {
            return false;
        };
        matches!(arg, "--sign" | "--detach-sign")
            || (arg.starts_with('-')
                && !arg.starts_with("--")
                && arg[1..].chars().any(|flag| flag == 's'))
    })
}

fn delegate_signing(args: &[OsString]) -> i32 {
    let av = match bundled_av_path() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("av-gpg: {error}");
            return 1;
        }
    };
    exit_status(Command::new(av).arg("gpg-sign").args(args).status())
}

fn delegate_gpg(args: &[OsString]) -> i32 {
    exit_status(Command::new("gpg").args(args).status())
}

fn exit_status(status: std::io::Result<std::process::ExitStatus>) -> i32 {
    match status {
        Ok(status) => status.code().unwrap_or(1),
        Err(error) => {
            eprintln!("av-gpg: failed to run GPG command: {error}");
            1
        }
    }
}

fn bundled_av_path() -> Result<PathBuf, String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("cannot locate the current executable: {error}"))?;
    let sibling = executable
        .parent()
        .ok_or_else(|| "current executable has no parent directory".to_string())?
        .join("av");
    validate_av_path(sibling)
}

fn validate_av_path(path: PathBuf) -> Result<PathBuf, String> {
    if !path.is_absolute() || !path.is_file() {
        return Err(format!(
            "Automic Vault signing command is missing at {}",
            path.display()
        ));
    }
    Ok(path)
}

pub(crate) fn sign_openpgp(
    armored_private_key: &str,
    passphrase: &str,
    payload: &[u8],
) -> Result<String, String> {
    let key_bytes = Zeroizing::new(armored_private_key.as_bytes().to_vec());
    let (key, _) = SignedSecretKey::from_armor_single(key_bytes.as_slice())
        .map_err(|error| format!("invalid OpenPGP private key: {error}"))?;
    key.verify_bindings()
        .map_err(|error| format!("OpenPGP private key bindings are invalid: {error}"))?;
    if !key.details.revocation_signatures.is_empty() {
        return Err("OpenPGP primary key is revoked".into());
    }
    let password = Password::from(passphrase);
    let signature = if let Some(subkey) = key.secret_subkeys.iter().find(|subkey| {
        subkey.key.algorithm().can_sign()
            && active_signing_binding(&subkey.signatures, subkey.key.created_at())
    }) {
        DetachedSignature::sign_binary_data(
            rand::thread_rng(),
            &subkey.key,
            &password,
            HashAlgorithm::Sha256,
            payload,
        )
    } else if key.primary_key.algorithm().can_sign() && active_primary_signing_binding(&key) {
        DetachedSignature::sign_binary_data(
            rand::thread_rng(),
            &key.primary_key,
            &password,
            HashAlgorithm::Sha256,
            payload,
        )
    } else {
        return Err("OpenPGP key has no active signing-capable primary key or subkey".into());
    }
    .map_err(|error| format!("OpenPGP signing failed: {error}"))?;
    signature
        .to_armored_string(ArmorOptions::default())
        .map_err(|error| format!("failed to encode OpenPGP signature: {error}"))
}

fn active_signing_binding(signatures: &[Signature], key_created: pgp::types::Timestamp) -> bool {
    if signatures
        .iter()
        .any(|signature| signature.typ() == Some(SignatureType::SubkeyRevocation))
    {
        return false;
    }
    signatures
        .iter()
        .filter(|signature| signature.typ() == Some(SignatureType::SubkeyBinding))
        .max_by_key(|signature| signature.created())
        .is_some_and(|signature| {
            signature.key_flags().sign()
                && signature_is_active(signature)
                && key_is_active(key_created, signature.key_expiration_time())
        })
}

fn active_primary_signing_binding(key: &SignedSecretKey) -> bool {
    key.details
        .direct_signatures
        .iter()
        .chain(key.details.users.iter().flat_map(|user| &user.signatures))
        .chain(
            key.details
                .user_attributes
                .iter()
                .flat_map(|attribute| &attribute.signatures),
        )
        .max_by_key(|signature| signature.created())
        .is_some_and(|signature| {
            signature.key_flags().sign()
                && signature_is_active(signature)
                && key_is_active(
                    key.primary_key.created_at(),
                    signature.key_expiration_time(),
                )
        })
}

fn signature_is_active(signature: &Signature) -> bool {
    match (signature.created(), signature.signature_expiration_time()) {
        (_, None) => true,
        (_, Some(lifetime)) if lifetime.as_secs() == 0 => true,
        (Some(created), Some(lifetime)) => std::time::SystemTime::from(created)
            .checked_add(lifetime.into())
            .is_some_and(|expires| expires > std::time::SystemTime::now()),
        (None, Some(_)) => false,
    }
}

fn key_is_active(created: pgp::types::Timestamp, lifetime: Option<pgp::types::Duration>) -> bool {
    match lifetime {
        None => true,
        Some(lifetime) if lifetime.as_secs() == 0 => true,
        Some(lifetime) => std::time::SystemTime::from(created)
            .checked_add(lifetime.into())
            .is_some_and(|expires| expires > std::time::SystemTime::now()),
    }
}

pub(crate) fn public_key_from_private(armored_private_key: &str) -> Result<String, String> {
    let key_bytes = Zeroizing::new(armored_private_key.as_bytes().to_vec());
    let (key, _) = SignedSecretKey::from_armor_single(key_bytes.as_slice())
        .map_err(|error| format!("invalid OpenPGP private key: {error}"))?;
    key.verify_bindings()
        .map_err(|error| format!("OpenPGP private key bindings are invalid: {error}"))?;
    key.to_public_key()
        .to_armored_string(ArmorOptions::default())
        .map_err(|error| format!("failed to encode OpenPGP public key: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pgp::composed::{EncryptionCaps, KeyType, SecretKeyParamsBuilder, SubkeyParamsBuilder};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("av-gpg-{label}-{}-{nanos}", std::process::id()))
    }

    fn signing_key() -> SignedSecretKey {
        let mut signing_subkey = SubkeyParamsBuilder::default();
        signing_subkey
            .key_type(KeyType::Ed25519Legacy)
            .can_sign(true)
            .can_encrypt(EncryptionCaps::None)
            .can_authenticate(false);
        let mut parameters = SecretKeyParamsBuilder::default();
        parameters
            .key_type(KeyType::Ed25519Legacy)
            .can_certify(true)
            .can_sign(false)
            .can_encrypt(EncryptionCaps::None)
            .primary_user_id("av-gpg test <av-gpg@example.invalid>".into())
            .subkeys(vec![signing_subkey.build().unwrap()]);
        parameters
            .build()
            .unwrap()
            .generate(rand::thread_rng())
            .unwrap()
    }

    #[test]
    fn recognizes_git_signing_forms() {
        assert!(is_sign_request(&args(&["-bsau", "DEADBEEF"])));
        assert!(is_sign_request(&args(&["--detach-sign"])));
        assert!(is_sign_request(&args(&["--sign"])));
        assert!(!is_sign_request(&args(&["--status-fd=2", "--verify"])));
    }

    #[test]
    fn refuses_a_relative_av_path() {
        assert!(validate_av_path(PathBuf::from("av")).is_err());
    }

    #[test]
    fn signs_with_an_explicit_signing_subkey() {
        let key = signing_key();
        let armored = key.to_armored_string(ArmorOptions::default()).unwrap();
        let payload = b"tree 0000000000000000000000000000000000000000\n\nav-gpg test\n";

        let signature = sign_openpgp(&armored, "", payload).unwrap();
        let (signature, _) = DetachedSignature::from_armor_single(signature.as_bytes()).unwrap();
        signature
            .verify(&key.secret_subkeys[0].key.public_key(), payload)
            .unwrap();
    }

    #[test]
    fn creates_a_signature_that_gnupg_can_verify() {
        if Command::new("gpg").arg("--version").output().is_err() {
            eprintln!("skipping GnuPG interoperability test: gpg is unavailable");
            return;
        }

        let home = temp_dir("gnupg");
        fs::create_dir_all(&home).unwrap();
        fs::set_permissions(&home, fs::Permissions::from_mode(0o700)).unwrap();
        let key = signing_key();
        let private_key = key.to_armored_string(ArmorOptions::default()).unwrap();
        let public_key = key
            .to_public_key()
            .to_armored_string(ArmorOptions::default())
            .unwrap();
        let public_key_path = home.join("public-key.asc");
        fs::write(&public_key_path, public_key).unwrap();
        let imported = Command::new("gpg")
            .args(["--batch", "--homedir"])
            .arg(&home)
            .arg("--import")
            .arg(&public_key_path)
            .status()
            .unwrap();
        assert!(imported.success());
        let payload = b"tree 0000000000000000000000000000000000000000\n\nav-gpg test\n";
        let signature = sign_openpgp(&private_key, "", payload).unwrap();

        let payload_path = home.join("payload");
        let signature_path = home.join("payload.asc");
        fs::write(&payload_path, payload).unwrap();
        fs::write(&signature_path, signature).unwrap();
        let verified = Command::new("gpg")
            .args(["--batch", "--homedir"])
            .arg(&home)
            .args(["--verify"])
            .arg(signature_path)
            .arg(payload_path)
            .status()
            .unwrap();
        assert!(verified.success());
        fs::remove_dir_all(home).unwrap();
    }
}
