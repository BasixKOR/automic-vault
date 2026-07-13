# zsh Detector

## Trigger Conditions

- Zsh startup file contains plaintext-looking credential assignment.

## Sensitive Files

- `$ZDOTDIR/.zshenv`
- `$ZDOTDIR/.zprofile`
- `$ZDOTDIR/.zshrc`
- `$ZDOTDIR/.zlogin`
- `$ZDOTDIR/.zlogout`
- `~/.zshenv`
- `~/.zprofile`
- `~/.zshrc`
- `~/.zlogin`
- `~/.zlogout`

## Why This is not Yet Hardened

Zsh startup files contain arbitrary user programs and shared environment
configuration. Automic Vault cannot rewrite them without changing shell
behavior or guessing which commands need each secret. Move the reported value
with `av save KEY`, then inject it only into the command that needs it.

[Open an issue to discuss a safer integration](https://github.com/automic-vault/automic-vault/issues).
