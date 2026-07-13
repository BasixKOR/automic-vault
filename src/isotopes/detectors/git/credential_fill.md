# git-credential-fill Detector

## Trigger Conditions

- Git config delegates GitHub credentials to `gh auth git-credential`.
- `git credential fill` returns a GitHub password or token for github.com.

## Sensitive Files

- `~/.gitconfig`
- `$XDG_CONFIG_HOME/git/config`
- `~/.config/git/config`

## Mitigation

Remove the affected credential helper from Git config and change GitHub remotes
to SSH. Reject any cached GitHub credential with `git credential reject`.
