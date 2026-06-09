# Product

## Register

product

## Users

Automic Vault is for developers, founders, and security-conscious teams who run AI coding agents on local Macs and need those agents to use real command-line tools without inheriting every credential and writable package path on the machine.

Primary users are comfortable with terminals, package managers, GitHub CLI, AWS CLI, MCP servers, and local automation. They may not be security specialists, but they understand that an autonomous agent with filesystem and shell access changes the risk model. They use Automic Vault during setup, before agent runs, while inspecting package or secret exposure, and at the moment a tool requests a sensitive action.

Secondary users are isotope contributors and package metadata maintainers. They need predictable rules for package manifests, approval gates, detectors, and local hazard reporting.

## Product Purpose

Automic Vault is a local macOS security layer for AI coding agents: package manager, secrets manager, and execution control plane. It keeps developer secrets out of plaintext files and model context, installs agent-used tools under controlled roots, surfaces risky local package and credential states, and adds human approval gates where sensitive tool actions execute.

Success looks like this: a developer can give an agent useful local tools while secrets remain in protected storage, privileged mutations stay explicit, dangerous commands carry enough context to approve or deny, and package state is inspectable from both the CLI and native app.

Automic Vault is not a general enterprise vault replacement, cloud console, or agent wrapper. It is the local runtime boundary beneath the agent.

## Messaging Contract

The canonical story lives in `BRAND.md`. Product copy should preserve this
shape:

1. Start from the familiar developer action: installing and running real tools.
2. Name the changed threat model: agents and compromised tools can read files
   and run commands without human judgment.
3. Show the concrete boundary: package root, executable path, requester,
   command, secret name, detector finding, or approval target.
4. Make the user the operator. Automic Vault is the control layer, not the hero.
5. End on a concrete action: scan, harden, save, inject, contain, approve, or
   deny.

The product should not present itself as generic AI safety, a cloud policy
console, or a promise that agents are safe. The claim is narrower and stronger:
useful developer tools can keep working while secrets, package mutations, and
risky commands stop being ambient authority.

## Brand Personality

Controlled, direct, technically credible.

The voice is calm but not soft. It should sound like a serious local tool built by someone who has operated package managers, credential helpers, and Mac apps in production. It speaks in concrete boundaries: paths, commands, packages, secrets, approvals, and risks. It does not promise magic, autonomy, or vague "AI safety." It explains what is controlled and where the control happens.

Brand essence:

- Conservative by design.
- Security is the default.
- Predictable paths.
- Execute nothing blind.

## Anti-references

Avoid generic AI-agent SaaS: pastel gradients, floating cards, vague automation metaphors, sparkle icons, chat bubbles, and "ten times faster" copy.

Avoid crypto and token aesthetics. Automic Vault is explicitly not affiliated with cryptocurrency or tokens, so never use coin, chain, wallet, token, exchange, or speculative-finance visual language.

Avoid enterprise security theater: stock hacker imagery, lock-and-shield wallpaper, inscrutable dashboards, glossy SOC drama, and red-alert noise that makes every state feel equally urgent.

Avoid black-box automation. The product should never feel like it hides the command, package, path, requester, or credential involved in a decision.

Avoid decorative UI that weakens trust: gratuitous glass, huge metric hero blocks, identical icon-card grids, colored side-stripe callouts, and gradient text.

## Design Principles

1. Put the boundary on screen.
   Every important surface should make the controlled layer visible: package roots, executable paths, secret names, parent processes, requester identity, version state, or approval target. The user should know what the system is protecting before being asked to act.

2. Prefer inspection before action.
   Install, update, remove, migrate, inject, approve, and deny flows should show enough context to make a deliberate decision. Fast actions are welcome, but hidden actions are not.

3. Use density with order.
   Package catalogs, dossiers, approval requests, and traces need high information density. The design should feel like a precise console, not a sparse brochure. Density must be made scannable with columns, hairlines, labels, stable spacing, and clear state colors.

4. Treat risk as semantic, not decorative.
   Green, amber, red, and blue indicate real system meaning: safe, warning, danger, information, update, or package source. Color should never be used as decoration where it could be confused with status.

5. Keep local-first trust.
   The product should feel native to macOS and honest about local state. Touch ID, Keychain-backed storage, fixed roots, helper authorization, and visible progress are part of the trust story.

## Accessibility & Inclusion

Target WCAG 2.2 AA for public web surfaces and native macOS accessibility conventions for app surfaces. Core workflows must remain usable with keyboard navigation, VoiceOver labels, visible focus indicators, and sufficient contrast in dark mode.

Do not rely on color alone for package or security state. Pair status colors with labels, icons, counts, or text. Approval prompts must make the command, target, requester, and secret context readable at normal macOS text sizes.

Respect reduced motion. Motion should communicate state changes such as loading, refresh, hover, reveal, copy success, approval result, or progress. Avoid motion that only performs atmosphere.
