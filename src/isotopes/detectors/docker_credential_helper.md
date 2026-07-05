# docker-credential-helper Detector

Reports when:
- Docker config uses an ambient Docker credential helper.

## Detection Caveats

- Scans `DOCKER_CONFIG/config.json` when `DOCKER_CONFIG` is set, otherwise `~/.docker/config.json`.
- Recognizes packaged helpers named `osxkeychain`, `secretservice`, `pass`, and `wincred`.
