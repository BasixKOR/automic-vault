# git-credential-oauth Detector

## Trigger Conditions

- Git config enables git-credential-oauth as an ambient credential helper.
- Git config contains a plaintext OAuth client secret.

## Sensitive Files

- `~/.gitconfig`
- `$XDG_CONFIG_HOME/git/config`
- `~/.config/git/config`
