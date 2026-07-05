# Travis CLI Radioisotope

`travis` stores Travis CI API tokens in `~/.travis/config.yml` after login.
Those tokens authorize API access for the configured Travis endpoint.

This radioisotope migrates a single configured token into Automic Vault,
removes `access_token` from the local file, and injects `TRAVIS_TOKEN` only
while the Travis CLI runs. Non-secret endpoint config remains on disk.

Configs with multiple endpoint tokens are left unchanged for manual handling,
because one `TRAVIS_TOKEN` value cannot safely represent every endpoint.
Direct execution of the original binary does not receive the migrated token.
