---
name: appkit-autolayout
description: Comprehensive macOS AppKit Auto Layout guide covering NSView programmatic constraints, NSStackView, debugging, and best practices. Use when building native macOS apps with AppKit Auto Layout.
triggers:
  - AppKit
  - Auto Layout
  - AutoLayout
  - NSLayoutConstraint
  - NSView layout
  - NSStackView
  - NSLayoutAnchor
  - NSLayoutGuide
  - macOS native UI
  - constraint debugging
  - NSSplitView
  - NSScrollView layout
  - translatesAutoresizingMaskIntoConstraints
  - intrinsicContentSize
  - content hugging
  - compression resistance
---

# AppKit Auto Layout — Complete Development Guide

> **Scope**: macOS/AppKit only. All code is Swift. All views are NSView, not UIView.

---

## 1. Constraint Equation

Every constraint is a linear equation:

```
view1.attribute = multiplier × view2.attribute + constant
```

Seven components: Item1, Attribute1, Relationship (=, ≥, ≤), Multiplier, Item2, Attribute2, Constant.

**Attribute categories**:
- **Position**: leading, trailing, top, bottom, centerX, centerY, left, right
- **Size**: width, height
- **Baseline**: firstBaseline, lastBaseline
- **Margin**: leadingMargin, trailingMargin, topMargin, bottomMargin

**Rules**:
- Never mix position and size attributes (e.g., leading → width is invalid)
- Always prefer `leading/trailing` over `left/right` (RTL support)
- Constraints are equations, not assignments — Item1 and Item2 are interchangeable (invert multiplier & constant)

**Priority values (macOS-specific)**:

| Value | Constant | Purpose |
|-------|----------|---------|
| 1000 | `.required` | Must be satisfied |
| 750 | `.defaultHigh` | Default Compression Resistance |
| 510 | `.dragThatCanResizeWindow` | **macOS only** — drag that resizes window |
| 500 | `.windowSizeStayPut` | **macOS only** — window stays current size |
| 490 | `.dragThatCannotResizeWindow` | **macOS only** — drag that cannot resize window |
| 250 | `.defaultLow` | Default Content Hugging |
| 50 | `.fittingSizeCompression` | Used by fittingSize calculation |

---

## 2. Three Ways to Create Constraints

### ✅ Preferred: Layout Anchors (NSLayoutAnchor)

```swift
let label = NSTextField(labelWithString: "Hello")
label.translatesAutoresizingMaskIntoConstraints = false
view.addSubview(label)

NSLayoutConstraint.activate([
    label.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 16),
    label.topAnchor.constraint(equalTo: view.topAnchor, constant: 16),
    label.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -16),
])
```

**Type safety** — Anchor subclasses prevent invalid constraints at compile time:
- `NSLayoutXAxisAnchor` (leading, trailing, centerX, left, right)
- `NSLayoutYAxisAnchor` (top, bottom, centerY, firstBaseline, lastBaseline)
- `NSLayoutDimension` (width, height) — **only** dimension anchors support `multiplier`

```swift
// ✅ Compiles — both are X-axis
label.leadingAnchor.constraint(equalTo: view.trailingAnchor)

// ❌ Won't compile — X-axis vs Y-axis
label.leadingAnchor.constraint(equalTo: view.topAnchor)

// ✅ Dimension with multiplier
view2.widthAnchor.constraint(equalTo: view1.widthAnchor, multiplier: 0.5)
```

### NSLayoutConstraint Class Method

```swift
NSLayoutConstraint(
    item: subview,
    attribute: .leading,
    relatedBy: .equal,
    toItem: view,
    attribute: .leadingMargin,
    multiplier: 1.0,
    constant: 0.0
).isActive = true
```

**Only use when**: you need a multiplier on non-dimension attributes (rare).
**Downsides**: No type safety, 7 params, verbose.

### Visual Format Language (VFL)

```swift
let views = ["btn1": button1, "btn2": button2]
let metrics = ["sp": 8.0]
let constraints = NSLayoutConstraint.constraints(
    withVisualFormat: "H:[btn1]-sp-[btn2]",
    options: .alignAllBaseline,
    metrics: metrics,
    views: views
)
NSLayoutConstraint.activate(constraints)
```

**Downsides**: No compile-time checks, no multiplier, no aspect ratio, no baseline alignment.

---

## 3. NSView Auto Layout Core API

### translatesAutoresizingMaskIntoConstraints

```swift
// ⚠️ CRITICAL: Set to false for EVERY programmatically created view
let myView = NSView()
myView.translatesAutoresizingMaskIntoConstraints = false
view.addSubview(myView)
```

- Default is `true` — the system auto-generates constraints from the autoresizing mask
- If you add your own constraints without setting this to `false`, you WILL get conflicts
- Views created in Interface Builder have this set to `false` automatically

### Layout Lifecycle

```swift
class MyView: NSView {
    // 1. Update phase — modify constraints here
    override func updateConstraints() {
        // Modify constraints based on current state
        // ⚠️ Call super LAST (opposite of most overrides!)
        super.updateConstraints()
    }

    // 2. Layout phase — frame is set here, adjust subviews
    override func layout() {
        super.layout()  // ← Call super FIRST
        // Frame-based adjustments after layout
    }

    // 3. Display phase — drawing
    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
    }
}
```

**Triggering layout updates**:
```swift
view.needsUpdateConstraints = true  // Schedule constraint update
view.needsLayout = true              // Schedule layout pass
view.layoutSubtreeIfNeeded()         // Force immediate layout (entire subtree)
view.invalidateIntrinsicContentSize() // Recalculate intrinsic size
```

### Debugging Methods (available at runtime)

```swift
// Check if layout is ambiguous
view.hasAmbiguousLayout

// Get all constraints affecting a specific axis
view.constraintsAffectingLayout(for: .horizontal)  // NSLayoutConstraint.Orientation
view.constraintsAffectingLayout(for: .vertical)

// Jump between ambiguous solutions (debug only)
view.exerciseAmbiguityInLayout()

// fittingSize — minimum size that satisfies all constraints
view.fittingSize
```

> ⚠️ **macOS uses** `NSLayoutConstraint.Orientation` (.horizontal, .vertical), NOT `UILayoutConstraintAxis`.

---

## 4. NSLayoutConstraint API

### Activation (always batch)

```swift
// ✅ Recommended — batch activation, better performance
NSLayoutConstraint.activate([
    view.leadingAnchor.constraint(equalTo: parent.leadingAnchor),
    view.trailingAnchor.constraint(equalTo: parent.trailingAnchor),
    view.topAnchor.constraint(equalTo: parent.topAnchor),
    view.bottomAnchor.constraint(equalTo: parent.bottomAnchor),
])

// Deactivation
NSLayoutConstraint.deactivate([constraint1, constraint2])

// ❌ Avoid — individual activation is slower
view.addConstraint(constraint)  // Legacy API
constraint.isActive = true       // One at a time
```

### Modifiable vs Immutable Properties

```swift
// ✅ Can modify after creation
constraint.constant = 20.0
constraint.priority = .defaultHigh  // ⚠️ See priority rules below
constraint.identifier = "sidebar-width"

// ❌ Immutable — must remove and recreate
// constraint.multiplier
// constraint.firstItem / secondItem
// constraint.firstAttribute / secondAttribute
// constraint.relation
```

### Priority Modification Rules

```swift
// ❌ CRASH: Cannot change from/to .required (1000)
let c = view.widthAnchor.constraint(equalToConstant: 200)
c.priority = .required
c.isActive = true
c.priority = .defaultHigh  // 💥 Runtime crash

// ✅ Use 999 instead of 1000 if you need to change later
let c = view.widthAnchor.constraint(equalToConstant: 200)
c.priority = NSLayoutConstraint.Priority(999)
c.isActive = true
c.priority = .defaultHigh  // ✅ Fine
```

### Constraint Identifiers (essential for debugging)

```swift
let leading = label.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 16)
leading.identifier = "Label.leading"
```

When constraints conflict, identifiers appear in the console log instead of memory addresses.

---

## 5. NSLayoutGuide (Replace Dummy Spacer Views)

```swift
// ❌ Old pattern: invisible spacer views (waste of resources)
let spacer = NSView()
spacer.isHidden = true

// ✅ Modern pattern: layout guides (lightweight, no rendering)
let spacer = NSLayoutGuide()
view.addLayoutGuide(spacer)

NSLayoutConstraint.activate([
    spacer.leadingAnchor.constraint(equalTo: view1.trailingAnchor),
    spacer.trailingAnchor.constraint(equalTo: view2.leadingAnchor),
    spacer.widthAnchor.constraint(equalToConstant: 20),
])
```

Use guides for: equal spacing, alignment groups, invisible layout regions.

---

## 6. Intrinsic Content Size + CHCR

### Intrinsic Content Size

Some views know their natural size based on content (NSTextField, NSButton, NSImageView).

```swift
class MyCustomView: NSView {
    var content: String = "" {
        didSet {
            invalidateIntrinsicContentSize()  // ⚠️ Must call when content changes!
        }
    }

    override var intrinsicContentSize: NSSize {
        let size = calculateContentSize()
        return NSSize(
            width: size.width > 0 ? size.width : NSView.noIntrinsicMetric,
            height: size.height > 0 ? size.height : NSView.noIntrinsicMetric
        )
    }
}
```

> ⚠️ `intrinsicContentSize` must NOT depend on `frame` (frame may not be set yet).
> Return `NSView.noIntrinsicMetric` for dimensions with no natural size.

### Content Hugging & Compression Resistance (CHCR)

- **Content Hugging** = "I don't want to be BIGGER than my intrinsic size" (default: 250)
- **Compression Resistance** = "I don't want to be SMALLER than my intrinsic size" (default: 750)

```swift
// Two labels side by side — which stretches when there's extra space?
// → The one with LOWER content hugging
label1.setContentHuggingPriority(.defaultHigh, for: .horizontal)   // 750 — stays tight
label2.setContentHuggingPriority(.defaultLow, for: .horizontal)    // 250 — stretches

// Which gets truncated when space is tight?
// → The one with LOWER compression resistance
label1.setContentCompressionResistancePriority(.required, for: .horizontal)   // 1000
label2.setContentCompressionResistancePriority(.defaultHigh, for: .horizontal) // 750
```

### Multi-line Text on macOS

`NSTextField` may need `preferredMaxLayoutWidth` for correct multi-line height calculation:

```swift
textField.preferredMaxLayoutWidth = 300  // Wrap at this width
```

---

## 7. NSStackView (macOS-Specific Behavior)

### Key Differences from UIStackView

| Feature | NSStackView (macOS) | UIStackView (iOS) |
|---------|--------------------|--------------------|
| Direction property | `orientation` | `axis` |
| Has gravity areas | ✅ Yes (top/leading, center, bottom/trailing) | ❌ No |
| `detachesHiddenViews` | ✅ Default `true` — hidden views removed from hierarchy | ❌ N/A |
| Visibility priority | ✅ Per-view priority for clipping order | ❌ N/A |
| Default rendering | **Is a layer** (renders itself) | Not a rendering view |
| Adding views | `addView(_:in:)` or `addArrangedSubview(_:)` | `addArrangedSubview(_:)` |

### Gravity Areas

```swift
let stack = NSStackView()
stack.orientation = .horizontal

// Add views to specific gravity areas
stack.addView(leftButton, in: .leading)    // Pinned to leading edge
stack.addView(titleLabel, in: .center)     // Centered
stack.addView(closeButton, in: .trailing)  // Pinned to trailing edge
```

### Distribution

```swift
stack.distribution = .fill              // Default — views fill based on hugging/resistance
stack.distribution = .fillEqually       // All views same size
stack.distribution = .fillProportionally // Proportional to intrinsic size
stack.distribution = .equalSpacing      // Equal spacing between views
stack.distribution = .equalCentering    // Equal spacing between view centers
stack.distribution = .gravityAreas      // Uses gravity areas (macOS unique!)
```

> ⚠️ When using `.gravityAreas`, add views with `addView(_:in:)`.
> When using other distributions, use `addArrangedSubview(_:)`.

### detachesHiddenViews (Critical macOS Gotcha)

```swift
// ⚠️ Default is TRUE on macOS!
// When you set view.isHidden = true, NSStackView REMOVES it from the view hierarchy
stackView.detachesHiddenViews = true  // default

// If you depend on hidden views staying in the hierarchy:
stackView.detachesHiddenViews = false
```

### Visibility Priority

```swift
// Controls which views get clipped first when space is tight
stackView.setVisibilityPriority(.mustHold, for: importantView)        // Never clip
stackView.setVisibilityPriority(.detachOnlyIfNecessary, for: optionalView) // Clip if needed
stackView.setVisibilityPriority(.notVisible, for: hiddenView)         // Always hidden
```

### Custom Spacing

```swift
stackView.setCustomSpacing(20, after: headerView)
```

---

## 8. NSScrollView Layout

### macOS NSScrollView Hierarchy

```
NSScrollView
  └─ NSClipView (contentView)
       └─ documentView (your content)
  └─ NSScroller (vertical)
  └─ NSScroller (horizontal)
```

### Correct Setup

```swift
let scrollView = NSScrollView()
scrollView.translatesAutoresizingMaskIntoConstraints = false
scrollView.hasVerticalScroller = true
scrollView.hasHorizontalScroller = false

let contentView = NSView()
contentView.translatesAutoresizingMaskIntoConstraints = false
scrollView.documentView = contentView

NSLayoutConstraint.activate([
    // ScrollView pinned to parent
    scrollView.leadingAnchor.constraint(equalTo: view.leadingAnchor),
    scrollView.trailingAnchor.constraint(equalTo: view.trailingAnchor),
    scrollView.topAnchor.constraint(equalTo: view.topAnchor),
    scrollView.bottomAnchor.constraint(equalTo: view.bottomAnchor),

    // ⚠️ macOS: documentView constrained to NSClipView
    contentView.leadingAnchor.constraint(equalTo: scrollView.contentView.leadingAnchor),
    contentView.trailingAnchor.constraint(equalTo: scrollView.contentView.trailingAnchor),
    contentView.topAnchor.constraint(equalTo: scrollView.contentView.topAnchor),
    // ❌ Do NOT pin bottom to clipView (that prevents scrolling)
    // Instead, let content's own height constraints define scrollable area
])
```

### Constraint Behavior in ScrollView

- **Edge constraints** (leading, trailing, top, bottom) between ScrollView and content → define **scrollable content area**
- **Size/center constraints** (width, height, centerX, centerY) between ScrollView and content → affect ScrollView's **frame**

---

## 9. NSWindow + contentLayoutGuide

### Full-Size Content View Window

When using `titlebarAppearsTransparent` or `fullSizeContentView`, content extends behind the title bar. Use `contentLayoutGuide` to avoid overlap:

```swift
let window = NSWindow(
    contentRect: NSRect(x: 0, y: 0, width: 800, height: 600),
    styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
    backing: .buffered,
    defer: false
)
window.titlebarAppearsTransparent = true
window.titleVisibility = .hidden

let contentView = window.contentView!

// Background extends behind title bar
let bg = NSVisualEffectView()
bg.translatesAutoresizingMaskIntoConstraints = false
bg.material = .sidebar
contentView.addSubview(bg)

NSLayoutConstraint.activate([
    bg.leadingAnchor.constraint(equalTo: contentView.leadingAnchor),
    bg.trailingAnchor.constraint(equalTo: contentView.trailingAnchor),
    bg.topAnchor.constraint(equalTo: contentView.topAnchor),
    bg.bottomAnchor.constraint(equalTo: contentView.bottomAnchor),
])

// ⚠️ Main content uses contentLayoutGuide to avoid title bar area
if let guide = window.contentLayoutGuide as? NSLayoutGuide {
    let mainView = NSView()
    mainView.translatesAutoresizingMaskIntoConstraints = false
    contentView.addSubview(mainView)

    NSLayoutConstraint.activate([
        mainView.topAnchor.constraint(equalTo: guide.topAnchor, constant: 8),
        mainView.leadingAnchor.constraint(equalTo: guide.leadingAnchor, constant: 8),
        mainView.trailingAnchor.constraint(equalTo: guide.trailingAnchor, constant: -8),
        mainView.bottomAnchor.constraint(equalTo: guide.bottomAnchor, constant: -8),
    ])
}
```

> ⚠️ `window.contentLayoutGuide` is typed as `Any?` — must cast to `NSLayoutGuide`.

---

## 10. NSSplitView / NSSplitViewController

### Holding Priority (macOS Unique)

Controls which panel resizes when the window size changes:

```swift
let sidebar = NSSplitViewItem(sidebarWithViewController: sidebarVC)
sidebar.minimumThickness = 200
sidebar.maximumThickness = 400
sidebar.holdingPriority = NSLayoutConstraint.Priority(260)  // Low — resizes easily

let content = NSSplitViewItem(viewController: contentVC)
content.holdingPriority = NSLayoutConstraint.Priority(490)  // Higher — resists resizing

let inspector = NSSplitViewItem(viewController: inspectorVC)
inspector.minimumThickness = 200
inspector.canCollapse = true
inspector.holdingPriority = NSLayoutConstraint.Priority(250)  // Lowest — resizes first

splitViewController.splitViewItems = [sidebar, content, inspector]
```

**Higher holdingPriority → panel resists resizing more → other panels absorb size changes first.**

### Collapse/Expand Animation

```swift
// Toggle sidebar collapse
let item = splitViewController.splitViewItems[0]
NSAnimationContext.runAnimationGroup { context in
    context.duration = 0.25
    item.animator().isCollapsed.toggle()
}
```

---

## 11. NSViewController Lifecycle

```swift
class MyViewController: NSViewController {
    override func loadView() {
        self.view = NSView()  // ⚠️ Must set self.view if not using nib
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        // Create and constrain subviews here
    }

    override func viewWillAppear() {
        super.viewWillAppear()
        // View is about to be displayed (macOS 10.10+)
    }

    override func viewDidAppear() {
        super.viewDidAppear()
        // View is now on screen; window and frame are valid
    }

    override func viewWillLayout() {
        super.viewWillLayout()
        // About to layout — adjust constraints if needed
    }

    override func viewDidLayout() {
        super.viewDidLayout()
        // Layout complete — frames are final
    }
}
```

> ⚠️ **macOS gotcha**: If `nibName` and `bundle` are both nil (macOS 10.10+), AppKit looks for a nib matching the class name. Override `loadView()` to create views programmatically.

---

## 12. macOS vs iOS Differences

| Aspect | macOS (AppKit) | iOS (UIKit) |
|--------|---------------|-------------|
| Base view | `NSView` | `UIView` |
| Coordinate origin | **Bottom-left** | Top-left |
| Layout method | `layout()` | `layoutSubviews()` |
| Needs layout | `needsLayout = true` | `setNeedsLayout()` |
| Force layout | `layoutSubtreeIfNeeded()` | `layoutIfNeeded()` |
| Update constraints | `needsUpdateConstraints = true` | `setNeedsUpdateConstraints()` |
| Stack view direction | `.orientation` | `.axis` |
| Orientation enum | `NSUserInterfaceLayoutOrientation` | `NSLayoutConstraint.Axis` |
| Constraint orientation | `NSLayoutConstraint.Orientation` | `NSLayoutConstraint.Axis` |
| Layout guide | `NSLayoutGuide` | `UILayoutGuide` |
| Safe area | `safeAreaLayoutGuide` (macOS 11+) | `safeAreaLayoutGuide` (iOS 11+) |
| Window layout guide | `NSWindow.contentLayoutGuide` | N/A |
| Split view priority | `holdingPriority` | N/A |
| Stack detaches hidden | `detachesHiddenViews = true` (default) | N/A |

---

## 13. Debugging

### Three Error Types

#### 1. Unsatisfiable Constraints (Conflicts)

Console output:
```
Unable to simultaneously satisfy constraints.
(
    "<NSLayoutConstraint:0x... V:|-(20)-[label] (active, names: '|':NSView:0x...)>",
    "<NSLayoutConstraint:0x... V:|-(30)-[label] (active)>"
)
Will attempt to recover by breaking constraint ...
```

**Fix checklist**:
1. Did you forget `translatesAutoresizingMaskIntoConstraints = false`?
2. Use `constraint.identifier` to find the offending constraint
3. Lower one constraint's priority from `.required` to 999

#### 2. Ambiguous Layout

```swift
// Detect
po view.hasAmbiguousLayout

// Find all ambiguous views
func findAmbiguous(in view: NSView) {
    if view.hasAmbiguousLayout {
        print("⚠️ Ambiguous: \(view)")
        print("  H: \(view.constraintsAffectingLayout(for: .horizontal))")
        print("  V: \(view.constraintsAffectingLayout(for: .vertical))")
    }
    view.subviews.forEach { findAmbiguous(in: $0) }
}

// Toggle between valid solutions
view.exerciseAmbiguityInLayout()
```

#### 3. Logical Errors

Layout is valid but wrong. Use Xcode's **Debug View Hierarchy** (Debug → View Debugging → Capture View Hierarchy).

### Symbolic Breakpoint

Add in Xcode: Symbol = `NSViewAlertForUnsatisfiableConstraints`
This pauses execution at the exact moment a conflict is detected.

---

## 14. Common Pitfalls

### Pitfall 1: Forgot translatesAutoresizingMaskIntoConstraints

```swift
// ❌ Constraints + autoresizing mask = conflict
let label = NSTextField(labelWithString: "Hi")
view.addSubview(label)
label.topAnchor.constraint(equalTo: view.topAnchor).isActive = true  // 💥

// ✅
let label = NSTextField(labelWithString: "Hi")
label.translatesAutoresizingMaskIntoConstraints = false
view.addSubview(label)
label.topAnchor.constraint(equalTo: view.topAnchor).isActive = true  // ✅
```

### Pitfall 2: Modifying constraints in layout()

```swift
// ❌ Can cause infinite layout loops
override func layout() {
    super.layout()
    someConstraint.constant = bounds.width / 2  // Triggers another layout pass!
}

// ✅ Modify constraints in updateConstraints()
override func updateConstraints() {
    someConstraint.constant = calculatedValue
    super.updateConstraints()  // ⚠️ Call super LAST
}
```

### Pitfall 3: NSStackView detachesHiddenViews

```swift
// ⚠️ Default is true — hidden views are REMOVED from the view hierarchy
stackView.detachesHiddenViews = true  // default!

// If you need hidden views to stay in the hierarchy:
stackView.detachesHiddenViews = false
```

### Pitfall 4: Changing required priority

```swift
// ❌ Crash
constraint.priority = .required  // 1000
constraint.isActive = true
constraint.priority = .defaultHigh  // 💥

// ✅ Start at 999
constraint.priority = NSLayoutConstraint.Priority(999)
constraint.isActive = true
constraint.priority = .defaultHigh  // ✅
```

### Pitfall 5: contentLayoutGuide type

```swift
// ❌ Compile error — contentLayoutGuide is Any?
view.topAnchor.constraint(equalTo: window.contentLayoutGuide.topAnchor)

// ✅ Cast first
if let guide = window.contentLayoutGuide as? NSLayoutGuide {
    view.topAnchor.constraint(equalTo: guide.topAnchor)
}
```

### Pitfall 6: Mixing leading/trailing with left/right

```swift
// ❌ Semantic mismatch
view.leadingAnchor.constraint(equalTo: other.leftAnchor)

// ✅ Be consistent
view.leadingAnchor.constraint(equalTo: other.leadingAnchor)
```

### Pitfall 7: Individual constraint activation

```swift
// ❌ Slower, less efficient
constraint1.isActive = true
constraint2.isActive = true
constraint3.isActive = true

// ✅ Batch activation
NSLayoutConstraint.activate([constraint1, constraint2, constraint3])
```

### Pitfall 8: NSScrollView bottom pinning

```swift
// ❌ Pinning documentView bottom to clipView prevents scrolling
contentView.bottomAnchor.constraint(equalTo: scrollView.contentView.bottomAnchor)

// ✅ Let content height be determined by its own subviews
// Only pin top, leading, trailing to clipView
// Content's internal constraints determine scrollable height
```

### Pitfall 9: forgetting invalidateIntrinsicContentSize

```swift
// ❌ Content changes but layout doesn't update
class MyView: NSView {
    var text: String = "" {
        didSet { /* nothing */ }
    }
}

// ✅
class MyView: NSView {
    var text: String = "" {
        didSet { invalidateIntrinsicContentSize() }
    }
}
```

---

## 15. Practical Examples

### Example 1: Basic View Setup Pattern

```swift
class MyViewController: NSViewController {
    private let headerLabel = NSTextField(labelWithString: "Title")
    private let contentView = NSView()
    private let footerButton = NSButton(title: "Done", target: nil, action: nil)

    override func loadView() {
        self.view = NSView()
    }

    override func viewDidLoad() {
        super.viewDidLoad()

        [headerLabel, contentView, footerButton].forEach {
            $0.translatesAutoresizingMaskIntoConstraints = false
            view.addSubview($0)
        }

        NSLayoutConstraint.activate([
            headerLabel.topAnchor.constraint(equalTo: view.topAnchor, constant: 16),
            headerLabel.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 16),
            headerLabel.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -16),

            contentView.topAnchor.constraint(equalTo: headerLabel.bottomAnchor, constant: 12),
            contentView.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 16),
            contentView.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -16),

            footerButton.topAnchor.constraint(equalTo: contentView.bottomAnchor, constant: 12),
            footerButton.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -16),
            footerButton.bottomAnchor.constraint(equalTo: view.bottomAnchor, constant: -16),
        ])
    }
}
```

### Example 2: Dynamic Constraint Switching

```swift
class AdaptiveViewController: NSViewController {
    private var compactConstraints: [NSLayoutConstraint] = []
    private var regularConstraints: [NSLayoutConstraint] = []

    override func viewDidLoad() {
        super.viewDidLoad()

        let sidebar = NSView()
        let content = NSView()
        [sidebar, content].forEach {
            $0.translatesAutoresizingMaskIntoConstraints = false
            view.addSubview($0)
        }

        // Compact: stacked vertically
        compactConstraints = [
            sidebar.topAnchor.constraint(equalTo: view.topAnchor),
            sidebar.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            sidebar.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            sidebar.heightAnchor.constraint(equalToConstant: 200),
            content.topAnchor.constraint(equalTo: sidebar.bottomAnchor),
            content.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            content.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            content.bottomAnchor.constraint(equalTo: view.bottomAnchor),
        ]

        // Regular: side by side
        regularConstraints = [
            sidebar.topAnchor.constraint(equalTo: view.topAnchor),
            sidebar.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            sidebar.bottomAnchor.constraint(equalTo: view.bottomAnchor),
            sidebar.widthAnchor.constraint(equalToConstant: 250),
            content.topAnchor.constraint(equalTo: view.topAnchor),
            content.leadingAnchor.constraint(equalTo: sidebar.trailingAnchor),
            content.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            content.bottomAnchor.constraint(equalTo: view.bottomAnchor),
        ]

        NSLayoutConstraint.activate(regularConstraints)
    }

    func switchToCompact() {
        NSLayoutConstraint.deactivate(regularConstraints)
        NSLayoutConstraint.activate(compactConstraints)
    }

    func switchToRegular() {
        NSLayoutConstraint.deactivate(compactConstraints)
        NSLayoutConstraint.activate(regularConstraints)
    }
}
```

### Example 3: Animated Constraint Changes

```swift
func expandPanel() {
    panelWidthConstraint.constant = 300

    NSAnimationContext.runAnimationGroup { context in
        context.duration = 0.3
        context.allowsImplicitAnimation = true
        view.layoutSubtreeIfNeeded()  // ← macOS: layoutSubtreeIfNeeded, NOT layoutIfNeeded
    }
}
```

### Example 4: Equal Spacing with Layout Guides

```swift
let buttons = (1...4).map { NSButton(title: "Btn \($0)", target: nil, action: nil) }
let spacers = (0...4).map { _ -> NSLayoutGuide in
    let guide = NSLayoutGuide()
    view.addLayoutGuide(guide)
    return guide
}

buttons.forEach {
    $0.translatesAutoresizingMaskIntoConstraints = false
    view.addSubview($0)
}

var constraints: [NSLayoutConstraint] = []
constraints.append(spacers[0].leadingAnchor.constraint(equalTo: view.leadingAnchor))

for (i, button) in buttons.enumerated() {
    constraints.append(button.leadingAnchor.constraint(equalTo: spacers[i].trailingAnchor))
    constraints.append(button.centerYAnchor.constraint(equalTo: view.centerYAnchor))
    constraints.append(spacers[i + 1].leadingAnchor.constraint(equalTo: button.trailingAnchor))
    if i > 0 {
        constraints.append(spacers[i].widthAnchor.constraint(equalTo: spacers[0].widthAnchor))
    }
}

constraints.append(spacers[4].trailingAnchor.constraint(equalTo: view.trailingAnchor))
constraints.append(spacers[4].widthAnchor.constraint(equalTo: spacers[0].widthAnchor))

NSLayoutConstraint.activate(constraints)
```

### Example 5: NSStackView Form

```swift
class FormViewController: NSViewController {
    override func loadView() { self.view = NSView() }

    override func viewDidLoad() {
        super.viewDidLoad()

        let form = NSStackView()
        form.orientation = .vertical
        form.alignment = .leading
        form.spacing = 12
        form.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(form)

        NSLayoutConstraint.activate([
            form.topAnchor.constraint(equalTo: view.topAnchor, constant: 20),
            form.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 20),
            form.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -20),
        ])

        // Each row is a horizontal stack
        for label in ["Name:", "Email:", "Phone:"] {
            let row = NSStackView()
            row.orientation = .horizontal
            row.spacing = 8
            row.alignment = .firstBaseline

            let labelField = NSTextField(labelWithString: label)
            labelField.setContentHuggingPriority(.defaultHigh, for: .horizontal)
            labelField.widthAnchor.constraint(equalToConstant: 80).isActive = true

            let input = NSTextField()
            input.placeholderString = "Enter \(label.dropLast())"
            input.setContentHuggingPriority(.defaultLow, for: .horizontal)

            row.addArrangedSubview(labelField)
            row.addArrangedSubview(input)

            form.addArrangedSubview(row)
            row.widthAnchor.constraint(equalTo: form.widthAnchor).isActive = true
        }
    }
}
```

---

## 16. Debugging Checklist

When layout breaks, check in this order:

- [ ] Every programmatic view has `translatesAutoresizingMaskIntoConstraints = false`
- [ ] Constraints fully define position AND size for each view (no ambiguity)
- [ ] No conflicting `.required` constraints
- [ ] CHCR priorities set correctly (especially when multiple views compete for space)
- [ ] Using `leading/trailing` consistently (not mixed with `left/right`)
- [ ] NSScrollView: documentView constrained correctly to clipView
- [ ] NSStackView: distribution matches how views were added
- [ ] Key constraints have `.identifier` set
- [ ] Window `contentMinSize` is reasonable
- [ ] For fullSizeContentView windows: using `contentLayoutGuide`
- [ ] Not modifying constraints inside `layout()` (use `updateConstraints()`)

---

## 17. Quick Reference

### Minimum Viable Constraint Setup

Every view needs **4 constraints** (or equivalent via intrinsic content size):
- X position (leading or centerX)
- Y position (top or centerY)
- Width (explicit, or trailing anchor, or intrinsic)
- Height (explicit, or bottom anchor, or intrinsic)

### Pin to Edges (most common pattern)

```swift
NSLayoutConstraint.activate([
    child.leadingAnchor.constraint(equalTo: parent.leadingAnchor),
    child.trailingAnchor.constraint(equalTo: parent.trailingAnchor),
    child.topAnchor.constraint(equalTo: parent.topAnchor),
    child.bottomAnchor.constraint(equalTo: parent.bottomAnchor),
])
```

### Center in Parent

```swift
NSLayoutConstraint.activate([
    child.centerXAnchor.constraint(equalTo: parent.centerXAnchor),
    child.centerYAnchor.constraint(equalTo: parent.centerYAnchor),
])
```

### Fixed Size

```swift
NSLayoutConstraint.activate([
    view.widthAnchor.constraint(equalToConstant: 200),
    view.heightAnchor.constraint(equalToConstant: 100),
])
```

### Aspect Ratio

```swift
view.widthAnchor.constraint(equalTo: view.heightAnchor, multiplier: 16.0/9.0).isActive = true
```

### Minimum API Versions

| API | macOS Version |
|-----|---------------|
| NSLayoutConstraint | 10.7+ |
| NSStackView | 10.9+ |
| NSLayoutGuide | 10.11+ |
| NSLayoutAnchor | 10.11+ |
| NSStackView.detachesHiddenViews | 10.11+ |
| NSWindow.contentLayoutGuide | 10.10+ |
| safeAreaLayoutGuide | 11.0+ |

