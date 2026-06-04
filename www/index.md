# Automic Vault

Secure the tools you brew install.

A new kind of package manager for a new kind of threat model

Automic Vault secures Homebrew tools, CLI secrets, and command approval gates
locally on your Mac before AI agents use them.

The same risk shows up across CLIs, SDKs, package managers, and MCP servers:
tokens sit in files an agent or malware can read. Automic Vault keeps watching
after setup, so new installs and stale tool config do not stay quiet.

## Secret Exposure Examples

- `gh auth token` prints a GitHub bearer token that an agent, script, or
  compromised tool can copy into API calls.
- `cat ~/.aws/credentials` turns a readable dotfile into cloud account access.
- `printf 'protocol=https\nhost=github.com\n\n' | git credential fill` asks the
  local credential helper to reveal repository credentials, even when there is
  no obvious token file.

## Founder Context

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
- [Security Methods and Gaps](/security/whitepaper/): candid white paper on controls, caveats, holes, successes, and hardening work.
- [Blog](/blog/): notes on agent tooling, package hardening, and local developer security.
- [Agent Pack](/blog/agent-pack/): agent CLIs and coding assistants in one installable pack.
- [UNIX++ Pack](/blog/unix-plus-plus/): modern command line replacements and operators in one installable pack.
- [The Agentic Toolkit](/blog/agentic-toolkit/): all the tools agents need in one installable pack.
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

## Blog

- [Agent Pack](/blog/agent-pack/): 10 agent CLIs and coding assistants for
  terminal-native planning, editing, review, model routing, and usage
  inspection.
- [UNIX++ Pack](/blog/unix-plus-plus/): 23 modern command line replacements and
  operators for search, file inspection, process monitoring, data wrangling,
  HTTP/DNS work, and file watching.
- [The Agentic Toolkit](/blog/agentic-toolkit/): 26 Homebrew packages for
  media processing, image manipulation, runtimes, search, shell, build, OCR,
  metadata, and document conversion.
- [Preventing the Nx Console extension compromise](/blog/prevent-nx-console-vscode-compromise/): Nx Console 18.95.0 stole local developer credentials from editor sessions. Automic Vault would have stopped the local secret access.
- [Preventing the GitHub employee device breach](/blog/prevent-github-vscode-extension-breach/): A poisoned VS Code extension reached a GitHub employee device. Automic Vault would have prevented local credentials from becoming repository access.
- [Preventing the durabletask PyPI compromise](/blog/prevent-durabletask-pypi-compromise/): Malicious durabletask PyPI releases fetched rope.pyz and stole cloud and developer secrets. Automic Vault would have blocked the local theft path.
- [Preventing the TanStack npm credential theft](/blog/prevent-tanstack-npm-compromise/): TanStack packages were poisoned through trusted publishing, then stole local secrets. Automic Vault would have prevented the endpoint theft.
- [Preventing the node-ipc npm backdoor](/blog/prevent-node-ipc-npm-backdoor/): The node-ipc backdoor ran on module load and exfiltrated secrets over DNS. Automic Vault would have prevented useful credential theft.
- [Preventing the Bitwarden CLI npm compromise](/blog/prevent-bitwarden-cli-npm-compromise/): The compromised Bitwarden CLI npm package used install-time code to steal developer secrets. Automic Vault would have stopped the local theft.
- [Preventing the LiteLLM PyPI compromise](/blog/prevent-litellm-pypi-compromise/): LiteLLM 1.82.7 and 1.82.8 stole local credentials. Automic Vault would have prevented the workstation credential theft phase.
