# jfrog-cli Radioisotope

JFrog CLI stores configured server credentials in its CLI home directory.

This radioisotope migrates a supported single-server credential out of
`jfrog-cli.conf.v6` into the Automic Vault keychain and wraps `jf`/`jfrog` so
the credential is injected as JFrog's own environment-variable authentication
only while the CLI runs.

Supported config shapes are one server with `url` and either `accessToken`, or
`user` plus `password`. The migration stores `JFROG_URL` plus
`JFROG_ACCESS_TOKEN`, or `JFROG_URL` plus `JFROG_USER` and `JFROG_PASSWORD`,
then blanks the persisted secret field. If a token or password is later written
back into `jfrog-cli.conf.v6`, the detector reports it again.

## Caveats

- We currently migrate `jfrog-cli.conf.v6` only, and only the single-server
  token/basic-auth shapes above.
- Configs with refresh tokens, SSH passphrases, both token and password, or
  credentials for multiple servers remain manual/detect-only.
- Direct execution of the original binary will not receive the credentials.
