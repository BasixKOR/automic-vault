# gh-cli Detector

## Trigger Conditions

- GitHub CLI `hosts.yml` contains a non-empty `oauth_token` entry.
- On macOS, a GitHub CLI Keychain item for `gh:github.com` or a host listed in `hosts.yml` allows `/usr/bin/security` to read the secret without an interactive prompt.
- The Keychain check inspects generic-password item ACLs; it does not correspond to a sensitive file on disk.

## Sensitive Files

- `$GH_CONFIG_DIR/hosts.yml`
- `$XDG_CONFIG_HOME/gh/hosts.yml`
- `~/.config/gh/hosts.yml`
