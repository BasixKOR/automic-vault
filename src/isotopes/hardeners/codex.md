# Codex

## How Automic Vault Hardens Codex

`av harden codex` performs Codex's supported credential-storage migration:

1. Refuse to proceed while ChatGPT.app is running.
2. Show the complete plan and ask for confirmation.
3. Atomically set `cli_auth_credentials_store = "keyring"` in
   `${CODEX_HOME:-$HOME/.codex}/config.toml`.
4. Run `codex login` and confirm it with `codex login status`.
5. Only after successful verification, delete the old plaintext `auth.json`.

If login or verification fails, Automic Vault restores the original
configuration and keeps `auth.json`. It also refuses to execute a `codex` binary
that does not carry OpenAI's expected code signature.

API-key and access-token logins preserve their existing method by passing the
credential to Codex over stdin. Mixed, malformed, Bedrock, and agent-identity
credential files fail closed for manual migration.

## ChatGPT Desktop

Codex CLI, the IDE extension, and Codex inside the ChatGPT desktop app share
Codex configuration layers. The desktop app's Codex surface may therefore ask
you to sign in again after this change. OpenAI's documentation does not specify
whether this CLI credential-storage setting affects the desktop app's existing
session, so close the app before changing it and expect to reauthenticate.

## Caveats

- `keyring` fails closed when the OS credential store is unavailable; `auto` can
  fall back to plaintext `auth.json`.
- A failed login never removes the existing `auth.json`.
- A project-level `.codex/config.toml` has higher precedence than the user file.
