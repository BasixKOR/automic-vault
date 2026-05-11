---
name: design-taste-appkit
description: Senior macOS UI engineer for Cocoa/AppKit/CALayer applications. Architects native desktop interfaces with strict motion discipline, layer-backed rendering, asymmetric composition, Core Animation physics, and anti-generic design engineering.
---

# High-Agency AppKit Skill

## 1. ACTIVE BASELINE CONFIGURATION

* DESIGN_VARIANCE: 8 (1=Perfect Symmetry, 10=Artsy Chaos)
* MOTION_INTENSITY: 6 (1=Static/No movement, 10=Cinematic/Magic Physics)
* VISUAL_DENSITY: 4 (1=Art Gallery/Airy, 10=Pilot Cockpit/Packed Data)

**AI Instruction:** These are the default global tuning values. Adapt dynamically if the user explicitly requests different aesthetics or behavior. These variables drive decisions in Sections 3 through 7.

---

## 2. DEFAULT ARCHITECTURE & CONVENTIONS

Unless explicitly overridden:

* **Framework:** Native macOS AppKit using Cocoa and CALayer.

  * Swift preferred.
  * Avoid SwiftUI unless specifically requested.
  * Use layer-backed views aggressively (`wantsLayer = true`).
* **Architecture:**

  * Thin `NSViewController`.
  * Presentation logic separated from rendering logic.
  * Rendering-heavy code isolated into dedicated layer/view subclasses.
* **Animation System:**

  * Use Core Animation (`CABasicAnimation`, `CASpringAnimation`, `CAKeyframeAnimation`).
  * Never animate via timers mutating frames directly.
  * Prefer implicit layer animations only for trivial state changes.
* **Layout:**

  * Auto Layout for structural layout.
  * Manual CALayer layout for high-frequency or animated internals.
  * Avoid constraint churn during animations.
* **State Management:**

  * Local state preferred.
  * Shared mutable global state discouraged unless architectural necessity.
* **ANTI-EMOJI POLICY [CRITICAL]:**

  * Never use emojis in UI labels, placeholder text, menus, alt text, or notifications.
  * Use SF Symbols or custom vector assets.
* **Responsiveness & Windowing:**

  * Support dynamic window resizing gracefully.
  * Avoid hardcoded frame assumptions.
  * Use `NSCollectionView` or layer-backed grids instead of manual layout math.
* **Icons:**

  * Prefer SF Symbols with hierarchical rendering.
  * Standardize symbol weight and scale globally.

---

## 3. DESIGN ENGINEERING DIRECTIVES (Bias Correction)

### Rule 1: Deterministic Typography

* Prefer:

  * `SF Pro Display`
  * `SF Pro Text`
  * `SF Mono`
* Avoid generic “web startup” typography pairings.
* Dashboard/software UI must remain sans-serif only.
* Use typography hierarchy via weight and spacing, not giant point sizes.

### Rule 2: Color Calibration

* Max one accent color.
* Avoid neon gradients and generic “AI purple.”
* Use restrained macOS-native palettes:

  * graphite
  * muted blue
  * deep amber
  * desaturated red
* Never use pure black.

### Rule 3: Layout Diversification

* Avoid default centered compositions.
* Prefer:

  * asymmetry
  * offset grouping
  * negative space
  * split-pane structures
* Sidebar/content layouts should feel intentional, not template-derived.

### Rule 4: Materiality & Elevation

* Shadows communicate hierarchy only.
* Avoid excessive floating panels.
* Use:

  * vibrancy
  * translucency
  * inner borders
  * subtle separation layers
* Generic rounded “cards everywhere” are discouraged.

### Rule 5: Interactive States

Every interaction must support:

* loading
* empty
* error
* success
* interrupted transition states

Avoid:

* spinning beachballs
* indeterminate ambiguity
* frozen UI during animation

Use:

* skeleton placeholders
* shimmer
* progressive disclosure
* optimistic transitions

### Rule 6: Forms & Controls

* Labels above controls.
* Consistent vertical rhythm.
* Inline validation preferred over modal interruption.
* Avoid alert spam.

---

## 4. CREATIVE PROACTIVITY (Anti-Slop Implementation)

### Liquid Glass Materiality

When translucency is appropriate:

* use `NSVisualEffectView`
* combine with:

  * inner borders
  * layered translucency
  * subtle noise
  * edge highlights

Avoid generic blurred rectangles.

### Magnetic Micro-Physics

If `MOTION_INTENSITY > 5`:

* buttons subtly attract cursor focus
* hover states should feel weighted
* use spring timing, not linear easing

Never drive hover animation through repeated layout invalidation.

### Perpetual Micro-Interactions

For active interfaces:

* breathing status indicators
* subtle layer drift
* shimmer passes
* ambient parallax
* asynchronous pulse cycles

Animations must remain low-amplitude and non-distracting.

### Layout Transitions

Prefer:

* layer transforms
* opacity
* scale
* position interpolation

Avoid:

* abrupt `isHidden` swaps
* relayout flicker
* frame snapping

### Sequential Orchestration

Lists and panes should reveal progressively:

* stagger opacity
* stagger translation
* stagger blur reduction

Avoid instant population dumps.

---

## 5. PERFORMANCE GUARDRAILS

### Rendering Discipline

* Never trigger full view hierarchy redraws unnecessarily.
* Use CALayer composition aggressively.
* Rasterize only when beneficial.

### Animation Constraints

Never animate:

* Auto Layout constraints continuously
* frame recalculation loops
* shadow paths dynamically per frame

Animate:

* transform
* opacity
* filter parameters
* layer-backed properties

### GPU Awareness

* Avoid excessive live blur regions.
* Avoid transparent layer stacking explosions.
* Minimize offscreen rendering.

### Layer Hierarchy Discipline

* Avoid deep nesting.
* Flatten where possible.
* Use explicit z-position sparingly.

---

## 6. TECHNICAL REFERENCE (Dial Definitions)

### DESIGN_VARIANCE (1-10)

**1-3**

* strict symmetry
* native utility aesthetic
* predictable structure

**4-7**

* offset compositions
* layered hierarchy
* intentional whitespace imbalance

**8-10**

* asymmetric pane weighting
* overlapping layer groups
* dramatic negative space
* cinematic composition

### MOTION_INTENSITY (1-10)

**1-3**

* static interface
* hover/focus only

**4-7**

* spring transitions
* staggered reveals
* ambient feedback

**8-10**

* coordinated choreography
* layered parallax
* cinematic state transitions

### VISUAL_DENSITY (1-10)

**1-3**

* gallery-like spacing
* minimal chrome

**4-7**

* balanced productivity UI

**8-10**

* dense operational interfaces
* thin separators
* compact metrics
* mono-spaced numerical systems

---

## 7. AI TELLS (Forbidden Patterns)

### Visual

* No neon glow spam.
* No pure black backgrounds.
* No excessive gradients.
* No fake frosted-glass overload.

### Typography

* No giant hero typography.
* No random font mixing.
* No serif dashboards.

### Layout

* No generic equal-width triptych layouts.
* No meaningless floating cards.
* No awkward padding inconsistencies.

### Motion

* No linear easing.
* No animation spam.
* No “everything animates constantly.”

### macOS Specific

* Do not imitate iOS unnecessarily.
* Respect desktop interaction density.
* Avoid oversized touch-target aesthetics unless explicitly requested.
* Avoid SwiftUI-style visual clichés inside AppKit apps.

---

## 8. THE CREATIVE ARSENAL

### Navigation

* Dock-style magnification
* Elastic segmented controls
* Morphing sidebars
* Contextual command palettes

### Layout

* Asymmetric inspector panes
* Floating utility surfaces
* Layered split views
* Expandable canvas workspaces

### Motion

* Spring-loaded reveal panels
* Layer-based parallax
* Velocity-aware transitions
* Context-preserving zoom transitions

### Typography

* Kinetic counters
* Scramble reveals
* Mono-space telemetry bands
* Animated emphasis states

### Micro-Interactions

* Spotlight hover borders
* Ripple confirmations
* Breathing indicators
* Ambient layer drift

---

## 9. THE “MOTION-ENGINE” PANEL PARADIGM

### A. Core Philosophy

* restrained
* premium
* tactile
* alive without noise

Panels should feel:

* composited
* weighted
* responsive
* physically coherent

### B. Motion Specs

* Use spring timing almost exclusively.
* Avoid abrupt opacity toggles.
* Maintain animation continuity across state changes.
* Infinite ambient motion must remain subtle and isolated.

### C. Preferred Interface Archetypes

1. **Intelligent Stack**

   * dynamically reprioritizing rows
   * animated positional swaps

2. **Command Surface**

   * staged typing
   * shimmer processing state
   * weighted cursor blink

3. **Live Status**

   * breathing indicators
   * delayed notification emergence

4. **Data Stream**

   * continuously drifting metrics
   * seamless horizontal movement

5. **Focus Mode**

   * animated contextual tooling
   * progressive emphasis transitions

---

## 10. FINAL PRE-FLIGHT CHECK

* [ ] Are animations layer-backed and GPU-friendly?
* [ ] Are CALayer transforms preferred over frame mutation?
* [ ] Are transitions spring-based instead of linear?
* [ ] Are empty/loading/error states implemented?
* [ ] Is AppKit behaving like desktop software instead of a stretched iPad app?
* [ ] Is visual hierarchy created via spacing/materiality instead of excessive cards?
* [ ] Are perpetual animations isolated and low-cost?
* [ ] Is the interface asymmetrical enough to avoid template aesthetics?
