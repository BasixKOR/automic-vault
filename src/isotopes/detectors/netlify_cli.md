# Netlify CLI radioisotope

This radioisotope protects Netlify CLI credentials that are normally stored in
plaintext JSON under the user's Netlify global config.

## What it migrates

It stores a single `users.<id>.auth.token` value in the macOS keychain as
`NETLIFY_AUTH_TOKEN`.

The migration rewrites the local config to valid JSON with that Netlify token
field blanked. The wrapper injects `NETLIFY_AUTH_TOKEN` while `netlify` runs.

## Caveats

- Only the default current config path and legacy `~/.netlify/config.json`
  location are migrated.
- Configs with embedded `users.<id>.auth.github.token` values or multiple
  Netlify auth tokens must be migrated manually.
- Direct execution of the original binary will not receive migrated
  credentials.
