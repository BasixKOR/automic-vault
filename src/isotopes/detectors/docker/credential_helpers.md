# docker-credential-helpers Detector

## Trigger Conditions

- Docker config uses an ambient credential store.
- Docker config uses an ambient per-registry credential helper.

## Sensitive Files

- `$DOCKER_CONFIG/config.json`
- `~/.docker/config.json`

## Why This is not Yet Hardened

Docker credential helpers are ambient: any process running as the user can ask
the configured helper for registry credentials. A hardener needs an
approval-aware helper that preserves Docker's credential-helper protocol and
registry routing. Rewriting the existing helper configuration is not enough.

[Open an issue to discuss a safer integration](https://github.com/automic-vault/automic-vault/issues).
