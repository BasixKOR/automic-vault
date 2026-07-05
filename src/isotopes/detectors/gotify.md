# Gotify Radioisotope

Gotify CLI stores an application token in JSON config. Upstream searches
`./cli.json`, the XDG config location, `~/.gotify/cli.json`, and `/etc`.

This radioisotope migrates user-level config tokens into Automic Vault and
rewrites those files without the token. The wrapper then injects
`GOTIFY_TOKEN` for the real `gotify` process, while leaving non-secret config
such as the server URL available in the original config files.

Project-local `./cli.json` files are detected by Gotify itself, but this
radioisotope does not migrate arbitrary working-directory files because those
belong to the caller or project rather than the package install.
