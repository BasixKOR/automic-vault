# Mercurial

Mercurial reads user configuration from `~/.hgrc` and
`~/.config/hg/hgrc`. Those files can contain `[auth]` credentials, including
password fields and password-bearing remote URLs.

This radioisotope migrates those hgrc files to the keychain and wraps `hg` so
they are recreated under a temporary home/config tree while Mercurial runs.
