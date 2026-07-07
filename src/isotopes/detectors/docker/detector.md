# docker Detector

## Trigger Conditions

- Docker legacy config contains registry credentials.
- Docker Desktop is installed without an Automic Vault default credential helper.
- Current user can write the Docker daemon socket, which grants root-equivalent host access through root containers and writable bind mounts.
- Docker config contains inline registry credentials.
- Docker config uses an ambient credential store.
- Docker config uses an ambient per-registry credential helper.

## Sensitive Files

- `$DOCKER_CONFIG/config.json`
- `~/.docker/config.json`
- `~/.dockercfg`
- `/etc/group`
- `/var/run/docker.sock`
- `/run/docker.sock`
- `Unix socket path named by $DOCKER_HOST`
