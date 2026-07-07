# secretlint-shell-history Detector

## Trigger Conditions

- Shell history contains Secretlint invocations that can expose unmasked secrets.

## Sensitive Files

- `~/.zsh_history`
- `~/.bash_history`
- `~/.history`
