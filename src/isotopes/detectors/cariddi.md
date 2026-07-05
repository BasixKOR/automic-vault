# cariddi Detector

## Trigger Conditions

- cariddi default output can contain discovered secrets.
- Shell history contains cariddi header or custom secret-scanner arguments.

## Sensitive Files

- `~/output-cariddi/secrets/**`
- `~/.zsh_history`
- `~/.bash_history`
- `~/.history`
