# git Detector

## Trigger Conditions

- Git credential store contains plaintext credentials.
- Git config enables a plaintext Git credential-store file.
- Git credential helper delegates github.com credentials to `gh auth git-credential`.
- `git credential fill` returns a GitHub password or token for github.com.
- Git config enables git-credential-oauth as an ambient credential helper.
- Git config contains a plaintext OAuth client secret.

## Sensitive Files

- `~/.git-credentials`
- `~/.gitconfig`
- `$XDG_CONFIG_HOME/git/config`
- `~/.config/git/config`
- `credential-store files referenced by global Git config`
