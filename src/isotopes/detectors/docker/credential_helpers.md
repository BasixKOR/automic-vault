# docker-credential-helpers Detector

## Trigger Conditions

- Docker config uses an ambient credential store.
- Docker config uses an ambient per-registry credential helper.

## Sensitive Files

- `$DOCKER_CONFIG/config.json`
- `~/.docker/config.json`
