# Automic Vault Brew Install Security HyperFrames Rebuild

## Style Block

Source video: `marketing/promo-video/src/BrewInstallSecurity.tsx`, rendered as `marketing/promo-video/out/automic-vault-brew-install-security.mp4`.

Design source: `/Users/mxcl/src/automic-vault/DESIGN.md`.

Brand values to preserve:

- Brand: Automic Vault, a local macOS security boundary for AI agent tools, packages, secrets, and approvals.
- Creative north star: "The Hardened Mission Console."
- Primary palette: Nuclear Black `#0A0D10`, Void Black `#030506`, Panel `#12191D`, Panel Strong `#172126`, Cold Steel `#3A4A52`, Fallout Beige `#D6C7A1`, Muted Beige `#B89B73`, Iron Red `#D83A2F`.
- Operational state palette: Terminal Green `#6BFFB0`, Radar Amber `#FFB347`, App Blue `#8CABD1`, App Cyan `#1A85FF`, Danger Red `#FF2E38`.
- Typography: Campaign Display uses `Barlow Condensed`; body uses `Geist`; labels and command context use `Geist Mono`.
- Shape: public/video panels use 8px to 12px corners; chips use pill radius.

The latest Remotion piece is light, soft, glassy, and notification-led. This rebuild preserves its essence but restages the same product idea as a dark control-room sequence: package tools enter the mission console, risk events are triaged, the command `brew install` becomes the bridge, and the close lands on Automic Vault plus the Homebrew founder proof point.

## Rhythm Declaration

Pattern: `hook-risk-risk-risk-PEAK-CTA`.

Duration: 28.65 seconds at 1920x1080. The first four beats are fast operational alerts. The fifth beat is a slower command-lockup peak. The final beat resolves into the brand.

## Global Rules

- Background is dark across the composition: `#030506` into `#0A0D10`, with panel layers in `#12191D` and `#172126`.
- Do not reuse the Remotion light paper/glass look. Preserve the story, not the implementation.
- Use CSS-only transitions: primary transition is velocity-matched upward blur; accent transition before the command peak is a shutter/block cover. No jump cuts.
- Each scene has 8-10 visual elements: background grid, scanlines, ghost type, top label, main panel, command/status rows, state chips, rules, meters, and small registration labels.
- Every scene element enters; no individual element exits before a transition. Outgoing scene content remains readable until the scene-level transition begins.
- Keep typography readable at video scale: headlines 72px+, body 28px+, labels 18px+.
- Use state colors only where the content represents package, secret, approval, or system state.

## Beat 1: Plain Text Secret Exposure

Concept: The viewer arrives inside a live vault console while an agent-run tool lights up in red. The alert is not decorative: the machine has found a plain-text secret exposure in `gh`.

Mood direction: Cold War operations desk; scanline grit; a terse terminal incident card, not a generic SaaS notification.

Depth layers: BG dark grid and ghost `SECRET SCAN`; MG risk panel with title and command row; FG red state chip, route marker, package mist labels, and a thin diagnostic meter.

Animation choreography: grid drifts, ghost text breathes, panel drops into focus, label scans on, title stamps upward, command row types into place, red meter fills, peripheral tools float.

Transition out: velocity-matched upward blur, 0.55s, `power2.in` out and `power3.out` in.

## Beat 2: Cloud Key Left Readable

Concept: The second alert pivots to cloud credentials. The system marks `awscli` as readable and overlays an amber caution state over the same local boundary.

Mood direction: Caution board, amber lamps, dense but legible package telemetry.

Depth layers: BG ghost `CREDENTIAL HARDENING`; MG amber panel; FG source chip, path-like command snippet, exposure count, and amber rule.

Animation choreography: amber line draws, source chip snaps, title slides from the right, command snippet rises, small telemetry counters count into place.

Transition out: same upward blur, 0.55s, consistent primary motion.

## Beat 3: Postinstall Wants a Token

Concept: Installation risk becomes active: `node` wants token access through a postinstall script. This beat feels more mechanical and blue/cyan because the threat is a script flow.

Mood direction: Mechanical inspection gate with blue active-system highlights.

Depth layers: BG ghost `INSTALL GUARD`; MG split panel with command inspection; FG script badge, cyan route line, token-lock indicator, blue rail.

Animation choreography: split rail sweeps in, `node` badge locks, title cascades, script blocks stack, token indicator pulses once.

Transition out: upward blur, 0.55s.

## Beat 4: Agent Command Needs Approval

Concept: The sequence reaches the local human boundary. `gemini-cli` is not denied; it is paused for approval before a sensitive action runs.

Mood direction: Approval checkpoint, calm green trust signal with strict framing.

Depth layers: BG ghost `APPROVAL GATE`; MG green approval dossier; FG allow/deny affordance chips, local-only tag, command lock icon substitute as text glyph, state meter.

Animation choreography: dossier slides up, green beacon blooms, title settles, approval chips cascade, state meter fills from red to green.

Transition out: shutter/block cover, 0.65s, red blocks wipe into the command peak.

## Beat 5: Secure The Tools You Brew Install

Concept: The alerts collapse into the plain promise. The command `brew install` becomes the physical object being secured by Automic Vault.

Mood direction: Hero command lockup, poster confidence, less telemetry and more brand punch.

Depth layers: BG giant ghost `BREW INSTALL`; MG stacked headline; FG command slab, red clamp lines, package-source chips, scan dots.

Animation choreography: words stamp in one by one, command slab slams into place, clamp lines draw inward, chips orbit lightly, red beam sweeps under the command.

Transition out: gentle focus-pull blur, 0.65s into brand close.

## Beat 6: Automic Vault Close

Concept: The machine resolves to the brand. The proof point "From the creator of Homebrew" appears before the logo, then the URL anchors the action.

Mood direction: Mission complete title card: quiet, precise, credible.

Depth layers: BG subdued grid and scanlines; MG icon plus wordmark text; FG proof label, URL capsule, red terminal cursor, small local-boundary labels.

Animation choreography: proof label types on, icon scales in, Automic/Vault wordmark locks into two-line mono, URL slides up, terminal cursor pulses, whole scene fades to black at the end.

Transition out: final fade to black only.

## Recurring Motifs

- Thin beige rules and registration marks.
- Tool labels floating at the frame edge: `gh`, `awscli`, `node`, `gemini-cli`, `docker`, `git`.
- Ghost typography behind every beat.
- A red command clamp that becomes stronger as the video approaches `brew install`.

## Negative Prompt

- No light paper background, glass-card copy, or direct Remotion port.
- No generic blue-purple SaaS gradients.
- No centered empty slide layouts.
- No pure black or pure white.
- No invented palette outside `DESIGN.md`.
- No unsupported schema changes or unrelated repo edits.
