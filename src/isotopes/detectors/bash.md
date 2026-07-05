# bash Detector

Reports when:
- Bash startup file contains plaintext-looking credential assignment.

## Detection Caveats

- Scans `~/.bashrc`, `~/.bash_profile`, `~/.bash_login`, `~/.profile`, and `BASH_ENV` when set.
- Only literal assignments are reported; empty, masked, command-substitution, and variable-reference values are ignored.
