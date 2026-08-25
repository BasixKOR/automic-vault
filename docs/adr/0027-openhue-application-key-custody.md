# ADR 0027: OpenHue application-key custody

Status: Accepted

## Context

OpenHue CLI is a single Go executable that stores a Hue bridge application key
in `$XDG_CONFIG_HOME/openhue/config.yaml` or `~/.openhue/config.yaml`. It has no
credential-helper interface. Its setup and manual-config commands update the
same file used by authenticated commands, so an environment wrapper would still
need a temporary plaintext config.

## Decision

The `automic-vault/openhue-cli` fork publishes a pinned OpenHue CLI Isotope
signed with Developer ID, Hardened Runtime, timestamping, and no entitlements.
The signed Isotopes tap pins the exact fork release URL and digest as specified
by [ADR 0029](0029-fork-owned-isotope-releases.md). The upstream patch is limited
to `Config.Load` and `Config.Save`: it reads and writes the application key
through dedicated XPC operations and persists only an `@av` marker.

The approval service binds helper operations to the live signed `openhue`
parent, its complete arguments, the bridge scope, and the exact Secret Name.
Unknown commands fail closed, and application-key flags are redacted from
authorization history.

## Consequences

OpenHue setup and bridge-control workflows continue without a reusable key on
disk. Unsupported YAML, ambiguous fields, unsafe paths, invalid scopes, source
drift, signing drift, and unexpected archive contents stop installation or use.
