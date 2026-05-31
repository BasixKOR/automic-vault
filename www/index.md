# Automic Vault

If you use `brew install`, you need Automic Vault around the tools it puts on
your Mac. Automic Vault finds plaintext secrets and exposed dev-tool state, then
hardens supported packages so credentials leave easy-read files and get served
only at runtime.

Homebrew is the familiar case, but the same problem shows up across CLIs, SDKs,
package managers, and MCP servers. Automic Vault keeps watching after setup and
reports new hazards before an agent or malware can treat them as normal machine
state.
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

- Find plaintext credentials in local dev-tool files such as `.env`, `.netrc`,
  `.npmrc`, GitHub CLI config, AWS credentials, and MCP config.
- Harden supported Homebrew, npm, and PyPI tools so secrets are exposed only to
  approved executables at runtime.
- Ask before sensitive commands publish packages, change cloud state, reveal
  tokens, or use protected secrets.
- Keep watching for new hazards while Automic Vault runs.
- Trace shell installers before an agent or developer runs them.
