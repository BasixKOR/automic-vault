# gallery-dl

gallery-dl stores site credentials and OAuth values in user configuration
files such as `~/.config/gallery-dl/config.json` and `~/.gallery-dl.conf`.

This radioisotope migrates those user config files to the keychain and wraps
`gallery-dl` so they are recreated under a temporary home/config tree while
the CLI runs.

