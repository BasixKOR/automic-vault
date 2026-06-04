# Zsh Radioisotope Detector

This detector reports plaintext-looking credential assignments in Zsh startup
files. It does not modify Zsh or rewrite a user's shell configuration.

Detected files under `$ZDOTDIR`, or under `$HOME` when `ZDOTDIR` is unset:

- `.zshenv`
- `.zprofile`
- `.zshrc`
- `.zlogin`
- `.zlogout`

Move reported values into Automic Vault with `av save KEY`, then run tools with
`av inject +KEY /absolute/tool` instead of exporting the value from a startup
file.
