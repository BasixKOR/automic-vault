# httpie Detector

## Trigger Conditions

- HTTPie session contains plaintext auth material.

## Sensitive Files

- `$XDG_CONFIG_HOME/httpie/sessions/**/default.json`
- `~/.config/httpie/sessions/**/default.json`
- `~/.httpie/sessions/**/default.json`
