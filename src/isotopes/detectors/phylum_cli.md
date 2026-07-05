# Phylum CLI Radioisotope

Phylum CLI stores its login token in the user config file at
`~/.config/phylum/settings.yaml`. That token can authorize Phylum API requests
and should not remain in plaintext package-owned config.

This radioisotope migrates the default `auth_info.offline_access` token into
the Automic Vault keychain and removes it from the persisted config file. The
installed `phylum` launcher is wrapped so Automic Vault injects the token as
`PHYLUM_API_KEY` while the command runs.

The wrapper runs Phylum with a temporary config file copied from the user's
config with the stored token removed. This preserves non-secret settings while
keeping the runtime token out of the user's config file.

## Caveats

- Only the default XDG config path is migrated.
- Explicit `--config` files are treated as caller-managed and are not migrated.
- Direct execution of the original binary will not receive the injected token.
