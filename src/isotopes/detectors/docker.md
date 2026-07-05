# Docker Radioisotope Detector

This detector reports Docker registry credential configurations that expose
credentials to agents or other local processes. It also reports local Docker
daemon access that gives a non-root user root-equivalent host access.

Detected hazards:

- Inline `auth`, `identitytoken`, or `identityToken` entries in
  `~/.docker/config.json` or `$DOCKER_CONFIG/config.json`
- Legacy `~/.dockercfg` registry credentials
- `credsStore` or `credHelpers` entries that use ambient Docker credential
  helpers such as `osxkeychain` or `desktop`
- Docker Desktop installs that do not configure an Automic Vault-backed default
  credential helper
- Membership in the Unix `docker` group
- Writable local Docker daemon sockets such as `/var/run/docker.sock`

On Linux, access to the Docker daemon is root-equivalent. A process that can
control Docker can start a container as root and bind-mount privileged host
paths writable, for example mounting `/etc` and overwriting host configuration
from inside the container.

This radioisotope is detect-only. It does not wrap Docker, because Docker
Desktop owns the usual CLI symlink locations and can replace wrappers during
install, update, or settings changes.
