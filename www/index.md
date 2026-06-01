# Automic Vault

The secrets manager for the open source ecosystem.

The same risk shows up across CLIs, SDKs, package managers, and MCP servers:
tokens sit in files an agent or malware can read. Automic Vault keeps watching
after setup, so new installs and stale tool config do not stay quiet.

## Founder Note

> I built Homebrew. It was designed before AI agents existed.
>
> Install with Homebrew. Secure with Automic Vault.
>
> Stop agents, malware, and compromised tools from accessing secrets or
> performing sensitive actions without approval.
>
> - Max Howell, Creator of Homebrew

Automic Vault is free open-source software under the Apache License 2.0.

## Key Pages

- [Documentation](/docs/): CLI commands and runtime patterns.
- [Security](/security/): threat model and disclosure information.
- [Pricing](/pricing/): free open-source software pricing.
- [security.txt](/.well-known/security.txt): machine-readable security disclosure policy.
- [llms.txt](/llms.txt): concise AI system navigation.
- [llms-full.txt](/llms-full.txt): all checked-in site text in one file.

## Core Use Cases

- Find plaintext credentials in local dev-tool files such as `.env`, `.netrc`,
  `.npmrc`, GitHub CLI config, AWS credentials, and MCP config.
- Harden supported Homebrew, npm, and PyPI tools so secrets are exposed only to
  approved executables at runtime.
- Ask before sensitive commands publish packages, change cloud state, reveal
  tokens, or use protected secrets.
- Keep watching new installs, stale tools, and local config for hazards.
- Trace shell installers before an agent or developer runs them.
