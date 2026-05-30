---
name: Automic Vault
description: Local macOS security boundary for AI agent tools, packages, secrets, and approvals.
colors:
  nuclear-black: "#0A0D10"
  void-black: "#030506"
  panel: "#12191D"
  panel-strong: "#172126"
  cold-steel: "#3A4A52"
  fallout-beige: "#D6C7A1"
  muted-beige: "#B89B73"
  iron-red: "#D83A2F"
  radar-amber: "#FFB347"
  terminal-green: "#6BFFB0"
  app-green: "#1ADB94"
  app-blue: "#8CABD1"
  app-cyan: "#1A85FF"
  danger-red: "#FF2E38"
typography:
  display:
    fontFamily: "Geist, system-ui, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif"
    fontSize: "12.8rem"
    fontWeight: 800
    lineHeight: 0.78
    letterSpacing: "0"
  campaign-display:
    fontFamily: "Barlow Condensed, Arial Narrow, Impact, sans-serif"
    fontSize: "clamp(3.45rem, 7vw, 7.4rem)"
    fontWeight: 800
    lineHeight: 0.9
    letterSpacing: "0"
  app-title:
    fontFamily: "-apple-system, BlinkMacSystemFont, SF Pro Text, system-ui, sans-serif"
    fontSize: "20px"
    fontWeight: 600
    lineHeight: 1.2
    letterSpacing: "0"
  body:
    fontFamily: "Geist, system-ui, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif"
    fontSize: "16px"
    fontWeight: 400
    lineHeight: 1.58
    letterSpacing: "0"
  app-body:
    fontFamily: "-apple-system, BlinkMacSystemFont, SF Pro Text, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.45
    letterSpacing: "0"
  mono-label:
    fontFamily: "Geist Mono, SFMono-Regular, Consolas, monospace"
    fontSize: "0.79rem"
    fontWeight: 700
    lineHeight: 1.45
    letterSpacing: "0"
rounded:
  xs: "2px"
  sm: "5px"
  md: "8px"
  lg: "12px"
  pill: "999px"
spacing:
  xxs: "4px"
  xs: "8px"
  sm: "12px"
  md: "18px"
  lg: "24px"
  xl: "44px"
  xxl: "64px"
components:
  button-primary:
    backgroundColor: "{colors.iron-red}"
    textColor: "{colors.nuclear-black}"
    typography: "{typography.mono-label}"
    rounded: "{rounded.md}"
    padding: "12px 18px"
  button-secondary:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.fallout-beige}"
    typography: "{typography.mono-label}"
    rounded: "{rounded.md}"
    padding: "12px 18px"
  app-action-button:
    backgroundColor: "{colors.panel-strong}"
    textColor: "{colors.fallout-beige}"
    typography: "{typography.app-body}"
    rounded: "{rounded.xs}"
    padding: "8px 14px"
  status-success:
    backgroundColor: "{colors.app-green}"
    textColor: "{colors.nuclear-black}"
    rounded: "{rounded.pill}"
    padding: "3px 8px"
  status-warning:
    backgroundColor: "{colors.radar-amber}"
    textColor: "{colors.nuclear-black}"
    rounded: "{rounded.pill}"
    padding: "3px 8px"
  status-danger:
    backgroundColor: "{colors.danger-red}"
    textColor: "{colors.nuclear-black}"
    rounded: "{rounded.pill}"
    padding: "3px 8px"
---

# Design System: Automic Vault

## 1. Overview

**Creative North Star: "The Hardened Mission Console"**

Automic Vault should feel like a local control room for high-trust developer operations: fixed paths, visible requests, package dossiers, command traces, and approval decisions. The working scene is a developer at a Mac during an autonomous agent run, often in a dim workstation context, checking what a local tool wants to do before credentials or mutation rights cross the boundary. That scene earns a dark, high-contrast interface, dense columns, precise labels, and restrained status color.

The system has two expressions. The native macOS app is the operational surface: compact, glass-backed, list-heavy, and built around package inspection. The public website and docs are the brand surface: more poster-like, with Cold War control-room references, radar lines, industrial red, aged beige, and blunt explanatory copy. Both expressions should share the same promise: software under control.

Frontmatter colors are hex for Stitch compatibility. Existing and new CSS can express the same tokens in OKLCH where supported, as the public site already does. Do not create a parallel palette with different roles.

**Key Characteristics:**

- Dark control-room surfaces with tinted neutrals, not pure black.
- Red is the brand action color on public pages, not a generic error color.
- Green, amber, blue, cyan, and red carry operational state in product UI.
- Mono labels and fixed columns make package and command context scannable.
- Texture, scanlines, radar grids, and Liquid Glass are used only where they support the product's controlled-machine character.

## 2. Colors

The palette combines Cold War industrial branding with native macOS operational state. Use color deliberately; a user should be able to tell whether color means brand, state, or structure.

### Primary

- **Nuclear Black** (#0A0D10): Main website, docs, and SEO background. Use instead of pure black.
- **Iron Red** (#D83A2F): Primary brand action color on marketing, docs, download calls to action, and critical brand marks.
- **Fallout Beige** (#D6C7A1): Main brand ink for public pages, display headings, wordmarks, and high-emphasis copy.

### Secondary

- **Terminal Green** (#6BFFB0): Success, trust, command output, safe package states, and positive status in brand collateral.
- **Radar Amber** (#FFB347): Warnings, attention, caution, focus outlines, and secondary action emphasis.
- **Cold Steel** (#3A4A52): Muted panels, border-adjacent neutrals, and supporting industrial structure.

### Tertiary

- **App Green** (#1ADB94): Native app success and protected-package accent.
- **App Blue** (#8CABD1): Native app informational and secondary package metadata accent.
- **App Cyan** (#1A85FF): Active links, external references, or live system state where blue is already established.
- **Danger Red** (#FF2E38): Native app vulnerable, exposed, destructive, or denial-related states.

### Neutral

- **Void Black** (#030506): Deepest edge and vignette color for page backgrounds.
- **Panel** (#12191D): Standard public page panel background.
- **Panel Strong** (#172126): Raised public page panel, code-adjacent surface, and strong contrast container.
- **Muted Beige** (#B89B73): Public page secondary text, navigation, metadata, and long-form support copy.

### Named Rules

**The State Is Sacred Rule.** Green, amber, red, blue, and cyan must map to a real package, secret, approval, or system state in product UI. Do not spend them on decoration inside task surfaces.

**The Red Has Two Jobs Rule.** On public brand pages, red is the primary action and identity color. In native app workflows, red is danger. Keep those contexts separate.

**The No Pure Extremes Rule.** Avoid pure black and pure white in new design work. Use tinted blacks, aged light neutrals, and opacity-adjusted text tokens instead.

## 3. Typography

**Display Font:** Geist for the main homepage, with system sans fallback.
**Campaign Display Font:** Barlow Condensed for SEO, docs, and campaign-like pages.
**Body Font:** Geist on web, SF Pro/system on native macOS.
**Label/Mono Font:** Geist Mono on web, SF Mono/SFMono-Regular in native app and CLI-adjacent UI.

**Character:** Typography should feel like a package console with poster confidence. Labels are terse and technical. Display type can be large and compressed on public surfaces, but product controls, package rows, and approval prompts stay native, legible, and quiet.

### Hierarchy

- **Display** (800, 12.8rem, 0.78): Homepage brand title and rare public hero usage. Keep it uppercase and spatially dominant.
- **Campaign Display** (800, clamp(3.45rem, 7vw, 7.4rem), 0.9): SEO and docs hero headlines that need more industrial urgency.
- **Headline** (700 to 800, 1.95rem to 6.25rem, 0.9 to 1.06): Public page section titles, final calls to action, and ranked feature stories.
- **App Title** (semibold, 20px to 22px, 1.2): Native app dossier names, overlay titles, update flows, and pack windows.
- **App Body** (regular to medium, 12px to 14px, 1.35 to 1.55): Package descriptions, metadata, rows, prompts, and helper text.
- **Body** (regular, 16px, 1.58): Public page prose and docs content. Keep long copy within 65 to 75 characters where possible.
- **Mono Label** (semibold to bold, 0.74rem to 0.86rem, uppercase): Navigation, section labels, package source chips, metadata keys, command snippets, counts, and status tags.

### Named Rules

**The Console Legibility Rule.** If the user is approving, installing, tracing, or comparing package state, use native/system or mono typography at stable sizes. Display type belongs to brand surfaces, not decision controls.

**The Path Readability Rule.** Commands, package names, executable paths, root paths, and secret names use mono treatment or monospaced native labels. Truncate in the middle when the end of a path is important.

## 4. Elevation

Automic Vault uses tonal layering first and shadows second. Public pages use framed shells, hairline borders, subtle inset shadows, scanline overlays, and deep drop shadows to create a physical command-room object. Native macOS surfaces use Liquid Glass material, translucent tints, hairlines, and selected fills. Approval prompts and package rows remain mostly flat so dense information does not fight decorative depth.

### Shadow Vocabulary

- **Public Shell** (`box-shadow: 0 34px 90px oklch(6% 0.006 255 / 0.48)`): Main homepage shell, used once per page-level frame.
- **Docs Shell** (`box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.72) inset, 0 28px 80px -48px rgba(216, 58, 47, 0.34)`): Documentation and SEO shells with industrial red ambience.
- **Toast / Suggestion** (`box-shadow: 0 16px 40px oklch(8% 0.03 260 / 0.28)`): Floating language and notification surfaces.
- **Native Status Glow** (`shadow radius: 5` with semantic color at low opacity): Badges and high-risk indicators only.

### Named Rules

**The Flat At Rest Rule.** Product rows, panels, approvals, and controls are flat at rest. Use borders, tints, and selected fills before adding lift.

**The Liquid Glass Boundary Rule.** Liquid Glass belongs in native macOS windows as an operating-system material. Do not turn every panel into a decorative glass card.

## 5. Components

### Buttons

Buttons are command controls, not decorative pills.

- **Shape:** Public buttons use 8px corners, native control chrome can use 2px to match the current AppKit utility style, and SwiftUI glass controls use 8px to 10px where already established.
- **Primary:** Public primary buttons use Iron Red background with Nuclear Black text. Native app approvals and package actions should use semantic tinting through macOS controls and only use red for danger.
- **Hover / Focus:** Public hover may translate by 1px to 2px with a 160ms transition. Focus uses a visible amber or semantic outline. Native app focus follows macOS conventions.
- **Secondary / Ghost / Tertiary:** Secondary buttons are bordered or panel-filled with beige text. Text buttons are quiet and should become higher contrast only on hover.

### Chips

Chips are compact state or source labels.

- **Style:** Use mono uppercase labels, 999px radius, tight vertical padding, and a 1px border or translucent semantic fill.
- **State:** Package source chips, "BREW", "CASK", isotope labels, status badges, and scan result labels must include text, not only color.
- **Critical Counts:** Counts can use stronger red or amber borders, but must stay readable in dense sidebars.

### Cards / Containers

Containers should feel like instrument panels, not generic cards.

- **Corner Style:** 8px for panels and language suggestions, 12px for the large homepage shell, 5px to 6px for compact native controls.
- **Background:** Use Panel, Panel Strong, native translucent tints, or low-opacity white fills over dark materials.
- **Shadow Strategy:** Use the elevation rules above. Most repeated content uses borders and tonal shifts, not shadows.
- **Border:** Hairlines are essential. Prefer full 1px borders, separators, and grid lines. Do not add colored side-stripe borders.
- **Internal Padding:** Compact product panels use 8px to 18px. Public sections can use 44px to 64px at desktop widths.

### Inputs / Fields

Fields should preserve the command-console feel.

- **Style:** Dark filled or transparent backgrounds, 1px border, small radius, mono where the value is command-like.
- **Focus:** Use amber on public pages and semantic/native focus rings in app surfaces.
- **Error / Disabled:** Disabled controls drop opacity and keep layout stable. Error states use text plus danger color, never color alone.

### Navigation

Navigation should stay predictable and scannable.

- **Website:** Sticky masthead, compact brand mark or wordmark, mono uppercase nav, 44px minimum target height, and simple underline or color hover states.
- **Native App:** Column-based shell with sidebar, package list, dossier panel, and external surface. Sidebar rows are 32px high with SF Symbols, counts, and selected fills.
- **Docs:** Keep top navigation narrow, then let content density come from headings, code blocks, and anchors.

### Package Rows

Package rows are the heart of the product UI.

- **Style:** Dense, fixed-height rows with package name, version, source, badges, and status visible without opening a detail view.
- **Selection:** Selected rows use translucent fill and primary text, not large color floods.
- **Security:** Vulnerable or protected states must include lock, warning, count, or label treatment in addition to color.

### Approval Panels

Approval panels are decision instruments.

- **Style:** Fixed-size panels with clear sections for command, requester, environment, executable, script, secret names, and authorization options.
- **Hierarchy:** The requested command and requested secret/action are highest priority. Parent process, working directory, and environment are supporting evidence.
- **Actions:** Approve and deny must be visually distinct and keyboard reachable. "Always allow" must never look like the default path unless policy truly permits it.

## 6. Do's and Don'ts

**Do:**

- Show the concrete boundary: package root, executable path, command, process, secret, detector, or requested action.
- Use dark tinted surfaces and aged neutrals instead of pure black and pure white.
- Keep product UI dense, columnar, and stable. The user is comparing state, not browsing decoration.
- Use OKLCH for new CSS tokens when practical, matching the existing public site palette.
- Pair every security state color with text, iconography, or both.
- Keep motion short, state-driven, and transform-based. Hover, selection, refresh, progress, and copy feedback are valid.
- Preserve macOS-native behaviors in the app: Touch ID, Keychain expectations, keyboard focus, VoiceOver labels, and standard controls.

**Don't:**

- Do not make Automic Vault look like a crypto product, token wallet, cloud SaaS dashboard, or generic AI wrapper.
- Do not hide the command, package, requester, or secret involved in an approval.
- Do not use gradient text.
- Do not use colored side-stripe borders for callouts, cards, alerts, or list items. Use full borders, icons, semantic backgrounds, or inline labels.
- Do not use identical icon-card grids as a default explanatory pattern.
- Do not use glass as generic decoration. Liquid Glass is for native macOS surfaces where the OS material is doing real structural work.
- Do not let red mean both danger and primary action inside the same product workflow.
- Do not rely on color alone for package security, approval, vulnerability, or update state.
