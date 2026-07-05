# zsh Detector

Reports when:
- Zsh startup file contains plaintext-looking credential assignment.

## Detection Caveats

- Scans `.zshenv`, `.zprofile`, `.zshrc`, `.zlogin`, and `.zlogout` under `ZDOTDIR` when set, otherwise under `HOME`.
- Only literal assignments are reported; empty, masked, command-substitution, and variable-reference values are ignored.
