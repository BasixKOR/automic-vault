# git-credential-fill Detector

## Trigger Conditions

- Git config delegates GitHub credentials to `gh auth git-credential`.
- `git credential fill` returns a GitHub password or token for github.com.

## Sensitive Files

- `~/.gitconfig`
- `$XDG_CONFIG_HOME/git/config`
- `~/.config/git/config`
