# docker-credential-helper Detector

## Trigger Conditions

- Docker config uses an ambient Docker credential helper.

## Sensitive Files

- `$DOCKER_CONFIG/config.json`
- `~/.docker/config.json`
