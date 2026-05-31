# Automic Vault

If you use `brew install`, you need a local boundary around the tools it puts on
your Mac. Automic Vault finds plaintext secrets and insecure dev-tool state,
then hardens supported packages so credentials move out of easy-read files and
into protected runtime access.

Homebrew is the front door, but the same problem shows up across CLIs, SDKs,
package managers, and MCP servers. Automic Vault keeps watching after setup and
reports new hazards before an agent or malware can treat them as ambient state.
It is the only safe way to keep using dev tools on a Mac with agents in the
loop.

Automic Vault is free open-source software under the Apache License 2.0.

## Key Pages

- [Documentation](/docs/): CLI commands and runtime patterns.
- [Security](/security/): threat model and disclosure information.
- [Pricing](/pricing/): free open-source software pricing.
- [security.txt](/.well-known/security.txt): machine-readable security disclosure policy.
- [llms.txt](/llms.txt): concise AI system navigation.
- [llms-full.txt](/llms-full.txt): all checked-in site text in one file.

## Core Use Cases

- Detect plaintext credentials in local dev-tool files such as `.env`, `.netrc`,
  `.npmrc`, GitHub CLI config, AWS credentials, and MCP config.
- Harden supported Homebrew, npm, and PyPI tools so secrets are exposed only to
  approved executables at runtime.
- Require approval before sensitive commands mutate infrastructure, publish
  packages, reveal tokens, or use protected secrets.
- Keep hazard notifications visible while Automic Vault runs.
- Trace shell installers before an agent or developer runs them.
