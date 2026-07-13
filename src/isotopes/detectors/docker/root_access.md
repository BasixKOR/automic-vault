# docker-root-access Detector

## Trigger Conditions

- Current user is in the docker group.
- Current user can write the Docker daemon socket.

## Sensitive Files

- `/etc/group`
- `/var/run/docker.sock`
- `/run/docker.sock`
- `Unix socket path named by $DOCKER_HOST`

## Why This is not Yet Hardened

Docker daemon access is equivalent to root access on the host. Automic Vault
cannot narrow that authority by wrapping the Docker CLI because any process can
connect to the daemon socket directly. Remove unnecessary group membership and
restrict the socket at the operating-system boundary.

[Open an issue to discuss a safer integration](https://github.com/automic-vault/automic-vault/issues).
