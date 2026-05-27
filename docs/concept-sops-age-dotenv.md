# SOPS + age Dotenv Concept

This document sketches a possible Automic Vault workflow for encrypted dotenv
files backed by SOPS and age. It is a concept document, not a committed CLI
contract.

## Problem

Dotenv files are useful because many developer tools and applications already
expect configuration in environment variables. The problem is that plaintext
`.env` files are also easy for local agents to read, summarize, copy into model
context, or pass to subprocesses.

The desirable workflow is:

1. Keep dotenv values encrypted in the project.
2. Keep the age private identity out of the project and out of agent-readable
   files.
3. Decrypt only at execution time.
4. Inject the resulting environment into one approved target process without
   writing plaintext dotenv contents to disk.

## Boundary Model

`sops` should remain a green skip. It does not own a package-local plaintext
secret store. SOPS encrypts and decrypts documents, but the sensitive key
material is delegated to providers such as age, GnuPG, or cloud KMS.

`age` is different. Plaintext age identity files can contain reusable private
key material such as `AGE-SECRET-KEY-...`. Those files are not package-owned in
the usual radioisotope sense, but they are still a meaningful exposure for
Automic Vault to detect and eventually mediate.

For this workflow, Automic Vault should hold the age private identity in the
macOS Keychain and provide it to SOPS only through an approved helper boundary.

## Proposed Workflow

Projects commit an encrypted dotenv file, for example:

```text
.env.enc
.sops.yaml
```

The user's age private identity is stored in Automic Vault under a keychain
account such as `SOPS_AGE_KEY`. The public age recipient is safe to keep in
`.sops.yaml`.

A candidate command shape is:

```sh
av dotenv run --file .env.enc -- <command> [args...]
```

At runtime, Automic Vault would:

1. Launch the AV-managed `sops` executable to decrypt `.env.enc` as dotenv.
2. Provide the age identity through SOPS' command hook:

   ```sh
   SOPS_AGE_KEY_CMD="/usr/local/bin/av credential-helper sops-age --key SOPS_AGE_KEY"
   ```

3. Capture decrypted dotenv output in memory.
4. Parse the dotenv keys and values.
5. Ask for approval showing the encrypted file path, target command, and
   variable names only.
6. Execute the target command with the decrypted variables in its environment.

The decrypted dotenv content and age identity should not be written to a
persistent file by Automic Vault.

## Non-Goals

- Do not build a `gnupg` wrapper isotope for this workflow. GnuPG is already a
  key-management boundary mediated by `gpg-agent` and `pinentry`.
- Do not build a `sops` wrapper radioisotope. SOPS does not store package-owned
  plaintext secrets.
- Do not require users to keep a plaintext age `keys.txt` file on disk.
- Do not write the decrypted dotenv file to disk as part of `av dotenv run`.
- Do not treat this concept document as a committed CLI/API contract.

## Detection

`sops` should remain a green skip in scan metadata because it does not own
plaintext secrets.

`age` should get detector/helper treatment. A detector should report plaintext
age identities containing `AGE-SECRET-KEY-` in bounded common locations, such as
SOPS age defaults:

```text
$XDG_CONFIG_HOME/sops/age/keys.txt
~/Library/Application Support/sops/age/keys.txt
~/.config/sops/age/keys.txt
```

That detector would not make `age` a normal wrapper radioisotope. It would make
the unmanaged key-file risk visible and give Automic Vault a clear remediation
path: move the age identity into the Keychain and expose it through an approved
helper only while SOPS decrypts.

## Open Questions

- Should v1 support only `SOPS_AGE_KEY_CMD`, or should it also support a private
  temporary `SOPS_AGE_KEY_FILE` for compatibility?
- Should always-allow be disabled initially for `av dotenv run`?
- Should dotenv parsing be strict and AV-owned, or should Automic Vault trust
  SOPS' dotenv output format and keep parsing minimal?
