# Codex

## How Automic Vault Hardens Codex

`av harden codex` guides Codex's own supported credential-storage migration:

1. Close active Codex sessions.
2. Set `cli_auth_credentials_store = "keyring"` in
   `${CODEX_HOME:-$HOME/.codex}/config.toml`.
3. Run `codex login` and confirm it with `codex login status`.
4. Only then delete the old plaintext `auth.json`.

Automic Vault does not delete `auth.json` automatically. Losing the only working
credential copy would be worse than leaving a clearly reported plaintext copy
for the user to remove after a verified login.

## ChatGPT Desktop

Codex CLI, the IDE extension, and Codex inside the ChatGPT desktop app share
Codex configuration layers. The desktop app's Codex surface may therefore ask
you to sign in again after this change. OpenAI's documentation does not specify
whether this CLI credential-storage setting affects the desktop app's existing
session, so close the app before changing it and expect to reauthenticate.

## Caveats

- `keyring` fails closed when the OS credential store is unavailable; `auto` can
  fall back to plaintext `auth.json`.
- The configuration change does not migrate or remove an existing `auth.json`.
- A project-level `.codex/config.toml` has higher precedence than the user file.
