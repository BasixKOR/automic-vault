import AppKit
import Security

private final class HeaderGlitchTextAnimator {
    private struct Burst {
        let startedAt: Date
        let endsAt: Date
    }

    private weak var field: NSTextField?
    private let size: CGFloat
    private let baseColor: NSColor
    private let glitchColor: NSColor
    private let weight: NSFont.Weight
    private let tracking: CGFloat
    private let timerInterval: TimeInterval = 1.0 / 30.0

    private var timer: Timer?
    private var baseText = ""
    private var bursts: [Burst] = []
    private var nextShiftAt = Date.distantPast

    init(
        field: NSTextField,
        size: CGFloat,
        baseColor: NSColor,
        glitchColor: NSColor,
        weight: NSFont.Weight,
        tracking: CGFloat
    ) {
        self.field = field
        self.size = size
        self.baseColor = baseColor
        self.glitchColor = glitchColor
        self.weight = weight
        self.tracking = tracking
    }

    deinit {
        stop()
    }

    func setText(_ text: String?, animated: Bool) {
        guard let text, !text.isEmpty else {
            baseText = ""
            stop()
            field?.stringValue = ""
            return
        }

        if text == baseText {
            if animated {
                startTimerIfNeeded()
            } else {
                stop()
            }
            return
        }

        baseText = text
        bursts.removeAll(keepingCapacity: true)
        applyCurrentFrame()

        if animated {
            scheduleNextShift(from: Date())
            startTimerIfNeeded()
        } else {
            stopTimer()
        }
    }

    func stop() {
        stopTimer()
        bursts.removeAll(keepingCapacity: true)
        applyCurrentFrame()
    }

    private func startTimerIfNeeded() {
        guard timer == nil else { return }
        let timer = Timer(timeInterval: timerInterval, repeats: true) { [weak self] _ in
            self?.tick()
        }
        self.timer = timer
        RunLoop.main.add(timer, forMode: .common)
    }

    private func stopTimer() {
        timer?.invalidate()
        timer = nil
    }

    private func tick() {
        guard !baseText.isEmpty else {
            stop()
            return
        }

        let now = Date()

        if now >= nextShiftAt {
            let duration = burstDuration()
            bursts.append(
                Burst(
                    startedAt: now,
                    endsAt: now.addingTimeInterval(duration)
                )
            )
            nextShiftAt = now.addingTimeInterval(duration + shimmerPause())
            applyCurrentFrame()
        }

        guard !bursts.isEmpty else { return }

        var didChange = false
        bursts.removeAll { burst in
            let expired = now >= burst.endsAt
            didChange = didChange || expired
            return expired
        }

        if didChange || !bursts.isEmpty {
            applyCurrentFrame()
        }
    }

    private func scheduleNextShift(from date: Date) {
        nextShiftAt = date.addingTimeInterval(.random(in: 0.36 ... 0.72))
    }

    private func burstDuration() -> TimeInterval {
        let baseDuration = TimeInterval.random(in: 1.10 ... 1.46)
        return baseDuration * Double.random(in: 0.92 ... 1.12)
    }

    private func shimmerPause() -> TimeInterval {
        TimeInterval.random(in: 0.82 ... 1.56)
    }

    private func applyCurrentFrame() {
        guard let field else { return }
        let rendered = NSMutableAttributedString(
            attributedString: UIStyle.attributedMonoText(
                baseText,
                size: size,
                color: baseColor,
                weight: weight,
                tracking: tracking
            )
        )

        let now = Date()
        for index in 0 ..< baseText.count {
            guard let color = sweepColor(for: index, at: now) else { continue }
            let range = NSRange(location: index, length: 1)
            rendered.addAttributes(
                [
                    .font: UIStyle.monoFont(size: size, weight: weight),
                    .foregroundColor: color,
                    .kern: tracking,
                    .paragraphStyle: UIStyle.wrapParagraphStyle()
                ],
                range: range
            )
        }

        field.attributedStringValue = rendered
    }

    private func sweepColor(for index: Int, at now: Date) -> NSColor? {
        let count = baseText.count
        guard count > 0 else { return nil }
        let center = Double(count - 1) / 2.0
        let maxDistance = max(center, Double(count - 1) - center)

        var strongestAlpha = 0.0
        for burst in bursts {
            let duration = burst.endsAt.timeIntervalSince(burst.startedAt)
            guard duration > 0 else { continue }
            let progress = now.timeIntervalSince(burst.startedAt) / duration
            guard progress >= 0, progress <= 1 else { continue }

            let waveRadius = progress * maxDistance
            let distance = abs(Double(index) - center)
            let waveWidth = max(1.0, maxDistance * 0.28)
            let delta = abs(distance - waveRadius)
            guard delta <= waveWidth else { continue }

            let ringStrength = 1.0 - (delta / waveWidth)
            let easedRing = ringStrength * ringStrength * (3.0 - 2.0 * ringStrength)
            let fadeIn = min(1.0, progress / 0.12)
            let fadeOut = 1.0 - max(0.0, progress - 0.78) / 0.22
            strongestAlpha = max(strongestAlpha, easedRing * fadeIn * fadeOut)
        }

        guard strongestAlpha > 0 else { return nil }
        return baseColor.blended(
            withFraction: CGFloat(min(0.78, strongestAlpha)),
            of: glitchColor
        ) ?? glitchColor
    }
}

final class LayerGlitchTextAnimator {
    private struct Burst {
        let startedAt: Date
        let endsAt: Date
    }

    private weak var layer: CATextLayer?
    private let size: CGFloat
    private let baseColor: NSColor
    private let glitchColor: NSColor
    private let weight: NSFont.Weight
    private let tracking: CGFloat
    private let timerInterval: TimeInterval = 1.0 / 30.0

    private var timer: Timer?
    private var baseText = ""
    private var bursts: [Burst] = []
    private var nextShiftAt = Date.distantPast

    init(
        layer: CATextLayer,
        size: CGFloat,
        baseColor: NSColor,
        glitchColor: NSColor,
        weight: NSFont.Weight,
        tracking: CGFloat
    ) {
        self.layer = layer
        self.size = size
        self.baseColor = baseColor
        self.glitchColor = glitchColor
        self.weight = weight
        self.tracking = tracking
    }

    deinit {
        stop()
    }

    func setText(_ text: String?, animated: Bool) {
        guard let text, !text.isEmpty else {
            baseText = ""
            stopTimer()
            bursts.removeAll(keepingCapacity: true)
            layer?.string = nil
            return
        }

        if text != baseText {
            baseText = text
            bursts.removeAll(keepingCapacity: true)
            applyCurrentFrame()
        }

        if animated {
            scheduleNextShift(from: Date())
            startTimerIfNeeded()
        } else {
            stop()
        }
    }

    func stop() {
        stopTimer()
        bursts.removeAll(keepingCapacity: true)
        applyCurrentFrame()
    }

    private func startTimerIfNeeded() {
        guard timer == nil else { return }
        let timer = Timer(timeInterval: timerInterval, repeats: true) { [weak self] _ in
            self?.tick()
        }
        self.timer = timer
        RunLoop.main.add(timer, forMode: .common)
    }

    private func stopTimer() {
        timer?.invalidate()
        timer = nil
    }

    private func tick() {
        guard !baseText.isEmpty else {
            stop()
            return
        }

        let now = Date()

        if now >= nextShiftAt {
            let duration = burstDuration()
            bursts.append(
                Burst(
                    startedAt: now,
                    endsAt: now.addingTimeInterval(duration)
                )
            )
            nextShiftAt = now.addingTimeInterval(duration + shimmerPause())
            applyCurrentFrame()
        }

        guard !bursts.isEmpty else { return }

        var didChange = false
        bursts.removeAll { burst in
            let expired = now >= burst.endsAt
            didChange = didChange || expired
            return expired
        }

        if didChange || !bursts.isEmpty {
            applyCurrentFrame()
        }
    }

    private func scheduleNextShift(from date: Date) {
        nextShiftAt = date.addingTimeInterval(.random(in: 0.36 ... 0.72))
    }

    private func burstDuration() -> TimeInterval {
        let baseDuration = TimeInterval.random(in: 1.10 ... 1.46)
        return baseDuration * Double.random(in: 0.92 ... 1.12)
    }

    private func shimmerPause() -> TimeInterval {
        TimeInterval.random(in: 0.82 ... 1.56)
    }

    private func applyCurrentFrame() {
        guard let layer else { return }
        let rendered = NSMutableAttributedString(
            attributedString: UIStyle.attributedMonoText(
                baseText,
                size: size,
                color: baseColor,
                weight: weight,
                tracking: tracking
            )
        )

        let now = Date()
        for index in 0 ..< baseText.count {
            guard let color = sweepColor(for: index, at: now) else { continue }
            let range = NSRange(location: index, length: 1)
            rendered.addAttributes(
                [
                    .font: UIStyle.monoFont(size: size, weight: weight),
                    .foregroundColor: color,
                    .kern: tracking,
                    .paragraphStyle: UIStyle.wrapParagraphStyle()
                ],
                range: range
            )
        }

        layer.string = rendered
    }

    private func sweepColor(for index: Int, at now: Date) -> NSColor? {
        let count = baseText.count
        guard count > 0 else { return nil }
        let center = Double(count - 1) / 2.0
        let maxDistance = max(center, Double(count - 1) - center)

        var strongestAlpha = 0.0
        for burst in bursts {
            let duration = burst.endsAt.timeIntervalSince(burst.startedAt)
            guard duration > 0 else { continue }
            let progress = now.timeIntervalSince(burst.startedAt) / duration
            guard progress >= 0, progress <= 1 else { continue }

            let waveRadius = progress * maxDistance
            let distance = abs(Double(index) - center)
            let waveWidth = max(1.0, maxDistance * 0.28)
            let delta = abs(distance - waveRadius)
            guard delta <= waveWidth else { continue }

            let ringStrength = 1.0 - (delta / waveWidth)
            let easedRing = ringStrength * ringStrength * (3.0 - 2.0 * ringStrength)
            let fadeIn = min(1.0, progress / 0.12)
            let fadeOut = 1.0 - max(0.0, progress - 0.78) / 0.22
            strongestAlpha = max(strongestAlpha, easedRing * fadeIn * fadeOut)
        }

        guard strongestAlpha > 0 else { return nil }
        return baseColor.blended(
            withFraction: CGFloat(min(0.78, strongestAlpha)),
            of: glitchColor
        ) ?? glitchColor
    }
}

private final class MastheadButton: NSButton {
    struct Palette {
        let baseChrome: UIStyle.ControlChrome
        let hoverChrome: UIStyle.ControlChrome
        let disabledChrome: UIStyle.ControlChrome
    }

    var palette: Palette? {
        didSet { updateAppearance() }
    }

    private var trackingArea: NSTrackingArea?
    private var isHovering = false {
        didSet { updateAppearance() }
    }

    override var isEnabled: Bool {
        didSet { updateAppearance() }
    }

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        wantsLayer = true
        layer = CALayer()
        isBordered = false
        layerContentsRedrawPolicy = .onSetNeedsDisplay
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        if let trackingArea {
            removeTrackingArea(trackingArea)
        }
        let trackingArea = NSTrackingArea(
            rect: bounds,
            options: [.activeInKeyWindow, .inVisibleRect, .mouseEnteredAndExited],
            owner: self,
            userInfo: nil
        )
        addTrackingArea(trackingArea)
        self.trackingArea = trackingArea
    }

    override func mouseEntered(with event: NSEvent) {
        isHovering = true
        super.mouseEntered(with: event)
    }

    override func mouseExited(with event: NSEvent) {
        isHovering = false
        super.mouseExited(with: event)
    }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        updateAppearance()
    }

    override func layout() {
        super.layout()
        UIStyle.layoutControlChrome(in: layer)
    }

    private func updateAppearance() {
        guard let palette else { return }
        let chrome: UIStyle.ControlChrome
        if isEnabled == false {
            chrome = palette.disabledChrome
        } else if isHovering {
            chrome = palette.hoverChrome
        } else {
            chrome = palette.baseChrome
        }

        UIStyle.applyControlChrome(to: layer, chrome: chrome)
        contentTintColor = chrome.contentColor
    }
}

private final class CommandPaletteTextField: NSTextField {
    var onActivate: (() -> Void)?

    override func mouseDown(with event: NSEvent) {
        onActivate?()
        super.mouseDown(with: event)
    }

    override func becomeFirstResponder() -> Bool {
        let accepted = super.becomeFirstResponder()
        if accepted {
            onActivate?()
        }
        return accepted
    }
}

private final class CommandPaletteView: NSView {
    private static let placeholderText = "COMMAND PALETTE · \u{2318}P · TYPE TO QUERY"
    private static let placeholderFontSize: CGFloat = 10
    private static let placeholderMinimumTracking: CGFloat = 0.45
    private static let placeholderTrailingSlack: CGFloat = 8

    let field = CommandPaletteTextField(string: "")
    var onActivate: (() -> Void)?
    private var trackingArea: NSTrackingArea?
    private var isHovering = false {
        didSet { updateAppearance() }
    }

    var isActive = false {
        didSet { updateAppearance() }
    }

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        wantsLayer = true
        layer = CALayer()

        field.onActivate = { [weak self] in
            self?.onActivate?()
        }
        field.isEditable = true
        field.isSelectable = true
        field.isBordered = false
        field.isBezeled = false
        field.drawsBackground = false
        field.backgroundColor = .clear
        field.focusRingType = .none
        field.font = UIStyle.monoFont(size: 11, weight: .medium)
        field.textColor = UIStyle.text.withAlphaComponent(0.84)
        field.lineBreakMode = .byClipping
        field.maximumNumberOfLines = 1
        field.usesSingleLineMode = true
        field.cell?.wraps = false
        field.cell?.isScrollable = true
        updatePlaceholderText()
        addSubview(field)
        updateAppearance()
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        if let trackingArea {
            removeTrackingArea(trackingArea)
        }
        let trackingArea = NSTrackingArea(
            rect: bounds,
            options: [.activeInKeyWindow, .inVisibleRect, .mouseEnteredAndExited],
            owner: self,
            userInfo: nil
        )
        addTrackingArea(trackingArea)
        self.trackingArea = trackingArea
    }

    override func layout() {
        super.layout()
        field.frame = bounds.insetBy(dx: 10, dy: 5)
        updatePlaceholderText()
        UIStyle.layoutControlChrome(in: layer)
    }

    override func mouseEntered(with event: NSEvent) {
        isHovering = true
        super.mouseEntered(with: event)
    }

    override func mouseExited(with event: NSEvent) {
        isHovering = false
        super.mouseExited(with: event)
    }

    override func mouseDown(with event: NSEvent) {
        onActivate?()
        window?.makeFirstResponder(field)
        super.mouseDown(with: event)
    }

    private func updateAppearance() {
        let hasText = !field.stringValue.isEmpty
        let chrome: UIStyle.ControlChrome

        if isActive {
            chrome = UIStyle.ControlChrome(
                topBackgroundColor: .clear,
                bottomBackgroundColor: .clear,
                borderColor: NSColor(calibratedWhite: 0.42, alpha: 0.9),
                contentColor: UIStyle.text.withAlphaComponent(0.9),
                topInnerStrokeColor: nil,
                bottomInnerStrokeColor: nil
            )
        } else if isHovering {
            chrome = UIStyle.ControlChrome(
                topBackgroundColor: UIStyle.accent.withAlphaComponent(0.05),
                bottomBackgroundColor: UIStyle.accent.withAlphaComponent(0.02),
                borderColor: UIStyle.accent.withAlphaComponent(0.28),
                contentColor: UIStyle.accent,
                topInnerStrokeColor: UIStyle.accent.withAlphaComponent(0.14),
                bottomInnerStrokeColor: UIStyle.accent.withAlphaComponent(0.06)
            )
        } else if hasText {
            chrome = UIStyle.ControlChrome(
                topBackgroundColor: .clear,
                bottomBackgroundColor: .clear,
                borderColor: NSColor(calibratedWhite: 0.32, alpha: 0.88),
                contentColor: UIStyle.text.withAlphaComponent(0.84),
                topInnerStrokeColor: nil,
                bottomInnerStrokeColor: nil
            )
        } else {
            chrome = UIStyle.ControlChrome(
                topBackgroundColor: .clear,
                bottomBackgroundColor: .clear,
                borderColor: NSColor(calibratedWhite: 0.24, alpha: 0.82),
                contentColor: UIStyle.text.withAlphaComponent(0.84),
                topInnerStrokeColor: nil,
                bottomInnerStrokeColor: nil
            )
        }

        UIStyle.applyControlChrome(to: layer, chrome: chrome)
        field.textColor = chrome.contentColor
    }

    private func updatePlaceholderText() {
        let text = Self.placeholderText
        let font = UIStyle.monoFont(size: Self.placeholderFontSize, weight: .light)
        let naturalWidth = (text as NSString).size(withAttributes: [.font: font]).width
        let targetWidth = max(field.bounds.width - Self.placeholderTrailingSlack, 0)
        let extraWidth = max(targetWidth - naturalWidth, 0)
        let trackingSlots = max(text.count - 1, 1)
        let tracking = max(
            Self.placeholderMinimumTracking,
            extraWidth / CGFloat(trackingSlots)
        )

        field.placeholderAttributedString = UIStyle.attributedMonoText(
            text,
            size: Self.placeholderFontSize,
            color: UIStyle.text.withAlphaComponent(0.34),
            weight: .light,
            tracking: tracking
        )
    }
}

final class RootViewController: NSViewController, DossierViewDelegate, PackageFieldViewDelegate, NSUserInterfaceValidations, NSTextFieldDelegate {
    private static let searchPageSize = 64
    private static let searchLoadMoreThreshold: CGFloat = 240
    private static let codeSignatureHashLength = 8
    // Right masthead controls use their own vertical tuning so the left
    // title cluster can stay fixed while the status/search/button baselines
    // are adjusted together.
    private static let leftMastheadTitleYOffset: CGFloat = 2
    private static let rightMastheadSearchYOffset: CGFloat = 2
    private static let rightMastheadStatusYOffset: CGFloat = 2
    private static let rightMastheadUpdateButtonYOffset: CGFloat = -3

    private enum PaletteCommand: String, CaseIterable, Equatable {
        case all
        case pulse

        init?(query: String) {
            self.init(rawValue: query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased())
        }

        var panelHeaderTitle: String {
            switch self {
            case .all:
                return "ALL"
            case .pulse:
                return "PULSE"
            }
        }

        var descriptionText: String {
            switch self {
            case .all:
                return "all available packages"
            case .pulse:
                return "recently updated packages"
            }
        }

        var paletteItem: CommandPaletteItem {
            CommandPaletteItem(token: rawValue, description: descriptionText)
        }

        static func matchingItems(filter: String) -> [CommandPaletteItem] {
            let normalized = filter.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
            return allCases
                .filter { command in
                    normalized.isEmpty
                        || command.rawValue.contains(normalized)
                        || command.descriptionText.contains(normalized)
                }
                .map(\.paletteItem)
        }
    }

    private enum PaletteMode: Equatable {
        case installed
        case search(query: String)
        case commandBrowser(filter: String)
        case command(PaletteCommand)

        init(searchQuery: String, commandPaletteFocused: Bool) {
            let trimmed = searchQuery.trimmingCharacters(in: .whitespacesAndNewlines)
            if trimmed.isEmpty {
                self = commandPaletteFocused ? .commandBrowser(filter: "") : .installed
            } else if trimmed.hasPrefix(">") {
                let commandQuery = String(trimmed.dropFirst())
                if let command = PaletteCommand(query: commandQuery) {
                    self = .command(command)
                } else {
                    self = .commandBrowser(filter: commandQuery)
                }
            } else {
                self = .search(query: trimmed)
            }
        }
    }

    private final class SmokeOverlayView: NSView {
        override init(frame frameRect: NSRect) {
            super.init(frame: frameRect)
            wantsLayer = true
            layer = CALayer()
            layer?.masksToBounds = false
            layer?.zPosition = 100
        }

        required init?(coder: NSCoder) {
            fatalError("init(coder:) has not been implemented")
        }

        override var isFlipped: Bool {
            true
        }

        override func hitTest(_ point: NSPoint) -> NSView? {
            nil
        }
    }

    private final class RootView: NSView {
        let headerLayer = CATextLayer()
        let commandPalette = CommandPaletteView(frame: .zero)
        let separatorLayer = CALayer()
        let statusLabel = NSTextField(labelWithString: "")
        let updateButton = MastheadButton(title: "UPDATE ALL", target: nil, action: nil)
        let hazardSmokeOverlay = SmokeOverlayView(frame: .zero)
        weak var keyDelegate: RootViewController?

        override init(frame frameRect: NSRect) {
            super.init(frame: frameRect)
            wantsLayer = true
            layer = CALayer()
            layer?.backgroundColor = UIStyle.background.cgColor

            headerLayer.contentsScale = NSScreen.main?.backingScaleFactor ?? 2
            headerLayer.alignmentMode = .left
            headerLayer.isWrapped = false
            headerLayer.truncationMode = .end
            headerLayer.masksToBounds = true
            layer?.addSublayer(headerLayer)
            separatorLayer.backgroundColor = UIStyle.separator.cgColor
            layer?.addSublayer(separatorLayer)

            configureLabel(statusLabel)
            addSubview(commandPalette)
            addSubview(statusLabel)

            updateButton.font = UIStyle.monoFont(size: 11, weight: .medium)
            updateButton.palette = MastheadButton.Palette(
                baseChrome: UIStyle.ControlChrome(
                    topBackgroundColor: UIStyle.accent.withAlphaComponent(0.05),
                    bottomBackgroundColor: UIStyle.accent.withAlphaComponent(0.02),
                    borderColor: UIStyle.accent.withAlphaComponent(0.28),
                    contentColor: UIStyle.accent,
                    topInnerStrokeColor: NSColor.white.withAlphaComponent(0.10),
                    bottomInnerStrokeColor: NSColor.black.withAlphaComponent(0.18)
                ),
                hoverChrome: UIStyle.ControlChrome(
                    topBackgroundColor: UIStyle.accent.withAlphaComponent(0.14),
                    bottomBackgroundColor: UIStyle.accent.withAlphaComponent(0.09),
                    borderColor: UIStyle.accent.withAlphaComponent(0.54),
                    contentColor: UIStyle.text,
                    topInnerStrokeColor: NSColor.white.withAlphaComponent(0.14),
                    bottomInnerStrokeColor: NSColor.black.withAlphaComponent(0.20)
                ),
                disabledChrome: UIStyle.ControlChrome(
                    topBackgroundColor: UIStyle.accent.withAlphaComponent(0.03),
                    bottomBackgroundColor: UIStyle.accent.withAlphaComponent(0.015),
                    borderColor: UIStyle.accent.withAlphaComponent(0.08),
                    contentColor: UIStyle.accent.withAlphaComponent(0.38),
                    topInnerStrokeColor: NSColor.white.withAlphaComponent(0.04),
                    bottomInnerStrokeColor: NSColor.black.withAlphaComponent(0.12)
                )
            )
            updateButton.isHidden = true
            addSubview(updateButton)
        }

        required init?(coder: NSCoder) {
            fatalError("init(coder:) has not been implemented")
        }

        override var acceptsFirstResponder: Bool {
            true
        }

        override var mouseDownCanMoveWindow: Bool {
            true
        }

        override func keyDown(with event: NSEvent) {
            guard keyDelegate?.handleKeyDown(event) == true else {
                super.keyDown(with: event)
                return
            }
        }

        private func configureLabel(_ label: NSTextField) {
            label.isEditable = false
            label.isBordered = false
            label.drawsBackground = false
            label.backgroundColor = .clear
            label.lineBreakMode = .byTruncatingTail
            label.maximumNumberOfLines = 1
            label.usesSingleLineMode = true
            label.allowsDefaultTighteningForTruncation = true
        }
    }

    private let bridge = NucleusBridge()
    private let helperBridge = NukeHelperBridge()
    private let statusStore = NucleusStatusStore()
    private lazy var appUpdateCoordinator = AppUpdateCoordinator(statusStore: statusStore)
    private let packageScrollView = NSScrollView()
    private let packageFieldView = PackageFieldView(frame: .zero)
    private let dossierScrollView = NSScrollView()
    private let dossierView = DossierView(frame: .zero)
    private let externalSurfaceView = ExternalSurfaceView(frame: .zero)
    private let appVersion = Bundle.main.object(
        forInfoDictionaryKey: "CFBundleShortVersionString"
    ) as? String ?? "0.0.0"
    private let codeSignatureHash = RootViewController.abbreviatedCodeSignatureHash()
    private var updateOverlayController: UpdateProgressViewController?
    private var statusAnimator: HeaderGlitchTextAnimator?
    private var installedRecords: [PackageRecord] = []
    private var outdatedPackagesByName: [String: OutdatedPackageRecord] = [:]
    private var installedPackages: [PackagePresentation] = []
    private var recommendations: [PackagePresentation] = []
    private var homebrewMigrationRecommendation: HomebrewMigrationRecommendation?
    private var areRecommendationsVisibleInInstalledList = false
    private var searchResults: [PackagePresentation] = []
    private var searchResultsQuery: String?
    private var commandResults: [PackagePresentation] = []
    private var commandResultsCommand: PaletteCommand?
    private var visiblePackages: [PackagePresentation] = []
    private var detailsByPackageName: [String: PackageDetail] = [:]
    private var selectedItemID: String?
    private var reloadRequestID = 0
    private var searchRequestID = 0
    private var detailRequestID = 0
    private var homebrewMigrationRequestID = 0
    private var loadingDetailItemID: String?
    private var activeOverlayOperationID = 0
    private var snapshotObserver: NSObjectProtocol?
    private var packageScrollObserver: NSObjectProtocol?
    private var isReloadingPackages = false
    private var pendingPackageReload = false
    private var matchingInstalledPackages: [PackagePresentation] = []
    private var searchExcludedPackageNames: Set<String> = []
    private var totalDiscoveryCount = 0
    private var searchNextOffset: Int?
    private var commandNextOffset: Int?
    private var commandTotalCount = 0
    private var isSearching = false {
        didSet {
            updateHeader()
            updatePaneLoadingIndicators()
        }
    }
    private var isLoadingMoreSearchResults = false {
        didSet {
            updateHeader()
            updatePaneLoadingIndicators()
        }
    }
    private var isLoadingCommandResults = false {
        didSet {
            updateHeader()
            updatePaneLoadingIndicators()
        }
    }
    private var isLoadingMoreCommandResults = false {
        didSet {
            updateHeader()
            updatePaneLoadingIndicators()
        }
    }
    private var isRunningPrivilegedUpdate = false {
        didSet {
            updateHeader()
            updateUpdateButtonVisibility()
        }
    }
    private var isInstallingAv = false {
        didSet {
            updateHeader()
            updateUpdateButtonVisibility()
            refreshUpdateAvailability()
        }
    }
    private var isRunningPackageOperation = false {
        didSet {
            updateHeader()
            updateUpdateButtonVisibility()
            dossierView.setActionInFlight(isRunningPackageOperation)
        }
    }
    private var isLoadingSelectedPackageDetail = false {
        didSet {
            updateHeader()
            updatePaneLoadingIndicators()
        }
    }
    private var hasUpdatesAvailable = false {
        didSet { updateUpdateButtonVisibility() }
    }
    private var searchQuery = "" {
        didSet {
            syncCommandPaletteText()
            updateHeader()
            reloadVisiblePackagesForSearch()
        }
    }
    private var isCommandPaletteFocused = false {
        didSet {
            guard isCommandPaletteFocused != oldValue else { return }
            updateHeader()
            reloadVisiblePackagesForSearch()
        }
    }

    override func loadView() {
        view = RootView(frame: NSRect(x: 0, y: 0, width: 1380, height: 860))
    }

    deinit {
        if let packageScrollObserver {
            NotificationCenter.default.removeObserver(packageScrollObserver)
        }
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        guard let rootView = view as? RootView else { return }
        rootView.keyDelegate = self
        rootView.commandPalette.onActivate = { [weak self] in
            self?.setCommandPaletteFocused(true)
        }
        rootView.commandPalette.field.delegate = self
        rootView.updateButton.target = self
        rootView.updateButton.action = #selector(beginUpdateFlow)
        appUpdateCoordinator.onStateChange = { [weak self] in
            self?.refreshUpdateAvailability()
        }
        appUpdateCoordinator.onError = { [weak self] message in
            self?.presentAppUpdateError(message)
        }
        statusAnimator = HeaderGlitchTextAnimator(
            field: rootView.statusLabel,
            size: 10,
            baseColor: UIStyle.accent.withAlphaComponent(0.86),
            glitchColor: UIStyle.text.withAlphaComponent(0.88),
            weight: .medium,
            tracking: 0.9
        )
        dossierView.delegate = self
        packageFieldView.delegate = self
        packageScrollView.drawsBackground = false
        packageScrollView.hasVerticalScroller = true
        packageScrollView.hasHorizontalScroller = false
        packageScrollView.autohidesScrollers = true
        packageScrollView.scrollerStyle = .overlay
        packageScrollView.borderType = .noBorder
        packageScrollView.contentView.postsBoundsChangedNotifications = true
        packageScrollView.documentView = packageFieldView
        dossierScrollView.drawsBackground = false
        dossierScrollView.hasVerticalScroller = true
        dossierScrollView.hasHorizontalScroller = false
        dossierScrollView.autohidesScrollers = true
        dossierScrollView.scrollerStyle = .overlay
        dossierScrollView.borderType = .noBorder
        dossierScrollView.documentView = dossierView
        view.addSubview(packageScrollView)
        view.addSubview(dossierScrollView)
        view.addSubview(externalSurfaceView)
        rootView.addSubview(rootView.hazardSmokeOverlay, positioned: .above, relativeTo: nil)
        if let smokeLayer = rootView.hazardSmokeOverlay.layer {
            packageFieldView.installHazardSmoke(
                in: smokeLayer,
                coordinateView: rootView.hazardSmokeOverlay
            )
        }
        updatePaneLoadingIndicators()
        updateHeader()
        refreshRecommendations()
        installSnapshotObserverIfNeeded()
        installPackageScrollObserverIfNeeded()
        applyStatusSnapshot(statusStore.loadSnapshot())
        reloadPackages()
        statusStore.requestRefresh()
        appUpdateCoordinator.startAutomaticChecks()
    }

    override var acceptsFirstResponder: Bool {
        true
    }

    func controlTextDidBeginEditing(_ obj: Notification) {
        guard let rootView = view as? RootView,
              let field = obj.object as? NSTextField,
              field === rootView.commandPalette.field else {
            return
        }
        applyCommandPaletteEditorTint(for: field)
        rootView.commandPalette.isActive = true
        setCommandPaletteFocused(true)
    }

    func controlTextDidEndEditing(_ obj: Notification) {
        guard let rootView = view as? RootView,
              let field = obj.object as? NSTextField,
              field === rootView.commandPalette.field else {
            return
        }
        rootView.commandPalette.isActive = false
        setCommandPaletteFocused(false)
    }

    func controlTextDidChange(_ obj: Notification) {
        guard let rootView = view as? RootView,
              let field = obj.object as? NSTextField,
              field === rootView.commandPalette.field else {
            return
        }

        let normalized = normalizedSearchText(field.stringValue)
        if normalized != field.stringValue {
            let selectedRange = (field.currentEditor() as? NSTextView)?.selectedRange()
                ?? NSRange(location: normalized.utf16.count, length: 0)
            field.stringValue = normalized
            (field.currentEditor() as? NSTextView)?.setSelectedRange(NSRange(
                location: min(selectedRange.location, normalized.utf16.count),
                length: 0
            ))
        }
        if searchQuery != normalized {
            searchQuery = normalized
        } else {
            rootView.commandPalette.needsLayout = true
        }
    }

    func control(
        _ control: NSControl,
        textView: NSTextView,
        doCommandBy commandSelector: Selector
    ) -> Bool {
        if commandSelector == #selector(moveDown(_:)) {
            guard moveSelection(.down) else { return false }
            view.window?.makeFirstResponder(view)
            return true
        }
        if commandSelector == #selector(cancelOperation(_:)) {
            if searchQuery.isEmpty {
                view.window?.makeFirstResponder(view)
            } else {
                searchQuery = ""
            }
            return true
        }
        return false
    }

    override func viewDidLayout() {
        super.viewDidLayout()
        guard let rootView = view as? RootView else { return }
        let bounds = view.bounds
        let topPadding: CGFloat = 12
        let topLabelHeight: CGFloat = 16
        let searchFieldHeight: CGFloat = 24
        let topBarGap: CGFloat = 10
        let contentTopGap: CGFloat = 0
        let gutter: CGFloat = 4
        let headerY = bounds.height - topPadding - topLabelHeight
        let separatorY = headerY - topBarGap
        let contentTop = separatorY - contentTopGap
        let contentHeight = max(contentTop, 120)
        let horizontalPadding: CGFloat = 16
        let titlebarLeadingInset: CGFloat = 86
        let mastheadGap: CGFloat = 24
        let clusterGap: CGFloat = 10
        let headerPreferredWidth: CGFloat = 360
        let updateButtonWidth: CGFloat = rootView.updateButton.isHidden ? 0 : 112
        let updateButtonHeight: CGFloat = rootView.updateButton.isHidden ? 0 : 24
        let searchPreferredWidth: CGFloat = 340
        let searchMinWidth: CGFloat = 156
        let leftClusterX = titlebarLeadingInset
        let rightClusterWidth = max(
            searchMinWidth,
            searchPreferredWidth
        ) + updateButtonWidth + (rootView.updateButton.isHidden ? 0 : clusterGap)
        let leftClusterAvailableWidth = max(
            bounds.width
                - leftClusterX
                - horizontalPadding
                - rightClusterWidth
                - mastheadGap * 2,
            0
        )
        let measuredHeaderWidth = min(
            headerPreferredWidth,
            attributedTextWidth(mastheadAttributedText())
        )
        let resolvedHeaderWidth = min(measuredHeaderWidth, leftClusterAvailableWidth)
        let leftClusterMaxX = leftClusterX + resolvedHeaderWidth
        let statusText = activityStatusText()
        let statusMeasuredWidth = statusText.map(statusTextWidth) ?? 0
        let rightControlsWidth = updateButtonWidth
            + (rootView.updateButton.isHidden ? 0 : clusterGap)
        let searchAndStatusAvailableWidth = max(
            bounds.width
                - horizontalPadding
                - mastheadGap
                - leftClusterMaxX
                - mastheadGap
                - rightControlsWidth,
            0
        )
        let statusAvailableWidth = max(
            searchAndStatusAvailableWidth - searchMinWidth,
            0
        )
        let statusWidth = min(statusMeasuredWidth, statusAvailableWidth)
        let searchWidth = min(
            searchPreferredWidth,
            max(searchAndStatusAvailableWidth - statusWidth, 0)
        )
        let searchX = bounds.width - horizontalPadding - searchWidth
        let updateButtonX = searchX - (rootView.updateButton.isHidden ? 0 : clusterGap) - updateButtonWidth
        let statusX = updateButtonX - mastheadGap - statusWidth
        let searchRowY = headerY + Self.rightMastheadSearchYOffset
        let statusRowY = headerY + Self.rightMastheadStatusYOffset
        let updateButtonY = headerY + Self.rightMastheadUpdateButtonYOffset
        let usableWidth = max(bounds.width - gutter * 2, 0)
        let leftWidth = floor(usableWidth * 0.46)
        let middleWidth = floor(usableWidth * 0.22)
        let rightWidth = max(usableWidth - leftWidth - middleWidth, 0)

        updatePackageScrollLayout(
            x: 0,
            y: 0,
            width: leftWidth,
            height: contentHeight
        )
        dossierScrollView.frame = CGRect(
            x: packageScrollView.frame.maxX + gutter,
            y: 0,
            width: middleWidth,
            height: contentHeight
        )
        dossierView.frame = CGRect(
            x: 0,
            y: 0,
            width: middleWidth,
            height: contentHeight
        )
        externalSurfaceView.frame = CGRect(
            x: dossierScrollView.frame.maxX + gutter,
            y: 0,
            width: rightWidth,
            height: contentTop
        )

        CATransaction.begin()
        CATransaction.setDisableActions(true)
        rootView.headerLayer.frame = CGRect(
            x: leftClusterX,
            y: headerY + Self.leftMastheadTitleYOffset,
            width: resolvedHeaderWidth,
            height: topLabelHeight
        )
        rootView.commandPalette.frame = CGRect(
            x: searchX,
            y: searchRowY - 5,
            width: searchWidth,
            height: searchFieldHeight
        )
        rootView.statusLabel.frame = CGRect(
            x: statusX,
            y: statusRowY,
            width: statusWidth,
            height: topLabelHeight
        )
        rootView.updateButton.frame = CGRect(
            x: updateButtonX,
            y: updateButtonY,
            width: updateButtonWidth,
            height: updateButtonHeight
        )
        rootView.hazardSmokeOverlay.frame = bounds
        rootView.separatorLayer.frame = CGRect(
            x: horizontalPadding,
            y: separatorY,
            width: bounds.width - horizontalPadding * 2,
            height: 1
        )
        CATransaction.commit()
        packageFieldView.refreshHazardSmokeLayout()
        updateOverlayController?.view.frame = bounds
    }

    func handleKeyDown(_ event: NSEvent) -> Bool {
        if event.modifierFlags.contains(.command), event.charactersIgnoringModifiers == "p" {
            focusCommandPalette()
            return true
        }

        if event.modifierFlags.contains(.command), event.charactersIgnoringModifiers == "r" {
            reloadPackages()
            return true
        }

        switch event.keyCode {
        case 123:
            return moveSelection(.left)
        case 124:
            return moveSelection(.right)
        case 125:
            return moveSelection(.down)
        case 126:
            if moveSelection(.up) {
                return true
            }
            focusCommandPalette(selectExistingText: false)
            return true
        case 53:
            return clearSelection()
        case 51:
            guard !searchQuery.isEmpty else {
                return false
            }
            focusCommandPalette(selectExistingText: false)
            searchQuery.removeLast()
            return true
        default:
            guard let characters = event.charactersIgnoringModifiers?
                .trimmingCharacters(in: .controlCharacters),
                !characters.isEmpty,
                event.modifierFlags.intersection([.command, .control, .option]).isEmpty else {
                return false
            }
            focusCommandPalette(selectExistingText: false)
            insertSearchText(characters)
            return true
        }
    }

    @objc func copy(_ sender: Any?) {
        guard !searchQuery.isEmpty else { return }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(searchQuery, forType: .string)
    }

    @objc func cut(_ sender: Any?) {
        guard !searchQuery.isEmpty else { return }
        copy(sender)
        searchQuery = ""
    }

    @objc func paste(_ sender: Any?) {
        guard let text = NSPasteboard.general.string(forType: .string) else {
            return
        }
        insertSearchText(text)
    }

    @objc override func selectAll(_ sender: Any?) {
        guard !searchQuery.isEmpty else { return }
        copy(sender)
    }

    @objc override func deleteBackward(_ sender: Any?) {
        guard !searchQuery.isEmpty else { return }
        searchQuery.removeLast()
    }

    func validateUserInterfaceItem(
        _ item: any NSValidatedUserInterfaceItem
    ) -> Bool {
        switch item.action {
        case #selector(copy(_:)),
             #selector(cut(_:)),
             #selector(selectAll(_:)),
             #selector(deleteBackward(_:)):
            return !searchQuery.isEmpty
        case #selector(paste(_:)):
            return NSPasteboard.general.canReadObject(
                forClasses: [NSString.self],
                options: nil
            )
        default:
            return true
        }
    }

    func packageFieldView(_ view: PackageFieldView, didSelect package: PackagePresentation) {
        select(itemID: package.selectionID)
    }

    func dossierView(_ view: DossierView, didRequestPrimaryActionFor detail: PackageDetail) {
        beginPackageMutation(for: detail)
    }

    func dossierView(_ view: DossierView, didRequestUpdateActionFor detail: PackageDetail) {
        beginPackageUpdate(for: detail)
    }

    func dossierView(_ view: DossierView, didRequestSecurityActionFor detail: PackageDetail) {
        beginSecurityMutation(for: detail)
    }

    func requestRefresh() {
        reloadPackages()
    }

    private func resetDossierScrollPosition() {
        dossierScrollView.contentView.scroll(to: .zero)
        dossierScrollView.reflectScrolledClipView(dossierScrollView.contentView)
    }

    private func reloadPackages() {
        if isReloadingPackages {
            pendingPackageReload = true
            return
        }

        isReloadingPackages = true
        pendingPackageReload = false
        areRecommendationsVisibleInInstalledList = false
        let requestID = reloadRequestID + 1
        reloadRequestID = requestID
        homebrewMigrationRecommendation = nil
        installedRecords = []
        installedPackages = []
        refreshRecommendations()
        applyStatusSnapshot(statusStore.loadSnapshot())
        reloadVisiblePackagesForSearch()
        loadHomebrewMigrationRecommendation(requestID: requestID)

        DispatchQueue.global(qos: .userInitiated).async {
            do {
                let records = try self.bridge.fetchPackages().sorted {
                    let left = $0.name.packageSearchOrderName
                    let right = $1.name.packageSearchOrderName
                    if left == right {
                        return $0.name < $1.name
                    }
                    return left < right
                }
                DispatchQueue.main.async {
                    guard self.reloadRequestID == requestID else { return }
                    self.installedRecords = records
                    self.refreshInstalledPackages()
                    self.refreshRecommendations()
                    self.statusStore.requestRefresh()
                    if self.finishPackageReload() {
                        self.revealRecommendationsAfterInstalledLoad(requestID: requestID)
                    }
                }
            } catch {
                DispatchQueue.main.async {
                    guard self.reloadRequestID == requestID else { return }
                    self.installedRecords = []
                    self.refreshInstalledPackages()
                    self.refreshRecommendations()
                    self.statusStore.requestRefresh()
                    if self.finishPackageReload() {
                        self.revealRecommendationsAfterInstalledLoad(requestID: requestID)
                    }
                }
            }
        }
    }

    private func finishPackageReload() -> Bool {
        isReloadingPackages = false
        if pendingPackageReload {
            pendingPackageReload = false
            reloadPackages()
            return false
        }
        return true
    }

    private func revealRecommendationsAfterInstalledLoad(requestID: Int) {
        DispatchQueue.main.async { [weak self] in
            guard let self else { return }
            guard self.reloadRequestID == requestID else { return }
            guard self.isReloadingPackages == false else { return }
            self.areRecommendationsVisibleInInstalledList = true
            self.refreshRecommendations()
        }
    }

    private func loadHomebrewMigrationRecommendation(requestID: Int) {
        let migrationRequestID = homebrewMigrationRequestID + 1
        homebrewMigrationRequestID = migrationRequestID
        DispatchQueue.global(qos: .utility).async {
            do {
                let recommendation = try self.bridge.fetchHomebrewMigrationRecommendation()
                DispatchQueue.main.async {
                    guard self.reloadRequestID == requestID,
                          self.homebrewMigrationRequestID == migrationRequestID else {
                        return
                    }
                    self.homebrewMigrationRecommendation = recommendation.packages.isEmpty
                        ? nil
                        : recommendation
                    self.refreshRecommendations()
                }
            } catch {
                DispatchQueue.main.async {
                    guard self.reloadRequestID == requestID,
                          self.homebrewMigrationRequestID == migrationRequestID else {
                        return
                    }
                    self.homebrewMigrationRecommendation = nil
                    self.refreshRecommendations()
                }
            }
        }
    }

    private func refreshDockBadge() {
        let outdatedCount = outdatedPackagesByName.count
        NSApp.dockTile.badgeLabel = outdatedCount > 0 ? String(outdatedCount) : nil
        NSApp.dockTile.display()
    }

    private func installSnapshotObserverIfNeeded() {
        guard snapshotObserver == nil else { return }
        snapshotObserver = statusStore.observeSnapshotChanges { [weak self] _ in
            guard let self else { return }
            self.applyStatusSnapshot(self.statusStore.loadSnapshot())
        }
    }

    private func installPackageScrollObserverIfNeeded() {
        guard packageScrollObserver == nil else { return }
        packageScrollObserver = NotificationCenter.default.addObserver(
            forName: NSView.boundsDidChangeNotification,
            object: packageScrollView.contentView,
            queue: .main
        ) { [weak self] _ in
            self?.loadNextSearchPageIfNeeded()
            self?.packageFieldView.refreshHazardSmokeLayout()
        }
    }

    private func applyStatusSnapshot(_ snapshot: NucleusStatusSnapshot) {
        outdatedPackagesByName = snapshot.outdatedPackagesByName
        detailsByPackageName = detailsByPackageName.mapValues { detail in
            normalizedDetail(detail)
        }
        refreshRecommendations()
        refreshDockBadge()
        refreshInstalledPackages()
        refreshUpdateAvailability()
        if let selectedPackageName = selectedItemID,
           let detail = detailsByPackageName[selectedPackageName] {
            dossierView.render(detail: detail, animation: .none)
            externalSurfaceView.render(detail: detail, animated: false)
        }
    }

    private func normalizedDetail(_ detail: PackageDetail) -> PackageDetail {
        detail.applying(outdated: outdatedPackagesByName[detail.packageName])
    }

    private func refreshInstalledPackages() {
        rebuildInstalledPackages()
        reloadVisiblePackagesForSearch()
    }

    private func rebuildInstalledPackages() {
        installedPackages = installedRecords.map { record in
            let mergedRecord: PackageRecord
            if let outdated = outdatedPackagesByName[record.name] {
                mergedRecord = record.applying(outdated: outdated)
            } else {
                mergedRecord = record
            }

            return PackagePresentation(
                item: .installed(mergedRecord),
                detail: detailsByPackageName[record.name],
                freshness: freshness(for: record.name)
            )
        }
    }

    private func refreshCurrentVisiblePackageDetails() {
        rebuildInstalledPackages()
        searchResults = searchResults.map(packagePresentationWithCachedDetail)
        commandResults = commandResults.map(packagePresentationWithCachedDetail)

        switch paletteMode {
        case .installed:
            applyVisiblePackages(installedPalettePackages)
        case .search(let query):
            matchingInstalledPackages = installedPackages.filter {
                ($0.packageName ?? "").localizedCaseInsensitiveContains(query)
            }
            applyVisiblePackages(matchingInstalledPackages + searchResults)
        case .commandBrowser(let filter):
            applyVisiblePackages(commandPaletteItems(filter: filter))
        case .command:
            applyVisiblePackages(commandResults)
        }
    }

    private func packagePresentationWithCachedDetail(
        _ package: PackagePresentation
    ) -> PackagePresentation {
        PackagePresentation(
            item: package.item,
            detail: detailsByPackageName[package.selectionID],
            freshness: package.freshness
        )
    }

    private func refreshRecommendations() {
        let installedPackageNames = installedRecommendationPackageNames()
        let toolingPackMissingPackageNames =
            PackageRecommendation.agenticToolingPackPackageNames.filter {
                installedPackageNames.contains($0) == false
            }

        let activeRecommendations = [
            bridge.cliToolsRecommendation(),
            bridge.xcodeCLTRecommendation(),
            PackageRecommendation.agenticToolingPack(
                missingPackageNames: toolingPackMissingPackageNames
            ),
            homebrewMigrationRecommendation.flatMap(PackageRecommendation.homebrewMigration)
        ].compactMap { $0 }

        let activeRecommendationNames = Set(activeRecommendations.map(\.packageName))
        recommendations = activeRecommendations.map { recommendation in
            PackagePresentation(
                item: .recommendation(recommendation),
                detail: recommendation.detail,
                freshness: freshness(for: recommendation.detail.packageName)
            )
        }
        [
            PackageRecommendation.automicVaultCLTName,
            PackageRecommendation.xcodeCLTName,
            PackageRecommendation.agenticToolingPackName,
            PackageRecommendation.homebrewMigrationName
        ]
        .filter { activeRecommendationNames.contains($0) == false }
        .forEach { detailsByPackageName.removeValue(forKey: $0) }
        for package in recommendations {
            if let detail = package.detail {
                detailsByPackageName[package.selectionID] = detail
            }
        }
        reloadVisiblePackagesForSearch()
    }

    private var installedPalettePackages: [PackagePresentation] {
        if areRecommendationsVisibleInInstalledList {
            return installedPackages + recommendations
        }
        return installedPackages
    }

    private func installedRecommendationPackageNames() -> Set<String> {
        Set(installedRecords.flatMap { record in
            var names = [record.name]
            if let source = record.source, case .formula(let rootFormula) = source {
                names.append(rootFormula)
            }
            if record.name.hasPrefix("brew:") {
                names.append(String(record.name.dropFirst("brew:".count)))
            }
            return names
        })
    }

    private func select(
        itemID: String,
        lazyLoadOnly: Bool = false,
        updateFieldView: Bool = true
    ) {
        selectedItemID = itemID
        resetDossierScrollPosition()
        if updateFieldView {
            packageFieldView.apply(
                packages: visiblePackages,
                selectedPackageName: itemID,
                searchQuery: searchQuery,
                secondarySectionTitle: packageSecondarySectionTitle,
                secondarySectionCount: packageSecondarySectionCount,
                panelHeaderTitle: packagePanelHeaderTitle,
                panelHeaderCount: packagePanelHeaderCount,
                commandPaletteHelpText: commandPaletteHelpText,
                commandPaletteQuoteText: commandPaletteQuoteText
            )
        }
        if let detail = detailsByPackageName[itemID] {
            loadingDetailItemID = nil
            isLoadingSelectedPackageDetail = false
            dossierView.render(
                detail: detail,
                animation: lazyLoadOnly ? .none : .full
            )
            externalSurfaceView.render(detail: detail, animated: !lazyLoadOnly)
            return
        }

        guard let package = visiblePackages.first(where: { $0.selectionID == itemID }) else {
            loadingDetailItemID = nil
            isLoadingSelectedPackageDetail = false
            dossierView.render(detail: nil, animation: .none)
            externalSurfaceView.render(detail: nil, animated: !lazyLoadOnly)
            return
        }

        if let commandQueryText = package.commandQueryText {
            loadingDetailItemID = nil
            isLoadingSelectedPackageDetail = false
            dossierView.render(detail: nil, animation: .none)
            externalSurfaceView.render(detail: nil, animated: false)
            if searchQuery != commandQueryText {
                searchQuery = commandQueryText
            }
            return
        }

        let fallbackDetail: PackageDetail
        switch package.item {
        case .installed(let record):
            fallbackDetail = record.fallbackDetail
        case .recommendation(let recommendation):
            fallbackDetail = recommendation.detail
        case .available(let result):
            fallbackDetail = result.fallbackDetail
        case .command:
            return
        }
        let isAwaitingSelectedDetail = isLoadingSelectedPackageDetail
            && loadingDetailItemID == itemID
        dossierView.render(
            detail: fallbackDetail,
            animation: lazyLoadOnly ? .none : .full
        )
        externalSurfaceView.render(
            detail: fallbackDetail,
            animated: !lazyLoadOnly,
            loading: !lazyLoadOnly || isAwaitingSelectedDetail
        )

        if lazyLoadOnly {
            if isAwaitingSelectedDetail == false {
                loadingDetailItemID = nil
                isLoadingSelectedPackageDetail = false
            }
            return
        }

        let requestID = detailRequestID + 1
        detailRequestID = requestID
        loadingDetailItemID = itemID
        isLoadingSelectedPackageDetail = true

        DispatchQueue.global(qos: .userInitiated).async {
            do {
                let detail: PackageDetail
                switch package.item {
                case .installed:
                    detail = try self.bridge.fetchDetail(
                        packageName: package.packageName ?? itemID
                    )
                case .recommendation(let recommendation):
                    detail = recommendation.detail
                case .available(let result):
                    detail = try self.bridge.fetchDetail(
                        packageName: result.detailLookupName
                    )
                case .command:
                    return
                }
                DispatchQueue.main.async {
                    guard self.detailRequestID == requestID else { return }
                    let normalizedDetail = self.normalizedDetail(detail)
                    self.detailsByPackageName[itemID] = normalizedDetail
                    self.refreshCurrentVisiblePackageDetails()
                    self.loadingDetailItemID = nil
                    self.isLoadingSelectedPackageDetail = false
                    if self.selectedItemID == itemID {
                        self.dossierView.render(
                            detail: normalizedDetail,
                            animation: .incremental
                        )
                        self.externalSurfaceView.render(
                            detail: normalizedDetail,
                            animated: true
                        )
                    }
                }
            } catch {
                DispatchQueue.main.async {
                    guard self.detailRequestID == requestID else { return }
                    self.loadingDetailItemID = nil
                    self.isLoadingSelectedPackageDetail = false
                    if self.selectedItemID == itemID {
                        self.dossierView.render(detail: fallbackDetail, animation: .none)
                        self.externalSurfaceView.render(detail: fallbackDetail, animated: false)
                    }
                }
            }
        }
    }

    private func freshness(for packageName: String) -> CGFloat {
        let hash = CGFloat(abs(packageName.hashValue % 1000)) / 1000
        return 0.28 + hash * 0.72
    }

    private func commandPaletteItems(filter: String) -> [PackagePresentation] {
        PaletteCommand.matchingItems(filter: filter).map { command in
            PackagePresentation(
                item: .command(command),
                detail: nil,
                freshness: freshness(for: command.selectionID)
            )
        }
    }

    private var paletteMode: PaletteMode {
        PaletteMode(
            searchQuery: searchQuery,
            commandPaletteFocused: isCommandPaletteFocused
        )
    }

    private var commandPaletteHelpText: NSAttributedString? {
        guard isCommandPaletteFocused,
              searchQuery.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            return nil
        }
        return Self.commandPaletteHelpText()
    }

    private var commandPaletteQuoteText: NSAttributedString? {
        guard isCommandPaletteFocused,
              searchQuery.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            return nil
        }
        return Self.commandPaletteQuoteText()
    }

    private static func commandPaletteHelpText() -> NSAttributedString {
        let rendered = NSMutableAttributedString()
        rendered.append(UIStyle.attributedMonoText(
            "Type to search installed tools and the vault catalog.",
            size: 12,
            color: UIStyle.dimText,
            weight: .regular,
            tracking: 0.15
        ))
        rendered.append(NSAttributedString(string: "\n"))
        rendered.append(UIStyle.attributedMonoText(
            "Begin with > for commands.",
            size: 12,
            color: UIStyle.text.withAlphaComponent(0.56),
            weight: .regular,
            tracking: 0.1
        ))
        return rendered
    }

    private static func commandPaletteQuoteText() -> NSAttributedString {
        UIStyle.attributedMonoText(
            "Press ESCAPE to show installed packages",
            size: 12,
            color: UIStyle.accent.withAlphaComponent(0.72),
            weight: .light,
            tracking: 0.2
        )
    }

    private var isShowingEmptyCommandPaletteHelp: Bool {
        isCommandPaletteFocused
            && searchQuery.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    private var packagePanelHeaderTitle: String {
        switch paletteMode {
        case .installed:
            return "INSTALLED"
        case .search:
            return matchingInstalledPackages.isEmpty ? "DISCOVERY" : "INSTALLED"
        case .commandBrowser:
            if isShowingEmptyCommandPaletteHelp {
                return "COMMAND PALETTE"
            }
            return "AVAILABLE COMMANDS"
        case .command(let command):
            return command.panelHeaderTitle
        }
    }

    private var packagePanelHeaderCount: Int? {
        switch paletteMode {
        case .installed:
            return nil
        case .search:
            return matchingInstalledPackages.isEmpty ? totalDiscoveryCount : nil
        case .commandBrowser:
            return nil
        case .command:
            return commandTotalCount
        }
    }

    private var packageSecondarySectionTitle: String {
        switch paletteMode {
        case .installed:
            return "RECOMMENDATIONS"
        case .search, .command, .commandBrowser:
            return "DISCOVERY"
        }
    }

    private var packageSecondarySectionCount: Int? {
        switch paletteMode {
        case .installed:
            guard areRecommendationsVisibleInInstalledList else { return nil }
            return recommendations.isEmpty ? nil : recommendations.count
        case .search:
            return totalDiscoveryCount
        case .command, .commandBrowser:
            return nil
        }
    }

    private func reloadVisiblePackagesForSearch() {
        let requestID = searchRequestID + 1
        searchRequestID = requestID
        switch paletteMode {
        case .installed:
            searchResults = []
            searchResultsQuery = nil
            commandResults = []
            commandResultsCommand = nil
            matchingInstalledPackages = []
            searchExcludedPackageNames = []
            totalDiscoveryCount = 0
            searchNextOffset = nil
            commandNextOffset = nil
            commandTotalCount = 0
            isSearching = false
            isLoadingMoreSearchResults = false
            isLoadingCommandResults = false
            isLoadingMoreCommandResults = false
            applyVisiblePackages(installedPalettePackages)
        case .search(let query):
            let retainedSearchResults = searchResultsQuery == query ? searchResults : []
            commandResults = []
            commandResultsCommand = nil
            commandNextOffset = nil
            commandTotalCount = 0
            isLoadingCommandResults = false
            isLoadingMoreCommandResults = false
            matchingInstalledPackages = installedPackages.filter {
                ($0.packageName ?? "").localizedCaseInsensitiveContains(query)
            }
            searchExcludedPackageNames = Set(installedPackages.compactMap(\.packageName))
            searchResults = retainedSearchResults
            searchResultsQuery = retainedSearchResults.isEmpty ? nil : query
            totalDiscoveryCount = 0
            searchNextOffset = nil
            isSearching = true
            isLoadingMoreSearchResults = false
            scrollPackageListToTop()
            applyVisiblePackages(matchingInstalledPackages + retainedSearchResults)
            requestSearchPage(query: query, offset: 0, requestID: requestID)
        case .commandBrowser(let filter):
            searchResults = []
            searchResultsQuery = nil
            commandResults = []
            commandResultsCommand = nil
            matchingInstalledPackages = []
            searchExcludedPackageNames = []
            totalDiscoveryCount = 0
            searchNextOffset = nil
            commandNextOffset = nil
            commandTotalCount = 0
            isSearching = false
            isLoadingMoreSearchResults = false
            isLoadingCommandResults = false
            isLoadingMoreCommandResults = false
            scrollPackageListToTop()
            applyVisiblePackages(commandPaletteItems(filter: filter))
        case .command(let command):
            let retainedCommandResults =
                commandResultsCommand == command ? commandResults : []
            searchResults = []
            searchResultsQuery = nil
            matchingInstalledPackages = []
            searchExcludedPackageNames = []
            totalDiscoveryCount = 0
            searchNextOffset = nil
            isSearching = false
            isLoadingMoreSearchResults = false
            commandResults = retainedCommandResults
            commandResultsCommand = retainedCommandResults.isEmpty ? nil : command
            commandNextOffset = nil
            commandTotalCount = 0
            isLoadingCommandResults = true
            isLoadingMoreCommandResults = false
            scrollPackageListToTop()
            applyVisiblePackages(retainedCommandResults)
            requestCommandPage(command: command, offset: 0, requestID: requestID)
        }
    }

    private func requestSearchPage(query: String, offset: Int, requestID: Int) {
        let excludedPackageNames = searchExcludedPackageNames
        let cachedDetailsByPackageName = detailsByPackageName
        DispatchQueue.global(qos: .userInitiated).async {
            do {
                let page = try self.bridge.fetchSearchResults(
                    query: query,
                    offset: offset,
                    limit: Self.searchPageSize
                )
                let results = page.packages
                    .filter { result in
                        excludedPackageNames.contains(result.name) == false
                    }
                    .map { result in
                        PackagePresentation(
                            item: .available(result),
                            detail: cachedDetailsByPackageName[result.name],
                            freshness: self.freshness(for: result.name)
                        )
                    }

                DispatchQueue.main.async {
                    guard self.searchRequestID == requestID else { return }
                    guard self.paletteMode == .search(query: query) else { return }
                    self.totalDiscoveryCount = max(
                        page.totalCount - self.matchingInstalledPackages.count,
                        0
                    )
                    if offset == 0 {
                        self.isSearching = false
                        self.searchResults = results
                        self.searchResultsQuery = query
                    } else {
                        self.isLoadingMoreSearchResults = false
                        self.searchResults.append(contentsOf: results)
                    }
                    self.searchNextOffset = page.nextOffset
                    self.applyVisiblePackages(self.matchingInstalledPackages + self.searchResults)
                    self.loadNextSearchPageIfNeeded()
                }
            } catch {
                DispatchQueue.main.async {
                    guard self.searchRequestID == requestID else { return }
                    guard self.paletteMode == .search(query: query) else { return }
                    if offset == 0 {
                        self.isSearching = false
                        self.searchResults = []
                        self.searchResultsQuery = nil
                        self.totalDiscoveryCount = 0
                        self.searchNextOffset = nil
                        self.applyVisiblePackages(self.matchingInstalledPackages)
                    } else {
                        self.isLoadingMoreSearchResults = false
                    }
                }
            }
        }
    }

    private func requestCommandPage(command: PaletteCommand, offset: Int, requestID: Int) {
        let cachedDetailsByPackageName = detailsByPackageName
        DispatchQueue.global(qos: .userInitiated).async {
            do {
                let page: PackageSearchPage
                switch command {
                case .all:
                    page = try self.bridge.fetchAvailablePackages(
                        offset: offset,
                        limit: Self.searchPageSize
                    )
                case .pulse:
                    page = try self.bridge.fetchPulsePackages(
                        offset: offset,
                        limit: Self.searchPageSize
                    )
                }
                let results = page.packages.map { result in
                    PackagePresentation(
                        item: .available(result),
                        detail: cachedDetailsByPackageName[result.name],
                        freshness: self.freshness(for: result.name)
                    )
                }

                DispatchQueue.main.async {
                    guard self.searchRequestID == requestID else { return }
                    guard self.paletteMode == .command(command) else { return }
                    self.commandTotalCount = page.totalCount
                    if offset == 0 {
                        self.isLoadingCommandResults = false
                        self.commandResults = results
                        self.commandResultsCommand = command
                    } else {
                        self.isLoadingMoreCommandResults = false
                        self.commandResults.append(contentsOf: results)
                    }
                    self.commandNextOffset = page.nextOffset
                    self.applyVisiblePackages(self.commandResults)
                    self.loadNextSearchPageIfNeeded()
                }
            } catch {
                DispatchQueue.main.async {
                    guard self.searchRequestID == requestID else { return }
                    guard self.paletteMode == .command(command) else { return }
                    if offset == 0 {
                        self.isLoadingCommandResults = false
                        self.commandResults = []
                        self.commandResultsCommand = nil
                        self.commandTotalCount = 0
                        self.commandNextOffset = nil
                        self.applyVisiblePackages([])
                    } else {
                        self.isLoadingMoreCommandResults = false
                    }
                }
            }
        }
    }

    private func loadNextSearchPageIfNeeded() {
        let nextOffset: Int
        switch paletteMode {
        case .installed:
            return
        case .search(let query):
            guard isSearching == false, isLoadingMoreSearchResults == false else {
                return
            }
            guard let offset = searchNextOffset else { return }
            nextOffset = offset
            if query.isEmpty {
                return
            }
        case .command(let command):
            guard isLoadingCommandResults == false,
                  isLoadingMoreCommandResults == false else {
                return
            }
            guard let offset = commandNextOffset else { return }
            nextOffset = offset
            switch command {
            case .all:
                break
            case .pulse:
                break
            }
        case .commandBrowser:
            return
        }
        let visibleRect = packageScrollView.contentView.bounds
        let remainingDistance = packageFieldView.frame.height - visibleRect.maxY
        guard remainingDistance <= Self.searchLoadMoreThreshold else { return }
        switch paletteMode {
        case .installed:
            return
        case .search(let query):
            isLoadingMoreSearchResults = true
            requestSearchPage(
                query: query,
                offset: nextOffset,
                requestID: searchRequestID
            )
        case .commandBrowser:
            return
        case .command(let command):
            isLoadingMoreCommandResults = true
            requestCommandPage(
                command: command,
                offset: nextOffset,
                requestID: searchRequestID
            )
        }
    }

    private func scrollPackageListToTop() {
        packageScrollView.contentView.scroll(to: CGPoint(x: 0, y: 0))
        packageScrollView.reflectScrolledClipView(packageScrollView.contentView)
    }

    private func moveSelection(
        _ direction: PackageFieldView.NavigationDirection
    ) -> Bool {
        guard let packageName = packageFieldView.adjacentPackageName(
            from: selectedItemID,
            direction: direction
        ) else {
            return false
        }

        select(itemID: packageName)
        if let frame = packageFieldView.frameForPackage(named: packageName) {
            packageFieldView.scrollToVisible(frame.insetBy(dx: 0, dy: -12))
        }
        return true
    }

    @discardableResult
    private func clearSelection() -> Bool {
        guard selectedItemID != nil else { return false }
        selectedItemID = nil
        loadingDetailItemID = nil
        isLoadingSelectedPackageDetail = false
        packageFieldView.apply(
            packages: visiblePackages,
            selectedPackageName: nil,
            searchQuery: searchQuery,
            secondarySectionTitle: packageSecondarySectionTitle,
            secondarySectionCount: packageSecondarySectionCount,
            panelHeaderTitle: packagePanelHeaderTitle,
            panelHeaderCount: packagePanelHeaderCount,
            commandPaletteHelpText: commandPaletteHelpText,
            commandPaletteQuoteText: commandPaletteQuoteText
        )
        dossierView.render(detail: nil, animation: .none)
        externalSurfaceView.render(detail: nil, animated: false)
        return true
    }

    private func applyVisiblePackages(_ packages: [PackagePresentation]) {
        visiblePackages = packages
        let previousSelection = selectedItemID

        let shouldAutoSelectLoneSearchResult =
            searchQuery.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
            && {
                if case .commandBrowser = paletteMode {
                    return false
                }
                return true
            }()
            && isSearching == false
            && packages.count == 1

        let nextSelection: String?
        if shouldAutoSelectLoneSearchResult {
            nextSelection = packages[0].selectionID
        } else if let selectedPackageName = selectedItemID,
           packages.contains(where: { $0.selectionID == selectedPackageName }) {
            nextSelection = selectedPackageName
        } else {
            nextSelection = nil
        }

        self.selectedItemID = nextSelection
        packageFieldView.apply(
            packages: packages,
            selectedPackageName: nextSelection,
            searchQuery: searchQuery,
            secondarySectionTitle: packageSecondarySectionTitle,
            secondarySectionCount: packageSecondarySectionCount,
            panelHeaderTitle: packagePanelHeaderTitle,
            panelHeaderCount: packagePanelHeaderCount,
            commandPaletteHelpText: commandPaletteHelpText,
            commandPaletteQuoteText: commandPaletteQuoteText
        )
        updatePackageScrollLayout(
            x: packageScrollView.frame.minX,
            y: packageScrollView.frame.minY,
            width: packageScrollView.frame.width,
            height: packageScrollView.frame.height
        )
        if let nextSelection, nextSelection != previousSelection {
            select(
                itemID: nextSelection,
                lazyLoadOnly: shouldAutoSelectLoneSearchResult == false,
                updateFieldView: false
            )
        } else if previousSelection != nil, nextSelection == nil {
            loadingDetailItemID = nil
            isLoadingSelectedPackageDetail = false
            dossierView.render(detail: nil, animation: .none)
            externalSurfaceView.render(detail: nil, animated: false)
        }
    }

    private func updatePackageScrollLayout(x: CGFloat, y: CGFloat, width: CGFloat, height: CGFloat) {
        packageScrollView.frame = CGRect(
            x: x,
            y: y,
            width: width,
            height: height
        )
        let packageContentHeight = packageFieldView.contentHeight(forWidth: width)
        packageFieldView.frame = CGRect(
            x: 0,
            y: 0,
            width: width,
            height: max(height, packageContentHeight)
        )
        packageScrollView.reflectScrolledClipView(packageScrollView.contentView)
    }

    private func updateHeader() {
        guard let rootView = view as? RootView else { return }
        rootView.headerLayer.string = mastheadAttributedText()
        rootView.commandPalette.isActive =
            view.window?.firstResponder === rootView.commandPalette.field.currentEditor()
            || view.window?.firstResponder === rootView.commandPalette.field

        let statusText = activityStatusText()
        if let statusText, statusText.isEmpty == false {
            statusAnimator?.setText(statusText, animated: true)
            rootView.statusLabel.isHidden = false
        } else {
            statusAnimator?.setText(nil, animated: false)
            rootView.statusLabel.stringValue = ""
            rootView.statusLabel.isHidden = true
        }
        view.needsLayout = true
    }

    private func updatePaneLoadingIndicators() {
        packageFieldView.setEyebrowLoading(
            isSearching
                || isLoadingMoreSearchResults
                || isLoadingCommandResults
                || isLoadingMoreCommandResults
        )
        dossierView.setEyebrowLoading(isLoadingSelectedPackageDetail)
        externalSurfaceView.setEyebrowLoading(isLoadingSelectedPackageDetail)
    }

    private func statusTextWidth(_ text: String) -> CGFloat {
        let attributed = UIStyle.attributedMonoText(
            text,
            size: 10,
            color: UIStyle.accent.withAlphaComponent(0.86),
            weight: .medium,
            tracking: 0.9
        )
        let bounds = attributed.boundingRect(
            with: CGSize(width: CGFloat.greatestFiniteMagnitude, height: 16),
            options: [.usesLineFragmentOrigin, .usesFontLeading]
        )
        return ceil(bounds.width)
    }

    private func mastheadAttributedText() -> NSAttributedString {
        let text = NSMutableAttributedString(
            attributedString: UIStyle.attributedMonoText(
                "AUTOMIC VAULT  ",
                size: 11,
                color: UIStyle.text.withAlphaComponent(0.60),
                weight: .medium,
                tracking: 1.15
            )
        )
        text.append(UIStyle.attributedMonoText(
            "v",
            size: 11,
            color: UIStyle.text.withAlphaComponent(0.24),
            weight: .medium,
            tracking: 1.15
        ))
        text.append(UIStyle.attributedMonoText(
            appVersion,
            size: 11,
            color: UIStyle.text.withAlphaComponent(0.42),
            weight: .medium,
            tracking: 1.15
        ))
        if let codeSignatureHash {
            text.append(UIStyle.attributedMonoText(
                "  \(codeSignatureHash)",
                size: 11,
                color: UIStyle.text.withAlphaComponent(0.34),
                weight: .medium,
                tracking: 1.15
            ))
        }
        return text
    }

    private func attributedTextWidth(_ attributed: NSAttributedString) -> CGFloat {
        let bounds = attributed.boundingRect(
            with: CGSize(width: CGFloat.greatestFiniteMagnitude, height: 16),
            options: [.usesLineFragmentOrigin, .usesFontLeading]
        )
        return ceil(bounds.width)
    }

    private static func abbreviatedCodeSignatureHash() -> String? {
        var currentCode: SecCode?
        let copyStatus = SecCodeCopySelf(SecCSFlags(), &currentCode)
        guard copyStatus == errSecSuccess, let currentCode else {
            return nil
        }

        var staticCode: SecStaticCode?
        let staticStatus = SecCodeCopyStaticCode(
            currentCode,
            SecCSFlags(),
            &staticCode
        )
        guard staticStatus == errSecSuccess, let staticCode else {
            return nil
        }

        var signingInfo: CFDictionary?
        let infoStatus = SecCodeCopySigningInformation(
            staticCode,
            SecCSFlags(rawValue: kSecCSSigningInformation),
            &signingInfo
        )
        guard infoStatus == errSecSuccess,
              let dictionary = signingInfo as NSDictionary?,
              let decoded = dictionary as? [String: Any] else {
            return nil
        }

        if let uniqueData = decoded[kSecCodeInfoUnique as String] as? Data {
            return abbreviatedHexString(from: uniqueData)
        }

        if let uniqueString = decoded[kSecCodeInfoUnique as String] as? String,
           !uniqueString.isEmpty {
            return String(uniqueString.prefix(codeSignatureHashLength))
        }

        return nil
    }

    private static func abbreviatedHexString(from data: Data) -> String? {
        guard data.isEmpty == false else { return nil }
        let hex = data.map { String(format: "%02x", $0) }.joined()
        return String(hex.prefix(codeSignatureHashLength))
    }

    private func activityStatusText() -> String? {
        if isRunningPackageOperation {
            return "nucleus package channel active"
        }
        if isInstallingAv {
            return "nucleus av install channel active"
        }
        if isRunningPrivilegedUpdate {
            return "nucleus update channel active"
        }
        return nil
    }

    private func insertSearchText(_ text: String) {
        let normalized = normalizedSearchText(text)
        guard !normalized.isEmpty else { return }
        searchQuery.append(normalized)
    }

    private func normalizedSearchText(_ text: String) -> String {
        let cleaned = text
            .replacingOccurrences(of: "\r\n", with: " ")
            .replacingOccurrences(of: "\n", with: " ")
            .replacingOccurrences(of: "\r", with: " ")
            .lowercased()
        let allowedScalars = cleaned.unicodeScalars.filter { scalar in
            !CharacterSet.controlCharacters.contains(scalar)
        }
        return String(String.UnicodeScalarView(allowedScalars))
    }

    private func focusCommandPalette(selectExistingText: Bool = true) {
        guard let rootView = view as? RootView else { return }
        view.window?.makeFirstResponder(rootView.commandPalette.field)
        applyCommandPaletteEditorTint(for: rootView.commandPalette.field)
        if selectExistingText == false {
            collapseCommandPaletteSelectionToEnd(for: rootView.commandPalette.field)
            DispatchQueue.main.async { [weak self, weak field = rootView.commandPalette.field] in
                guard let self, let field else { return }
                self.collapseCommandPaletteSelectionToEnd(for: field)
            }
        }
        rootView.commandPalette.isActive = true
        setCommandPaletteFocused(true)
    }

    private func setCommandPaletteFocused(_ focused: Bool) {
        guard isCommandPaletteFocused != focused else { return }
        isCommandPaletteFocused = focused
    }

    private func applyCommandPaletteEditorTint(for field: NSTextField) {
        guard let editor = field.currentEditor() as? NSTextView else { return }
        editor.insertionPointColor = UIStyle.accent
        editor.selectedTextAttributes = [
            .backgroundColor: UIStyle.accent.withAlphaComponent(0.28),
            .foregroundColor: UIStyle.text
        ]
    }

    private func collapseCommandPaletteSelectionToEnd(for field: NSTextField) {
        let insertionIndex = field.stringValue.utf16.count
        if let editor = field.currentEditor() as? NSTextView {
            editor.setSelectedRange(NSRange(location: insertionIndex, length: 0))
        }
    }

    private func syncCommandPaletteText() {
        guard let rootView = view as? RootView else { return }
        if rootView.commandPalette.field.stringValue != searchQuery {
            rootView.commandPalette.field.stringValue = searchQuery
        }
        rootView.commandPalette.needsLayout = true
    }

    private func updateUpdateButtonVisibility() {
        guard let rootView = view as? RootView else { return }
        let hasAppUpdate = appUpdateCoordinator.hasAvailableUpdate
        rootView.updateButton.title = hasAppUpdate ? "UPDATE SELF" : "UPDATE ALL"
        rootView.updateButton.isHidden = !hasUpdatesAvailable
            || (isRunningPrivilegedUpdate && hasAppUpdate == false)
        rootView.updateButton.isEnabled = hasAppUpdate
            || (!isRunningPrivilegedUpdate && !isRunningPackageOperation && !isInstallingAv)
        view.needsLayout = true
    }

    private func refreshUpdateAvailability() {
        hasUpdatesAvailable = appUpdateCoordinator.hasAvailableUpdate
            || !outdatedPackagesByName.isEmpty
            || hasCLTRecommendationUpdate
    }

    private var hasCLTRecommendationUpdate: Bool {
        recommendations.contains { package in
            guard case .recommendation(let recommendation) = package.item else {
                return false
            }
            return recommendation.isOutdated
        }
    }

    @objc private func beginUpdateFlow() {
        if appUpdateCoordinator.hasAvailableUpdate {
            beginSelfUpdateFlow()
            return
        }

        helperBridge.authenticateBiometrics(
            reason: "Authorize privileged package updates for Atomic Vault."
        ) { result in
            switch result {
            case .success:
                self.startUpdateOperation()
            case .failure(let error):
                self.presentHelperError(error)
            }
        }
    }

    private func beginSelfUpdateFlow() {
        appUpdateCoordinator.installWhenReady(
            readiness: { [weak self] in
                guard let self else {
                    return .busy("Automic Vault is not ready to update.")
                }
                if self.isRunningPackageOperation
                    || self.isRunningPrivilegedUpdate
                    || self.isInstallingAv {
                    return .busy("Wait for active operations to finish before updating Automic Vault.")
                }
                return .ready
            },
            prepareForInstall: { [weak self] in
                self?.prepareForSelfUpdateInstall()
            }
        )
    }

    private func prepareForSelfUpdateInstall() {
        NSRunningApplication.runningApplications(
            withBundleIdentifier: "com.automicvault.menu-helper"
        ).forEach { $0.terminate() }
        view.window?.orderOut(nil)
    }

    private func beginAutomicVaultCLTInstallFlow() {
        helperBridge.authenticateBiometrics(
            reason: "Authorize installation of Automic Vault command line tools into /usr/local/bin."
        ) { result in
            switch result {
            case .success:
                self.startAutomicVaultCLTInstallOperation()
            case .failure(let error):
                self.presentHelperError(error)
            }
        }
    }

    private func startUpdateOperation() {
        if outdatedPackagesByName.isEmpty, hasCLTRecommendationUpdate {
            startAutomicVaultCLTInstallOperation()
            return
        }

        let operationID = beginOverlayOperation()
        isRunningPrivilegedUpdate = true
        let overlay = presentUpdateOverlay()
        overlay.onRetry = { [weak self] in
            self?.startUpdateOperation()
        }
        overlay.onDismiss = { [weak self] in
            self?.dismissUpdateOverlay()
        }
        overlay.configure(
            title: "NUCLEUS UPDATE CHANNEL",
            awaitingClearance: "Awaiting clearance",
            idleStatus: "Nucleus idle",
            successOperation: "Update Complete",
            failureOperation: "Update Halted"
        )
        let packageNames = outdatedPackagesByName.keys.sorted()
        overlay.begin(
            packages: packageNames,
            activationLog: "Privilege gate cleared. Opening channel to nucleus."
        )
        helperBridge.updateAll(
            progress: { event in
                guard self.activeOverlayOperationID == operationID else { return }
                overlay.handle(event: event)
            },
            completion: { result in
                guard self.activeOverlayOperationID == operationID else { return }
                self.isRunningPrivilegedUpdate = false
                switch result {
                case .success(let summary):
                    summary.processedPackages.forEach {
                        self.detailsByPackageName.removeValue(forKey: $0)
                    }
                    overlay.succeed(
                        message: summary.message,
                        packages: summary.processedPackages
                    )
                    self.reloadPackages()
                    self.refreshRecommendations()
                    self.refreshUpdateAvailability()
                    self.refreshMenuBarAfterPrivilegedHelperOperation()
                    if self.hasCLTRecommendationUpdate {
                        self.startAutomicVaultCLTInstallOperation()
                    }
                case .failure(let error):
                    overlay.fail(message: error.localizedDescription)
                    self.presentHelperError(error, suppressAlertWhenOverlayVisible: true)
                    self.refreshRecommendations()
                    self.refreshUpdateAvailability()
                }
            }
        )
    }

    private func startAutomicVaultCLTInstallOperation() {
        let stagedDirectory: URL
        do {
            stagedDirectory = try bridge.exportBundledCLTForHelperInstall()
        } catch {
            presentHelperError(error)
            return
        }

        let operationID = beginOverlayOperation()
        isInstallingAv = true
        let overlay = presentUpdateOverlay()
        overlay.onRetry = { [weak self] in
            self?.startAutomicVaultCLTInstallOperation()
        }
        overlay.onDismiss = { [weak self] in
            self?.dismissUpdateOverlay()
        }
        overlay.configure(
            title: "NUCLEUS CLT INSTALL",
            awaitingClearance: "Awaiting install clearance",
            idleStatus: "Nucleus primed for CLT install",
            successOperation: "Install Complete",
            failureOperation: "Install Halted"
        )
        overlay.begin(
            packages: ["av"],
            activationLog: "Privilege gate cleared. Opening channel to nucleus for CLT install."
        )
        helperBridge.installAv(
            sourcePath: stagedDirectory.path,
            progress: { event in
                guard self.activeOverlayOperationID == operationID else { return }
                overlay.handle(event: event)
            },
            completion: { result in
                guard self.activeOverlayOperationID == operationID else { return }
                self.isInstallingAv = false
                try? FileManager.default.removeItem(
                    at: stagedDirectory
                )
                switch result {
                case .success(let summary):
                    overlay.succeed(
                        message: summary.message,
                        packages: summary.processedPackages
                    )
                    self.refreshRecommendations()
                    self.reloadPackages()
                    self.refreshMenuBarAfterPrivilegedHelperOperation()
                case .failure(let error):
                    overlay.fail(message: error.localizedDescription)
                    self.presentHelperError(error, suppressAlertWhenOverlayVisible: true)
                    self.refreshRecommendations()
                }
            }
        )
    }

    private func presentUpdateOverlay() -> UpdateProgressViewController {
        if let updateOverlayController {
            updateOverlayController.view.frame = view.bounds
            view.window?.makeFirstResponder(updateOverlayController.view)
            return updateOverlayController
        }

        let controller = UpdateProgressViewController()
        controller.onRetry = { [weak self] in
            self?.startUpdateOperation()
        }
        controller.onDismiss = { [weak self] in
            self?.dismissUpdateOverlay()
        }
        addChild(controller)
        controller.view.frame = view.bounds
        view.addSubview(controller.view)
        updateOverlayController = controller
        view.window?.makeFirstResponder(controller.view)
        return controller
    }

    private enum PackageMutationKind {
        case install
        case update
        case uninstall

        var overlayTitle: String {
            switch self {
            case .install:
                return "NUCLEUS INSTALL CHANNEL"
            case .update:
                return "NUCLEUS UPDATE CHANNEL"
            case .uninstall:
                return "NUCLEUS UNINSTALL CHANNEL"
            }
        }

        var awaitingClearance: String {
            switch self {
            case .install:
                return "Awaiting install clearance"
            case .update:
                return "Awaiting update clearance"
            case .uninstall:
                return "Awaiting uninstall clearance"
            }
        }

        var idleStatus: String {
            switch self {
            case .install:
                return "Nucleus primed for install"
            case .update:
                return "Nucleus primed for update"
            case .uninstall:
                return "Nucleus primed for uninstall"
            }
        }

        var successOperation: String {
            switch self {
            case .install:
                return "Install Complete"
            case .update:
                return "Update Complete"
            case .uninstall:
                return "Uninstall Complete"
            }
        }

        var failureOperation: String {
            switch self {
            case .install:
                return "Install Halted"
            case .update:
                return "Update Halted"
            case .uninstall:
                return "Uninstall Halted"
            }
        }

        func biometricReason(packageName: String) -> String {
            switch self {
            case .install:
                return "Authorize privileged package install for \(packageName)."
            case .update:
                return "Authorize privileged package update for \(packageName)."
            case .uninstall:
                return "Authorize privileged package uninstall for \(packageName)."
            }
        }

        func activationLog(packageName: String) -> String {
            switch self {
            case .install:
                return "Privilege gate cleared. Opening channel to nucleus for \(packageName)."
            case .update:
                return "Privilege gate cleared. Opening channel to nucleus for \(packageName)."
            case .uninstall:
                return "Privilege gate cleared. Opening channel to nucleus for \(packageName)."
            }
        }
    }

    private func beginPackageMutation(for detail: PackageDetail) {
        if detail.isAutomicVaultCLT {
            beginAutomicVaultCLTInstallFlow()
            return
        }
        if detail.isXcodeCLT {
            startXcodeCLTInstallOperation()
            return
        }
        let kind: PackageMutationKind = detail.installed ? .uninstall : .install
        helperBridge.authenticateBiometrics(
            reason: kind.biometricReason(packageName: detail.packageName)
        ) { result in
            switch result {
            case .success:
                self.startPackageMutation(kind, detail: detail)
            case .failure(let error):
                self.presentHelperError(error)
            }
        }
    }

    private func beginPackageUpdate(for detail: PackageDetail) {
        guard detail.installed, detail.isOutdated else { return }
        if detail.isAutomicVaultCLT {
            beginAutomicVaultCLTInstallFlow()
            return
        }
        if detail.isXcodeCLT {
            startXcodeCLTInstallOperation()
            return
        }
        let kind: PackageMutationKind = .update
        helperBridge.authenticateBiometrics(
            reason: kind.biometricReason(packageName: detail.packageName)
        ) { result in
            switch result {
            case .success:
                self.startPackageMutation(kind, detail: detail)
            case .failure(let error):
                self.presentHelperError(error)
            }
        }
    }

    private func beginSecurityMutation(for detail: PackageDetail) {
        guard let notice = detail.securityNotice,
              notice.source == .isotope,
              let applyPackageName = notice.applyPackageName else {
            return
        }
        if let source = detail.source, case .isotope(let isotopeName) = source {
            startIsotopeSecretMigrationFlow(
                isotopeName: isotopeName,
                packageName: detail.helperPackageName
            )
            return
        }
        if detail.helperPackageName == applyPackageName {
            return
        }

        let kind: PackageMutationKind = .install
        helperBridge.authenticateBiometrics(
            reason: "Authorize privileged security upgrade install for \(detail.packageName)."
        ) { result in
            switch result {
            case .success:
                self.startIsotopeSecurityMutation(
                    detail: detail,
                    applyPackageName: applyPackageName,
                    fallbackKind: kind
                )
            case .failure(let error):
                self.presentHelperError(error)
            }
        }
    }

    private func startIsotopeSecurityMutation(
        detail: PackageDetail,
        applyPackageName: String,
        fallbackKind: PackageMutationKind
    ) {
        guard let isotopeName = isotopeName(from: applyPackageName) else {
            startPackageMutation(
                fallbackKind,
                detail: detail,
                packageName: applyPackageName,
                activationPackageName: "security upgrade for \(detail.packageName)"
            )
            return
        }

        DispatchQueue.global(qos: .userInitiated).async {
            do {
                let plan = try self.bridge.fetchIsotopeMigrationPlan(isotopeName: isotopeName)
                DispatchQueue.main.async {
                    if plan.isRadioisotope == true {
                        self.startRadioisotopeConversionMutation(
                            isotopeName: isotopeName,
                            detail: detail,
                            plan: plan
                        )
                    } else {
                        self.startPackageMutation(
                            fallbackKind,
                            detail: detail,
                            packageName: applyPackageName,
                            activationPackageName: "security upgrade for \(detail.packageName)"
                        )
                    }
                }
            } catch {
                DispatchQueue.main.async {
                    self.presentHelperError(error)
                }
            }
        }
    }

    private func startRadioisotopeConversionMutation(
        isotopeName: String,
        detail: PackageDetail,
        plan: NucleusBridge.IsotopeMigrationPlan
    ) {
        let operationID = beginOverlayOperation()
        isRunningPackageOperation = true
        let packageName = "isotope:\(isotopeName)"
        let overlay = presentUpdateOverlay()
        overlay.onRetry = { [weak self] in
            self?.startRadioisotopeConversionMutation(
                isotopeName: isotopeName,
                detail: detail,
                plan: plan
            )
        }
        overlay.onDismiss = { [weak self] in
            self?.dismissUpdateOverlay()
        }
        overlay.configure(
            title: "ISOTOPE CONVERSION",
            awaitingClearance: "Preparing isotope conversion",
            idleStatus: "Nucleus isotope conversion channel ready",
            successOperation: "Conversion Complete",
            failureOperation: "Conversion Halted"
        )
        overlay.begin(
            packages: [packageName],
            activationLog: "Converting \(detail.packageName) to isotope."
        )

        runRadioisotopeMigrationAndConversion(
            isotopeName: isotopeName,
            packageName: packageName,
            plan: plan,
            operationID: operationID,
            overlay: overlay
        )
    }

    private func runRadioisotopeMigrationAndConversion(
        isotopeName: String,
        packageName: String,
        plan: NucleusBridge.IsotopeMigrationPlan,
        operationID: Int,
        overlay: UpdateProgressViewController
    ) {
        DispatchQueue.global(qos: .userInitiated).async {
            do {
                if plan.hasMigration {
                    DispatchQueue.main.async {
                        guard self.activeOverlayOperationID == operationID else { return }
                        overlay.handle(event: .log(
                            package: packageName,
                            message: "migrating secrets"
                        ))
                    }
                    _ = try self.bridge.migrateIsotope(isotopeName: isotopeName)
                }

                DispatchQueue.main.async {
                    guard self.activeOverlayOperationID == operationID else { return }
                    self.helperBridge.convertRadioisotope(
                        isotopeName: isotopeName,
                        progress: { event in
                            guard self.activeOverlayOperationID == operationID else { return }
                            overlay.handle(event: event)
                        }
                    ) { result in
                        guard self.activeOverlayOperationID == operationID else { return }
                        self.isRunningPackageOperation = false
                        switch result {
                        case .success(let summary):
                            summary.processedPackages.forEach {
                                self.detailsByPackageName.removeValue(forKey: $0)
                            }
                            self.detailsByPackageName.removeValue(forKey: packageName)
                            overlay.succeed(
                                message: summary.message,
                                packages: summary.processedPackages
                            )
                            self.reloadPackages()
                            self.refreshRecommendations()
                            self.refreshUpdateAvailability()
                            self.refreshMenuBarAfterPrivilegedHelperOperation()
                        case .failure(let error):
                            overlay.fail(message: error.localizedDescription)
                            self.presentHelperError(
                                error,
                                suppressAlertWhenOverlayVisible: true
                            )
                            self.refreshRecommendations()
                            self.refreshUpdateAvailability()
                        }
                    }
                }
            } catch {
                DispatchQueue.main.async {
                    guard self.activeOverlayOperationID == operationID else { return }
                    self.isRunningPackageOperation = false
                    overlay.fail(message: error.localizedDescription)
                    self.presentHelperError(error, suppressAlertWhenOverlayVisible: true)
                    self.refreshRecommendations()
                    self.refreshUpdateAvailability()
                }
            }
        }
    }

    private func startIsotopeSecretMigrationFlow(isotopeName: String, packageName: String) {
        let operationID = beginOverlayOperation()
        isRunningPackageOperation = true
        let overlay = presentUpdateOverlay()
        overlay.onRetry = { [weak self] in
            self?.startIsotopeSecretMigrationFlow(
                isotopeName: isotopeName,
                packageName: packageName
            )
        }
        overlay.onDismiss = { [weak self] in
            self?.dismissUpdateOverlay()
        }
        overlay.configure(
            title: "ISOTOPE SECRET MIGRATION",
            awaitingClearance: "Preparing secret migration",
            idleStatus: "Nucleus isotope migration channel ready",
            successOperation: "Secrets Secured",
            failureOperation: "Migration Halted"
        )
        overlay.begin(
            packages: [packageName],
            activationLog: "Opening channel to nucleus for \(packageName) migration."
        )
        runIsotopeSecretMigrationOnly(
            isotopeName: isotopeName,
            packageName: packageName,
            operationID: operationID,
            overlay: overlay
        )
    }

    private func runIsotopeSecretMigrationOnly(
        isotopeName: String,
        packageName: String,
        operationID: Int,
        overlay: UpdateProgressViewController
    ) {
        DispatchQueue.global(qos: .userInitiated).async {
            do {
                let plan = try self.bridge.fetchIsotopeMigrationPlan(isotopeName: isotopeName)
                if plan.hasMigration {
                    DispatchQueue.main.async {
                        guard self.activeOverlayOperationID == operationID else { return }
                        overlay.handle(event: .log(
                            package: packageName,
                            message: "migrating secrets"
                        ))
                    }
                    _ = try self.bridge.migrateIsotope(isotopeName: isotopeName)
                }
                DispatchQueue.main.async {
                    guard self.activeOverlayOperationID == operationID else { return }
                    self.isRunningPackageOperation = false
                    self.detailsByPackageName.removeValue(forKey: packageName)
                    overlay.succeed(
                        message: plan.hasMigration
                            ? "Isotope secrets migrated"
                            : "No isotope secrets migration required",
                        packages: [packageName]
                    )
                    self.reloadPackages()
                    self.refreshRecommendations()
                    self.refreshUpdateAvailability()
                    self.statusStore.requestRefresh()
                }
            } catch {
                DispatchQueue.main.async {
                    guard self.activeOverlayOperationID == operationID else { return }
                    self.isRunningPackageOperation = false
                    overlay.fail(message: error.localizedDescription)
                    self.presentHelperError(error, suppressAlertWhenOverlayVisible: true)
                    self.refreshRecommendations()
                    self.refreshUpdateAvailability()
                }
            }
        }
    }

    private func startXcodeCLTInstallOperation() {
        let operationID = beginOverlayOperation()
        isRunningPackageOperation = true
        let overlay = presentUpdateOverlay()
        overlay.onRetry = { [weak self] in
            self?.startXcodeCLTInstallOperation()
        }
        overlay.onDismiss = { [weak self] in
            self?.dismissUpdateOverlay()
        }
        overlay.configure(
            title: "XCODE CLT INSTALL",
            awaitingClearance: "Opening macOS installer",
            idleStatus: "macOS command line tools installer ready",
            successOperation: "Installer Opened",
            failureOperation: "Installer Halted"
        )
        overlay.begin(
            packages: [PackageRecommendation.xcodeCLTName],
            activationLog: "Opening Apple's Command Line Tools installer."
        )

        DispatchQueue.global(qos: .userInitiated).async {
            let process = Process()
            process.executableURL = URL(fileURLWithPath: "/usr/bin/xcode-select")
            process.arguments = ["--install"]
            let errorPipe = Pipe()
            process.standardOutput = FileHandle.nullDevice
            process.standardError = errorPipe

            do {
                try process.run()
                process.waitUntilExit()
                let errorData = errorPipe.fileHandleForReading.readDataToEndOfFile()
                let errorText = String(data: errorData, encoding: .utf8)?
                    .trimmingCharacters(in: .whitespacesAndNewlines)
                DispatchQueue.main.async {
                    guard self.activeOverlayOperationID == operationID else { return }
                    self.isRunningPackageOperation = false
                    if process.terminationStatus == 0 {
                        overlay.succeed(
                            message: "macOS Command Line Tools installer opened",
                            packages: [PackageRecommendation.xcodeCLTName]
                        )
                        self.refreshRecommendations()
                    } else {
                        let message = errorText?.isEmpty == false
                            ? errorText!
                            : "xcode-select exited with status \(process.terminationStatus)"
                        let error = NSError(
                            domain: "AutomicVault.XcodeCLT",
                            code: Int(process.terminationStatus),
                            userInfo: [NSLocalizedDescriptionKey: message]
                        )
                        overlay.fail(message: error.localizedDescription)
                        self.presentHelperError(error, suppressAlertWhenOverlayVisible: true)
                        self.refreshRecommendations()
                    }
                }
            } catch {
                DispatchQueue.main.async {
                    guard self.activeOverlayOperationID == operationID else { return }
                    self.isRunningPackageOperation = false
                    overlay.fail(message: error.localizedDescription)
                    self.presentHelperError(error, suppressAlertWhenOverlayVisible: true)
                    self.refreshRecommendations()
                }
            }
        }
    }

    func applicationWillTerminate() {
        appUpdateCoordinator.stop()
        if let snapshotObserver {
            DistributedNotificationCenter.default().removeObserver(snapshotObserver)
        }
        bridge.invalidate()
    }

    private func refreshMenuBarAfterPrivilegedHelperOperation() {
        try? statusStore.saveRemoteDatabaseRefreshState(.normal)
        statusStore.requestRefresh()
    }

    private func startPackageMutation(
        _ kind: PackageMutationKind,
        detail: PackageDetail,
        packageName: String? = nil,
        activationPackageName: String? = nil
    ) {
        let operationID = beginOverlayOperation()
        isRunningPackageOperation = true
        let overlay = presentUpdateOverlay()
        overlay.onRetry = { [weak self] in
            self?.startPackageMutation(
                kind,
                detail: detail,
                packageName: packageName,
                activationPackageName: activationPackageName
            )
        }
        overlay.onDismiss = { [weak self] in
            self?.dismissUpdateOverlay()
        }
        overlay.configure(
            title: kind.overlayTitle,
            awaitingClearance: kind.awaitingClearance,
            idleStatus: kind.idleStatus,
            successOperation: kind.successOperation,
            failureOperation: kind.failureOperation
        )
        let resolvedPackageNames = packageName.map { [$0] } ?? detail.helperPackageNames
        let resolvedPackageName = resolvedPackageNames.first ?? detail.helperPackageName
        let resolvedActivationName = activationPackageName ?? detail.packageName
        overlay.begin(
            packages: resolvedPackageNames,
            activationLog: kind.activationLog(packageName: resolvedActivationName)
        )

        let packages = resolvedPackageNames.map { AVPackageSpec(name: $0) }
        let completion: (Result<NukeHelperResult, Error>) -> Void = { result in
            guard self.activeOverlayOperationID == operationID else { return }
            self.isRunningPackageOperation = false
            switch result {
            case .success(let summary):
                summary.processedPackages.forEach {
                    self.detailsByPackageName.removeValue(forKey: $0)
                }
                overlay.succeed(
                    message: summary.message,
                    packages: summary.processedPackages
                )
                self.reloadPackages()
                self.refreshRecommendations()
                self.refreshUpdateAvailability()
                self.refreshMenuBarAfterPrivilegedHelperOperation()
            case .failure(let error):
                overlay.fail(message: error.localizedDescription)
                self.presentHelperError(error, suppressAlertWhenOverlayVisible: true)
                self.refreshRecommendations()
                self.refreshUpdateAvailability()
            }
        }

        if packages.count == 1,
           kind == .install,
           let isotopeName = isotopeName(from: resolvedPackageName) {
            startIsotopeInstallMutation(
                isotopeName: isotopeName,
                package: packages[0],
                operationID: operationID,
                overlay: overlay,
                completion: completion
            )
            return
        }

        switch kind {
        case .install:
            helperBridge.install(
                packages: packages,
                progress: { event in
                    guard self.activeOverlayOperationID == operationID else { return }
                    overlay.handle(event: event)
                },
                completion: completion
            )
        case .update:
            helperBridge.update(
                packages: packages,
                progress: { event in
                    guard self.activeOverlayOperationID == operationID else { return }
                    overlay.handle(event: event)
                },
                completion: completion
            )
        case .uninstall:
            helperBridge.uninstall(
                packages: packages,
                progress: { event in
                    guard self.activeOverlayOperationID == operationID else { return }
                    overlay.handle(event: event)
                },
                completion: completion
            )
        }
    }

    private func isotopeName(from packageName: String) -> String? {
        let prefix = "isotope:"
        guard packageName.hasPrefix(prefix) else {
            return nil
        }
        let name = String(packageName.dropFirst(prefix.count))
        return name.isEmpty ? nil : name
    }

    private func startIsotopeInstallMutation(
        isotopeName: String,
        package: AVPackageSpec,
        operationID: Int,
        overlay: UpdateProgressViewController,
        completion: @escaping (Result<NukeHelperResult, Error>) -> Void
    ) {
        let forwardProgress: (NukeHelperProgressEvent) -> Void = { event in
            guard self.activeOverlayOperationID == operationID else { return }
            overlay.handle(event: event)
        }
        helperBridge.installIsotopeRoot(
            isotopeName: isotopeName,
            progress: forwardProgress
        ) { result in
            guard self.activeOverlayOperationID == operationID else { return }
            switch result {
            case .success:
                self.runIsotopeMigrationStep(
                    isotopeName: isotopeName,
                    package: package,
                    operationID: operationID,
                    overlay: overlay,
                    progress: forwardProgress,
                    completion: completion
                )
            case .failure:
                completion(result)
            }
        }
    }

    private func runIsotopeMigrationStep(
        isotopeName: String,
        package: AVPackageSpec,
        operationID: Int,
        overlay: UpdateProgressViewController,
        progress: @escaping (NukeHelperProgressEvent) -> Void,
        completion: @escaping (Result<NukeHelperResult, Error>) -> Void
    ) {
        DispatchQueue.global(qos: .userInitiated).async {
            do {
                let plan = try self.bridge.fetchIsotopeMigrationPlan(isotopeName: isotopeName)
                if plan.hasMigration {
                    DispatchQueue.main.async {
                        guard self.activeOverlayOperationID == operationID else { return }
                        overlay.handle(event: .log(
                            package: package.name,
                            message: "migrating secrets"
                        ))
                    }
                    _ = try self.bridge.migrateIsotope(isotopeName: isotopeName)
                }
                DispatchQueue.main.async {
                    guard self.activeOverlayOperationID == operationID else { return }
                    self.uninstallIsotopeReplacementIfNeeded(
                        plan: plan,
                        isotopeName: isotopeName,
                        package: package,
                        operationID: operationID,
                        overlay: overlay,
                        progress: progress,
                        completion: completion
                    )
                }
            } catch {
                DispatchQueue.main.async {
                    guard self.activeOverlayOperationID == operationID else { return }
                    completion(.failure(error))
                }
            }
        }
    }

    private func uninstallIsotopeReplacementIfNeeded(
        plan: NucleusBridge.IsotopeMigrationPlan,
        isotopeName: String,
        package: AVPackageSpec,
        operationID: Int,
        overlay: UpdateProgressViewController,
        progress: @escaping (NukeHelperProgressEvent) -> Void,
        completion: @escaping (Result<NukeHelperResult, Error>) -> Void
    ) {
        guard let replacesPackage = plan.replacesPackage else {
            installIsotopeStubs(
                isotopeName: isotopeName,
                package: package,
                operationID: operationID,
                overlay: overlay,
                progress: progress,
                completion: completion
            )
            return
        }
        overlay.handle(event: .log(
            package: package.name,
            message: "removing replaced package \(replacesPackage)"
        ))
        helperBridge.uninstall(
            packages: [AVPackageSpec(name: replacesPackage)],
            progress: progress
        ) { result in
            guard self.activeOverlayOperationID == operationID else { return }
            switch result {
            case .success:
                self.installIsotopeStubs(
                    isotopeName: isotopeName,
                    package: package,
                    additionalProcessedPackages: [replacesPackage],
                    operationID: operationID,
                    overlay: overlay,
                    progress: progress,
                    completion: completion
                )
            case .failure:
                completion(result)
            }
        }
    }

    private func installIsotopeStubs(
        isotopeName: String,
        package: AVPackageSpec,
        additionalProcessedPackages: [String] = [],
        operationID: Int,
        overlay: UpdateProgressViewController,
        progress: @escaping (NukeHelperProgressEvent) -> Void,
        completion: @escaping (Result<NukeHelperResult, Error>) -> Void
    ) {
        overlay.handle(event: .log(package: package.name, message: "installing isotope stubs"))
        helperBridge.installIsotopeStubs(
            isotopeName: isotopeName,
            progress: progress
        ) { result in
            guard self.activeOverlayOperationID == operationID else { return }
            switch result {
            case .success(let summary):
                completion(.success(NukeHelperResult(
                    message: summary.message,
                    processedPackages: summary.processedPackages + additionalProcessedPackages
                )))
            case .failure:
                completion(result)
            }
        }
    }

    private func beginOverlayOperation() -> Int {
        activeOverlayOperationID += 1
        return activeOverlayOperationID
    }

    private func dismissUpdateOverlay() {
        activeOverlayOperationID += 1
        isInstallingAv = false
        isRunningPrivilegedUpdate = false
        isRunningPackageOperation = false
        guard let updateOverlayController else { return }
        updateOverlayController.view.removeFromSuperview()
        updateOverlayController.removeFromParent()
        self.updateOverlayController = nil
        view.window?.makeFirstResponder(view)
    }

    private func presentHelperError(
        _ error: Error,
        suppressAlertWhenOverlayVisible: Bool = false
    ) {
        if suppressAlertWhenOverlayVisible, updateOverlayController != nil {
            return
        }
        let alert = NSAlert()
        alert.alertStyle = .warning
        alert.messageText = "Privileged Operation Failed"
        alert.informativeText = error.localizedDescription
        alert.addButton(withTitle: "Dismiss")
        if let window = view.window {
            alert.beginSheetModal(for: window)
        } else {
            alert.runModal()
        }
    }

    private func presentAppUpdateError(_ message: String) {
        let alert = NSAlert()
        alert.alertStyle = .warning
        alert.messageText = "Could Not Update Automic Vault"
        alert.informativeText = message
        alert.addButton(withTitle: "Dismiss")
        if let window = view.window {
            alert.beginSheetModal(for: window)
        } else {
            alert.runModal()
        }
    }
}
