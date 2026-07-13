# docker-credential-helper Detector

## Trigger Conditions

- Docker config uses an ambient Docker credential helper.

## Sensitive Files

- `$DOCKER_CONFIG/config.json`
- `~/.docker/config.json`

## Why This is not Yet Hardened

Docker credential helpers are a credential-store boundary, not a normal CLI
secret file. This detector reports Docker config that uses the packaged helpers
without changing Docker's helper settings.

[Open an issue to discuss a safer integration](https://github.com/automic-vault/automic-vault/issues).
