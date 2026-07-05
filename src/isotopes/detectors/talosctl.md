# talosctl

`talosctl` reads its client configuration from `~/.talos/config` by default.
That talosconfig can contain client certificate, private key, CA, and basic auth
material for Talos clusters.

This radioisotope migrates the default talosconfig to the keychain and wraps
`talosctl` so it is recreated under a temporary home and selected with
`TALOSCONFIG` while the CLI runs.
