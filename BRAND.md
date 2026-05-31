# Automic Vault Brand

## Core Position

Stop agents reading dev-tool secrets.

`brew install` is the common path in. The real scope is broader: CLIs, SDKs,
package managers, MCP servers, and local automation leave credentials and
authority in places that made sense when a trusted human used the machine.
Agents and malware change that risk.

Automic Vault scans Homebrew and other dev tools for plaintext secrets, hardens
supported packages so credentials leave easy-read files, and asks before
commands use secrets.

## One-Line Promise

Stop agents reading dev-tool secrets.

## Audience

Primary readers are developers and founders who use Homebrew, GitHub CLI, AWS
CLI, npm, Python tools, MCP servers, and AI coding agents on a Mac. They are
comfortable with terminals and package managers. They may not think of
themselves as security people, but they understand that an agent with filesystem
and shell access is not the same as a human at a keyboard.

Secondary readers are security-conscious teams and isotope contributors who need
concrete package, path, credential, and approval semantics.

## Narrative

1. Start with the familiar behavior: `brew install`.
2. Show the hidden risk: dev tools store tokens and credentials in readable
   files such as `~/.netrc`, `.env`, `.npmrc`, `~/.aws/credentials`, GitHub CLI
   config, and MCP config.
3. Explain why the risk changed: agents and malware can read files and run tools
   without human memory or judgment.
4. Show what the product does: scan the machine, harden the package, gate the
   command, keep watching.
5. End with the operating mode: install Automic Vault, harden what it finds, and
   leave it running for hazard notifications.

## Message Pillars

- Scan Homebrew and other dev tools for readable secrets.
- Harden supported packages so secrets leave easy-read files.
- Gate commands before they use credentials.
- Keep watching new installs, stale tools, and local config for hazards.

## Voice

Direct, local, concrete. Use package names, paths, commands, and credential
states. Avoid vague AI-safety phrasing.

Good:

- "curl can read `~/.netrc`."
- "Agent wants to run `npm publish`. Approve this command?"
- "Vault shows the package, file path, and reason."
- "Keep the command. Remove the easy-read secret."

Avoid:

- Generic "AI safety" claims.
- Cloud dashboard language.
- Crypto, wallet, token, coin, or exchange metaphors.
- Stock hacker drama.
- Huge statistics without a concrete source.
- Prompt-only safety positioning.

## Visual Direction

The public site should feel like a dark local control room, not a SaaS wrapper.
Show the actual boundary: package rows, hazard counts, file paths, Homebrew
formula pages, approval prompts, and command examples.

Use the app screenshot story wherever possible:

- Left: Security Alerts count.
- Middle: package hazard and plaintext credential detail.
- Right: the familiar Homebrew formula context.

Red means attention or danger in product screenshots. Green means hardened or
approved. Beige carries brand text. Keep color semantic, not decorative.

## CTA Language

Primary:

- Download Automic Vault

Secondary:

- Run the scanner
- Read docs
- View source

Avoid "Get started" and "Learn more" when a more concrete action is available.

## SEO Phrases

- secure Homebrew packages
- AI agent secret scanner
- secrets manager for AI agents
- secure dev tools on macOS
- stop AI agents reading `.env` files
- command approval gates for AI agents
- local developer credential protection
