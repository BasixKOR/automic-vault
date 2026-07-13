# git-credentials-file Detector

## Trigger Conditions

- Git credential store contains plaintext credentials.
- Git config enables a plaintext Git credential-store file.

## Sensitive Files

- `~/.git-credentials`
- `~/.gitconfig`
- `$XDG_CONFIG_HOME/git/config`
- `~/.config/git/config`
- `credential-store files referenced by global Git config`

## Mitigation

Remove credentials from the reported files, disable Git's plaintext `store`
helper, and use SSH remotes or a credential helper backed by the system
keychain.
