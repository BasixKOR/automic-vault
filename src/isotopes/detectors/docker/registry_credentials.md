# docker-registry-credentials Detector

## Trigger Conditions

- Docker legacy config contains registry credentials.
- Docker config contains inline registry credentials.

## Sensitive Files

- `$DOCKER_CONFIG/config.json`
- `~/.docker/config.json`
- `~/.dockercfg`
