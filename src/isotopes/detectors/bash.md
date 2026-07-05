# Bash Radioisotope Detector

This detector reports plaintext-looking credential assignments in Bash startup
files. It does not modify Bash or rewrite a user's shell configuration.

Detected files:

- `~/.bashrc`
- `~/.bash_profile`
- `~/.bash_login`
- `~/.profile`
- `$BASH_ENV`

Move reported values into Automic Vault with `av save KEY`, then run tools with
`av inject +KEY /absolute/tool` instead of exporting the value from a startup
file.
