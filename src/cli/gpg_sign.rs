use std::ffi::OsString;
use std::io::{Read, Write};

use zeroize::Zeroizing;

use super::inject;

pub(crate) const DEFAULT_PRIVATE_KEY: &str = "AUTOMIC_GPG_SIGNING_PRIVATE_KEY";
pub(crate) const DEFAULT_PASSPHRASE: &str = "AUTOMIC_GPG_SIGNING_PASSPHRASE";
pub(crate) const ALTERNATE_PRIVATE_KEY: &str = "AUTOMIC_GPG_AGENT_SIGNING_PRIVATE_KEY";
pub(crate) const ALTERNATE_PASSPHRASE: &str = "AUTOMIC_GPG_AGENT_SIGNING_PASSPHRASE";
const MAX_SIGNING_PAYLOAD_BYTES: u64 = 16 * 1024 * 1024;

pub(crate) fn run(args: Vec<OsString>, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match sign(args) {
        Ok(signature) => {
            let _ = writeln!(stderr, "[GNUPG:] SIG_CREATED D 1 10 00 0 0 0 0");
            if stdout.write_all(signature.as_bytes()).is_ok() {
                0
            } else {
                1
            }
        }
        Err(error) => {
            let _ = writeln!(stderr, "av gpg-sign: {error}");
            1
        }
    }
}

pub(crate) fn validate(stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let mut private_key = Zeroizing::new(String::new());
    if let Err(error) = std::io::stdin()
        .take(MAX_SIGNING_PAYLOAD_BYTES + 1)
        .read_to_string(&mut private_key)
    {
        let _ = writeln!(stderr, "av gpg-public-key: {error}");
        return 1;
    }
    if private_key.len() as u64 > MAX_SIGNING_PAYLOAD_BYTES {
        let _ = writeln!(stderr, "av gpg-public-key: private key exceeds 16 MiB");
        return 1;
    }
    match bpb::public_key_from_private(&private_key) {
        Ok(public_key) => {
            if stdout.write_all(public_key.as_bytes()).is_ok() {
                0
            } else {
                1
            }
        }
        Err(error) => {
            let _ = writeln!(stderr, "av gpg-public-key: {error}");
            1
        }
    }
}

fn sign(args: Vec<OsString>) -> Result<String, String> {
    let mut args = args
        .into_iter()
        .map(|arg| {
            arg.into_string()
                .map_err(|_| "GPG arguments must be valid UTF-8".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut payload = Vec::new();
    std::io::stdin()
        .take(MAX_SIGNING_PAYLOAD_BYTES + 1)
        .read_to_end(&mut payload)
        .map_err(|error| format!("failed to read Git signing payload: {error}"))?;
    if payload.len() as u64 > MAX_SIGNING_PAYLOAD_BYTES {
        return Err("Git signing payload exceeds 16 MiB".into());
    }
    let payload_digest = ring::digest::digest(&ring::digest::SHA256, &payload);
    let payload_digest = payload_digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    args.push(format!("payload-sha256={payload_digest}"));
    let target = std::env::current_exe()
        .and_then(std::fs::canonicalize)
        .map_err(|error| format!("cannot resolve the signing Target: {error}"))?;
    let cwd = std::env::current_dir()
        .and_then(std::fs::canonicalize)
        .map_err(|error| format!("cannot resolve the working directory: {error}"))?;
    let names = [
        DEFAULT_PRIVATE_KEY,
        DEFAULT_PASSPHRASE,
        ALTERNATE_PRIVATE_KEY,
        ALTERNATE_PASSPHRASE,
    ]
    .map(String::from);
    let mut secrets = inject::approve_gpg_signing(
        target.to_string_lossy().into_owned(),
        args,
        cwd.to_string_lossy().into_owned(),
        &names,
    )?;
    let key_name = if secrets.contains_key(ALTERNATE_PRIVATE_KEY) {
        ALTERNATE_PRIVATE_KEY
    } else {
        DEFAULT_PRIVATE_KEY
    };
    let passphrase_name = if key_name == ALTERNATE_PRIVATE_KEY {
        ALTERNATE_PASSPHRASE
    } else {
        DEFAULT_PASSPHRASE
    };
    let private_key = Zeroizing::new(
        secrets
            .remove(key_name)
            .ok_or_else(|| "Automic Vault returned no private signing key".to_string())?,
    );
    let passphrase = Zeroizing::new(secrets.remove(passphrase_name).unwrap_or_default());
    bpb::sign_openpgp(&private_key, &passphrase, &payload)
}
