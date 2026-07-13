# git-credential-oauth Detector

## Trigger Conditions

- Git config enables git-credential-oauth as an ambient credential helper.
- Git config contains a plaintext OAuth client secret.

## Sensitive Files

- `~/.gitconfig`
- `$XDG_CONFIG_HOME/git/config`
- `~/.config/git/config`

## Why This is not Yet Hardened

git-credential-oauth is a credential helper rather than a normal application
secret store. This detector reports global helper use and plaintext OAuth client
secrets in Git config without changing Git helper order.

[Open an issue to discuss a safer integration](https://github.com/automic-vault/automic-vault/issues).
