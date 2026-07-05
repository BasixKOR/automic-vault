# Vagrant Radioisotope

Vagrant stores its Vagrant Cloud login token in
`~/.vagrant.d/data/vagrant_login_token`. The CLI also honors
`VAGRANT_CLOUD_TOKEN`, which is a clean wrapper boundary for runtime
credential injection.

This radioisotope migrates the saved login token into the macOS keychain,
removes the plaintext token file, and wraps `vagrant` so the token is present
only while the command runs.

## Caveats

- We currently migrate only the default Vagrant Cloud login token file.
- Tokens stored outside `VAGRANT_HOME` are not migrated.
- Direct execution of the original launcher will not receive credentials.
