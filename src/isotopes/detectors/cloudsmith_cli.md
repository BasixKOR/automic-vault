# cloudsmith-cli Radioisotope

Cloudsmith CLI uses two INI files: `config.ini` for non-secret settings and
`credentials.ini` for API authentication. The current CLI stores SSO access
and refresh tokens in the system keyring, but API keys can still be written to
`credentials.ini`.

This radioisotope migrates a single API key from default macOS
`credentials.ini` files into Automic Vault, removes `api_key` from the local
file, and injects `CLOUDSMITH_API_KEY` only while the CLI runs. Non-secret
profile and host settings remain on disk.

## Migrated Files

- `~/Library/Application Support/cloudsmith/credentials.ini`
- `~/.cloudsmith/credentials.ini`

The CLI also looks in the current working directory. This radioisotope does not
migrate `./credentials.ini` because that location is project data rather than a
stable package-owned user credential store.

## Caveats

- SSO keyring entries are left to Cloudsmith CLI's own keyring integration.
- Credentials files with multiple API keys are left unchanged for manual
  handling because one `CLOUDSMITH_API_KEY` cannot safely represent every
  profile.
- Running the original binary directly will not receive injected credentials.
