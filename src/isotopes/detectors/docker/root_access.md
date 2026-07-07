# docker-root-access Detector

## Trigger Conditions

- Current user is in the docker group.
- Current user can write the Docker daemon socket.

## Sensitive Files

- `/etc/group`
- `/var/run/docker.sock`
- `/run/docker.sock`
- `Unix socket path named by $DOCKER_HOST`
