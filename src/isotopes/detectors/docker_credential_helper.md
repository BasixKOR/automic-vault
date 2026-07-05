# Docker Credential Helper Radioisotope

Detect-only coverage for Docker credential-helper configuration.

Docker credential helpers are a credential-store boundary, not a normal CLI
secret file. This radioisotope reports Docker config that uses the packaged
helpers without changing Docker's helper settings.
