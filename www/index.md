# Automic Vault

Automic Vault is a local macOS security layer for AI coding agents. It keeps
developer secrets out of plaintext files and model context, injects approved
credentials only into trusted command-line tools, and adds human approval gates
where those tools run.

Automic Vault includes Nucleus, a package manager for Homebrew, npm, and PyPI
packages that installs under controlled roots. It is free open-source software
under the Apache License 2.0.

## Key Pages

- [Documentation](/docs/) — CLI commands and runtime patterns.
- [Security](/security/) — threat model and disclosure information.
- [Pricing](/pricing/) — free open-source software pricing.
- [security.txt](/.well-known/security.txt) — machine-readable security disclosure policy.
- [llms.txt](/llms.txt) — concise AI system navigation.
- [llms-full.txt](/llms-full.txt) — all checked-in site text in one file.

## Core Use Cases

- Protect API keys, cloud credentials, and GitHub tokens from AI coding agents.
- Replace plaintext `.env`, shell profile, and CLI config secrets.
- Require approval before sensitive commands mutate infrastructure or reveal data.
- Trace shell installers before an agent or developer runs them.
- Install agent-used packages under controlled roots.
