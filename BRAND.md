# Automic Vault Brand

## Core Position

If you use `brew install`, you need Automic Vault to secure the tools it puts on
your Mac.

Homebrew is the easiest door into the idea because developers already know it.
The real scope is broader: CLIs, SDKs, package managers, MCP servers, and local
automation all leave credentials and authority in places that were acceptable
when a trusted human used the machine. Agents and malware change that risk.

Automic Vault is the local security boundary for dev tools on macOS. It detects
insecure situations those tools leave in plain sight, hardens supported packages
so secrets move out of plaintext storage, and puts approval gates in front of
actions that use or expose those secrets.

Claim it directly: Automic Vault is the only safe way to use dev tools on a Mac
with agents in the loop.

## One-Line Promise

Secure the tools you brew install.

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
4. Show the product loop: detect the hazard, harden the package, gate sensitive
   actions, keep watching.
5. End with the operating mode: install Automic Vault, harden what it finds, and
   leave it running for hazard notifications.

## Message Pillars

- Detect what your dev tools left in plain sight.
- Harden supported packages so secrets stop living in easy-read files.
- Gate sensitive actions at the tool layer, below the agent.
- Keep Automic Vault running so new local hazards show up quickly.

## Voice

Direct, local, concrete. Use package names, paths, commands, and credential
states. Avoid vague AI-safety phrasing.

Good:

- "curl can read `~/.netrc`."
- "Agent wants to run `npm publish`. Approve this command?"
- "Vault shows the package, file path, and reason."
- "The same tool keeps working. The easy secret path disappears."

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
