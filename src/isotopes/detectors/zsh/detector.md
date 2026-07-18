# zsh Detector

## Trigger Conditions

- Zsh startup file contains plaintext-looking credential assignment.
- Zsh `PATH` places a user-writable directory before protected system
  directories.

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
- Directories listed in `$PATH`

## Mitigation

Zsh startup files contain arbitrary user programs and shared environment
configuration. Automic Vault cannot rewrite them without changing shell
behavior or guessing which commands need each secret. Move the reported value
with `av save KEY`, then inject it only into the command that needs it. For an
unsafe `PATH`, move every protected system directory before the reported
user-writable directories and remove empty or relative entries.

## Why This is not Yet Hardened

Automic Vault does not rewrite shell startup programs because doing so could
change command resolution, alter shell behavior, or execute attacker-controlled
configuration.

[Open an issue to discuss a safer integration](https://github.com/automic-vault/automic-vault/issues).
