# Pulumi Radioisotope

Pulumi stores cloud access tokens in `credentials.json` under the Pulumi home
directory. The radioisotope moves a single access token into the macOS
keychain, removes `accessTokens` from the persisted credentials file, and
injects `PULUMI_ACCESS_TOKEN` only while `pulumi` runs.

Non-secret credentials metadata such as the current backend remains on disk.
The detector reports the credentials file if access tokens reappear.

## Caveats

- Only `credentials.json` files with exactly one non-empty access token are
  migrated.
- Files with multiple access tokens are left unchanged for manual handling
  because one `PULUMI_ACCESS_TOKEN` cannot safely represent every backend.
- Direct execution of the original binary will not receive credentials.
