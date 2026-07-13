# bash Detector

## Trigger Conditions

- Bash startup file contains plaintext-looking credential assignment.

## Sensitive Files

- `~/.bashrc`
- `~/.bash_profile`
- `~/.bash_login`
- `~/.profile`
- `$BASH_ENV`

## Why This is not Yet Hardened

Bash startup files contain arbitrary user programs and shared environment
configuration. Automic Vault cannot rewrite them without changing shell
behavior or guessing which commands need each secret. Move the reported value
with `av save KEY`, then inject it only into the command that needs it.

[Open an issue to discuss a safer integration](https://github.com/automic-vault/automic-vault/issues).
