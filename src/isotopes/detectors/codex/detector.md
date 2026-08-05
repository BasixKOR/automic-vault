# codex Detector

## Trigger Conditions

- Codex CLI auth file contains a plaintext API key or ChatGPT token set.

## Sensitive Files

- `${CODEX_HOME:-$HOME/.codex}/auth.json`

## Why This Matters

Codex caches login credentials in a plaintext file by default on every platform,
including macOS. The file is created mode `0600`, which stops other users but not
anything running as you. It holds a refresh token alongside any API key, so a
copy keeps working after the access token expires.

## Why This is not Yet Hardened

Codex can store credentials in the system keyring itself, so the fix belongs in
its configuration rather than in a wrapper. Set `cli_auth_credentials_store` to
`keyring` in `${CODEX_HOME:-$HOME/.codex}/config.toml`, run `codex login` again,
then delete the plaintext file left behind.

```toml
cli_auth_credentials_store = "keyring"
```

```sh
codex login
rm ~/.codex/auth.json
```

The last step matters. Changing the setting neither migrates nor removes the file
already on disk, so the plaintext copy survives until you delete it.

Prefer `keyring` over `auto` on a workstation. `auto` falls back to the plaintext
file when no keyring is available, while `keyring` fails loudly.

[Open an issue to discuss a safer integration](https://github.com/automic-vault/automic-vault/issues).
