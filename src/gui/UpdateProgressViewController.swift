import AppKit

private enum PackageStage: String {
    case queued = "QUEUED"
    case resolving = "RESOLVING"
    case downloading = "DOWNLOADING"
    case extracting = "EXTRACTING"
    case installing = "INSTALLING"
    case completed = "COMPLETE"
    case failed = "FAULT"
}

private final class ProgressStripView: NSView {
    private let trackLayer = CALayer()
    private let fillLayer = CALayer()
    private var renderedProgress: CGFloat = 0

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        wantsLayer = true
        layer = CALayer()
        trackLayer.backgroundColor = UIStyle.separator.cgColor
        fillLayer.backgroundColor = UIStyle.accent.cgColor
        layer?.addSublayer(trackLayer)
        layer?.addSublayer(fillLayer)
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func layout() {
        super.layout()
        trackLayer.frame = bounds
        layoutFill(animated: false)
    }

    func setProgress(_ progress: CGFloat, animated: Bool) {
        renderedProgress = max(0, min(progress, 1))
        layoutFill(animated: animated)
    }

    private func layoutFill(animated: Bool) {
        let target = CGRect(
            x: 0,
            y: 0,
            width: bounds.width * renderedProgress,
            height: bounds.height
        )
        CATransaction.begin()
        CATransaction.setAnimationDuration(animated ? 0.08 : 0)
        fillLayer.frame = target
        CATransaction.commit()
    }
}

private enum PackageProgressListMetrics {
    static let rowHeight: CGFloat = 34
    static let rowSpacing: CGFloat = 6

    static func contentHeight(rowCount: Int) -> CGFloat {
        guard rowCount > 0 else { return rowHeight }
        return CGFloat(rowCount) * rowHeight
            + CGFloat(max(rowCount - 1, 0)) * rowSpacing
    }
}

private final class PackageProgressListView: NSView {
    override var isFlipped: Bool { true }

    override func layout() {
        super.layout()
        var y: CGFloat = 0
        for subview in subviews {
            subview.frame = CGRect(
                x: 0,
                y: y,
                width: bounds.width,
                height: PackageProgressListMetrics.rowHeight
            )
            y += PackageProgressListMetrics.rowHeight
                + PackageProgressListMetrics.rowSpacing
        }
    }
}

private final class PackageProgressRowView: NSView {
    private let nameField = NSTextField(labelWithString: "")
    private let statusField = NSTextField(labelWithString: "")
    private let speedField = NSTextField(labelWithString: "")
    private let progressView = ProgressStripView(frame: .zero)

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        wantsLayer = true
        layer = CALayer()
        layer?.backgroundColor = NSColor(calibratedWhite: 1, alpha: 0.02).cgColor
        layer?.cornerRadius = 4

        [nameField, statusField, speedField].forEach {
            $0.isEditable = false
            $0.isBordered = false
            $0.drawsBackground = false
            addSubview($0)
        }
        nameField.font = UIStyle.monoFont(size: 11, weight: .medium)
        nameField.textColor = UIStyle.text
        statusField.font = UIStyle.monoFont(size: 10, weight: .regular)
        statusField.textColor = UIStyle.accent.withAlphaComponent(0.9)
        speedField.font = UIStyle.monoFont(size: 10, weight: .regular)
        speedField.textColor = UIStyle.quietText
        addSubview(progressView)
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func layout() {
        super.layout()
        nameField.frame = CGRect(x: 10, y: bounds.height - 20, width: bounds.width - 160, height: 14)
        statusField.frame = CGRect(x: bounds.width - 145, y: bounds.height - 20, width: 78, height: 14)
        speedField.frame = CGRect(x: bounds.width - 72, y: bounds.height - 20, width: 62, height: 14)
        progressView.frame = CGRect(x: 10, y: 9, width: bounds.width - 20, height: 2)
    }

    func render(
        name: String,
        stage: PackageStage,
        progress: CGFloat,
        speed: String?,
        animated: Bool
    ) {
        nameField.stringValue = name
        statusField.stringValue = stage.rawValue
        speedField.stringValue = speed ?? ""
        progressView.setProgress(progress, animated: animated)
        if stage == .failed {
            statusField.textColor = UIStyle.danger
        } else if stage == .completed {
            statusField.textColor = UIStyle.accent
        } else {
            statusField.textColor = UIStyle.text.withAlphaComponent(0.72)
        }
    }
}

private final class GlitchTextAnimator {
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
        weight: NSFont.Weight = .medium,
        tracking: CGFloat = 0.2
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

    func setText(_ text: String, animated: Bool) {
        if text == baseText {
            if animated {
                startTimerIfNeeded()
            } else {
                stopTimer()
                bursts.removeAll(keepingCapacity: true)
                applyCurrentFrame()
            }
            return
        }

        baseText = text
        bursts.removeAll(keepingCapacity: true)
        applyCurrentFrame()

        if animated, !text.isEmpty {
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

final class UpdateProgressViewController: NSViewController {
    private enum ProgressLayout {
        static let queued: CGFloat = 0.02
        static let resolving: CGFloat = 0.04
        static let downloadFloor: CGFloat = 0.06
        static let downloadCeiling: CGFloat = 0.78
        static let extractFloor: CGFloat = 0.82
        static let extractCeiling: CGFloat = 0.92
        static let installFloor: CGFloat = 0.84
    }

    private struct PackageRuntimeState {
        var stage: PackageStage = .queued
        var lastRenderedProgress: CGFloat = ProgressLayout.queued
        var lastSpeed: String?
        var lastDownloadUpdateAt: Date?
        var lastDownloadProgress: CGFloat = 0
        var didLogDownloadStart = false
        var observedDownload = false
    }

    private final class RootView: NSView {
        let backdrop = NSView(frame: .zero)
        let panel = NSView(frame: .zero)
        let titleField = NSTextField(labelWithString: "")
        let operationField = NSTextField(labelWithString: "")
        let statusField = NSTextField(labelWithString: "")
        let packageScrollView = NSScrollView(frame: .zero)
        let packageListView = PackageProgressListView()
        let logScrollView = NSScrollView(frame: .zero)
        let logView = NSTextView(frame: .zero)
        let primaryButton = NSButton(title: "Abort", target: nil, action: nil)
        let secondaryButton = NSButton(title: "Dismiss", target: nil, action: nil)
        var onCancel: (() -> Void)?

        override init(frame frameRect: NSRect) {
            super.init(frame: frameRect)
            wantsLayer = true
            layer = CALayer()

            backdrop.wantsLayer = true
            backdrop.layer = CALayer()
            backdrop.layer?.backgroundColor = NSColor(calibratedWhite: 0, alpha: 0.6).cgColor
            addSubview(backdrop)

            panel.wantsLayer = true
            panel.layer = CALayer()
            panel.layer?.backgroundColor = UIStyle.surface.cgColor
            panel.layer?.borderColor = UIStyle.separator.cgColor
            panel.layer?.borderWidth = 1
            panel.layer?.cornerRadius = 12
            panel.layer?.shadowColor = UIStyle.accentShadow.cgColor
            panel.layer?.shadowOpacity = 1
            panel.layer?.shadowRadius = 18
            panel.layer?.shadowOffset = .zero
            addSubview(panel)

            [titleField, operationField, statusField].forEach {
                $0.isEditable = false
                $0.isBordered = false
                $0.drawsBackground = false
                panel.addSubview($0)
            }
            titleField.font = UIStyle.monoFont(size: 15, weight: .medium)
            titleField.textColor = UIStyle.text
            operationField.font = UIStyle.monoFont(size: 12, weight: .medium)
            operationField.textColor = UIStyle.accent
            statusField.font = UIStyle.monoFont(size: 10, weight: .regular)
            statusField.textColor = UIStyle.quietText

            packageScrollView.drawsBackground = false
            packageScrollView.borderType = .noBorder
            packageScrollView.hasHorizontalScroller = false
            packageScrollView.hasVerticalScroller = true
            packageScrollView.autohidesScrollers = true
            packageScrollView.contentView.postsBoundsChangedNotifications = true
            packageScrollView.documentView = packageListView
            panel.addSubview(packageScrollView)

            logView.isEditable = false
            logView.isSelectable = true
            logView.drawsBackground = false
            logView.textColor = UIStyle.dimText
            logView.font = UIStyle.monoFont(size: 11)
            logView.textContainerInset = NSSize(width: 0, height: 8)
            logScrollView.drawsBackground = false
            logScrollView.borderType = .noBorder
            logScrollView.hasVerticalScroller = true
            logScrollView.documentView = logView
            panel.addSubview(logScrollView)

            [primaryButton, secondaryButton].forEach {
                $0.wantsLayer = true
                $0.layer = CALayer()
                $0.isBordered = false
                $0.font = UIStyle.monoFont(size: 11, weight: .medium)
                UIStyle.applyControlChrome(
                    to: $0.layer,
                    chrome: UIStyle.ControlChrome(
                        topBackgroundColor: NSColor.white.withAlphaComponent(0.03),
                        bottomBackgroundColor: NSColor.white.withAlphaComponent(0.012),
                        borderColor: UIStyle.accent.withAlphaComponent(0.12),
                        contentColor: UIStyle.dimText,
                        topInnerStrokeColor: NSColor.white.withAlphaComponent(0.04),
                        bottomInnerStrokeColor: NSColor.black.withAlphaComponent(0.18)
                    )
                )
                panel.addSubview($0)
            }
            secondaryButton.isHidden = true
        }

        required init?(coder: NSCoder) {
            fatalError("init(coder:) has not been implemented")
        }

        override var acceptsFirstResponder: Bool {
            true
        }

        override func keyDown(with event: NSEvent) {
            if event.keyCode == 53 {
                onCancel?()
                return
            }
            super.keyDown(with: event)
        }

        override func cancelOperation(_ sender: Any?) {
            onCancel?()
        }

        override func layout() {
            super.layout()
            backdrop.frame = bounds
            let panelWidth = min(max(bounds.width * 0.54, 560), 780)
            let panelHeight = min(max(bounds.height * 0.70, 520), 720)
            panel.frame = CGRect(
                x: (bounds.width - panelWidth) / 2,
                y: (bounds.height - panelHeight) / 2,
                width: panelWidth,
                height: panelHeight
            )

            let inset: CGFloat = 24
            titleField.frame = CGRect(
                x: inset,
                y: panelHeight - 42,
                width: panelWidth - inset * 2,
                height: 20
            )
            operationField.frame = CGRect(
                x: inset,
                y: panelHeight - 68,
                width: panelWidth - inset * 2,
                height: 18
            )
            statusField.frame = CGRect(
                x: inset,
                y: panelHeight - 90,
                width: panelWidth - inset * 2,
                height: 16
            )

            let rowContentHeight = PackageProgressListMetrics.contentHeight(
                rowCount: packageListView.subviews.count
            )
            let buttonY: CGFloat = 24
            let buttonHeight: CGFloat = 30
            let logBottom = buttonY + buttonHeight + 16
            let contentTop = panelHeight - 112
            let packageLogGap: CGFloat = 14
            let availableContentHeight = contentTop - logBottom - packageLogGap
            let maxPackageHeight = min(320, max(120, availableContentHeight * 0.70))
            let packageHeight = min(rowContentHeight, maxPackageHeight)
            let packageY = contentTop - packageHeight
            let logHeight = max(104, packageY - packageLogGap - logBottom)

            packageScrollView.frame = CGRect(
                x: inset,
                y: packageY,
                width: panelWidth - inset * 2,
                height: packageHeight
            )
            packageScrollView.hasVerticalScroller = rowContentHeight > packageHeight + 0.5
            packageListView.frame = CGRect(
                x: 0,
                y: 0,
                width: packageScrollView.contentSize.width,
                height: max(rowContentHeight, packageScrollView.contentSize.height)
            )
            packageListView.needsLayout = true

            logScrollView.frame = CGRect(
                x: inset,
                y: logBottom,
                width: panelWidth - inset * 2,
                height: logHeight
            )
            primaryButton.frame = CGRect(
                x: panelWidth - 128,
                y: buttonY,
                width: 104,
                height: buttonHeight
            )
            secondaryButton.frame = CGRect(
                x: panelWidth - 240,
                y: buttonY,
                width: 104,
                height: buttonHeight
            )
            UIStyle.layoutControlChrome(in: primaryButton.layer)
            UIStyle.layoutControlChrome(in: secondaryButton.layer)
        }
    }

    var onRetry: (() -> Void)?
    var onDismiss: (() -> Void)?

    private var rows: [String: PackageProgressRowView] = [:]
    private var orderedPackages: [String] = []
    private var visiblePackages: Set<String> = []
    private var acceptsNewVisiblePackages = true
    private var packageStates: [String: PackageRuntimeState] = [:]
    private var isTerminalState = false
    private var channelTitle = "NUCLEUS UPDATE CHANNEL"
    private var awaitingClearanceText = "Awaiting clearance"
    private var idleStatusText = "Nucleus idle"
    private var successOperationTitle = "Update Complete"
    private var failureOperationTitle = "Update Halted"
    private var operationAnimator: GlitchTextAnimator?

    override func loadView() {
        view = RootView(frame: .zero)
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        guard let rootView = view as? RootView else { return }
        rootView.onCancel = { [weak self] in
            self?.onDismiss?()
        }
        operationAnimator = GlitchTextAnimator(
            field: rootView.operationField,
            size: 12,
            baseColor: UIStyle.accent,
            glitchColor: UIStyle.text.withAlphaComponent(0.86)
        )
        rootView.titleField.stringValue = channelTitle
        operationAnimator?.setText(awaitingClearanceText, animated: true)
        rootView.statusField.stringValue = idleStatusText
        rootView.primaryButton.target = self
        rootView.primaryButton.action = #selector(primaryAction)
        rootView.secondaryButton.target = self
        rootView.secondaryButton.action = #selector(secondaryAction)
    }

    func configure(
        title: String,
        awaitingClearance: String,
        idleStatus: String,
        successOperation: String,
        failureOperation: String
    ) {
        channelTitle = title
        awaitingClearanceText = awaitingClearance
        idleStatusText = idleStatus
        successOperationTitle = successOperation
        failureOperationTitle = failureOperation
        guard let rootView = view as? RootView else { return }
        rootView.titleField.stringValue = title
        operationAnimator?.setText(awaitingClearance, animated: true)
        rootView.statusField.stringValue = idleStatus
    }

    func begin(packages: [String], activationLog: String) {
        orderedPackages = packages
        visiblePackages = Set(packages)
        acceptsNewVisiblePackages = packages.isEmpty
        packageStates = Dictionary(
            uniqueKeysWithValues: packages.map { ($0, PackageRuntimeState()) }
        )
        isTerminalState = false
        if let rootView = view as? RootView {
            rootView.logView.string = ""
        }
        appendLog(activationLog)
        if packages.isEmpty {
            appendLog("Awaiting package plan from nucleus.")
        }
        updateButtons(primaryTitle: "Abort", showSecondary: false)
        rebuildRows()
        resetPackageScrollPosition()
    }

    func handle(event: NukeHelperProgressEvent) {
        switch event {
        case .resolving:
            setOperation("Resolving package graph")
            appendLog("Resolving package graph")
            orderedPackages.forEach {
                updateRow(
                    package: $0,
                    stage: .resolving,
                    progress: ProgressLayout.resolving,
                    speed: nil
                )
            }
        case .downloading(let package, let bytesPerSecond, let progress):
            let visiblePackage = visiblePackageName(
                forProgressPackage: package,
                allowsNewProgressPackage: true
            )
            let isVisiblePackage = visiblePackage != nil
            let rowPackage = visiblePackage ?? package
            let speedText = Self.format(speed: bytesPerSecond)
            setOperation("Updating \(package)")
            if shouldLogDownloadStart(for: package) {
                appendLog("Downloading \(package)")
            }
            guard isVisiblePackage else { return }
            guard shouldRenderDownloadUpdate(for: package, progress: CGFloat(progress)) else {
                return
            }
            updateRow(
                package: rowPackage,
                stage: .downloading,
                progress: downloadProgress(for: CGFloat(progress)),
                speed: speedText
            )
        case .installing(let package):
            let visiblePackage = visiblePackageName(
                forProgressPackage: package,
                allowsNewProgressPackage: true
            )
            let isVisiblePackage = visiblePackage != nil
            let rowPackage = visiblePackage ?? package
            let state = packageStates[rowPackage] ?? packageStates[package] ?? PackageRuntimeState()
            if state.observedDownload {
                setOperation("Extracting \(package)")
                appendLog("Extracting \(package)")
                guard isVisiblePackage else { return }
                updateRow(
                    package: rowPackage,
                    stage: .extracting,
                    progress: extractProgress(from: state.lastRenderedProgress),
                    speed: nil
                )
            } else {
                setOperation("Installing \(package)")
                appendLog("Installing \(package)")
                guard isVisiblePackage else { return }
                updateRow(
                    package: rowPackage,
                    stage: .installing,
                    progress: ProgressLayout.installFloor,
                    speed: nil
                )
            }
        case .log(let package, let message):
            _ = visiblePackageName(
                forProgressPackage: package,
                allowsNewProgressPackage: true
            ).map(track)
            setOperation(Self.sentenceCase(message))
            appendLog("\(package): \(message)")
        case .completed(let package):
            let visiblePackage = visiblePackageName(forProgressPackage: package)
            let isVisiblePackage = visiblePackage != nil
            let rowPackage = visiblePackage ?? package
            setOperation("Sealing \(package)")
            appendLog("Completed \(package)")
            guard isVisiblePackage else { return }
            updateRow(package: rowPackage, stage: .completed, progress: 1, speed: nil)
        case .error(let message):
            fail(message: message)
        }
    }

    func succeed(message: String, packages: [String]) {
        packages
            .compactMap { visiblePackageName(forProgressPackage: $0) }
            .forEach { updateRow(package: $0, stage: .completed, progress: 1, speed: nil) }
        isTerminalState = true
        operationAnimator?.stop()
        setOperation(successOperationTitle)
        setStatus(message, color: UIStyle.accent)
        appendLog(message)
        animateSuccessPulse()
        updateButtons(primaryTitle: "Dismiss", showSecondary: false)
    }

    func fail(message: String) {
        isTerminalState = true
        operationAnimator?.stop()
        setOperation(failureOperationTitle)
        setStatus(message, color: UIStyle.danger)
        appendLog("FAULT: \(message)")
        if let current = orderedPackages.last {
            updateRow(package: current, stage: .failed, progress: 1, speed: nil)
        }
        updateButtons(primaryTitle: "Retry", showSecondary: true)
    }

    @discardableResult
    private func track(_ package: String) -> Bool {
        if acceptsNewVisiblePackages {
            visiblePackages.insert(package)
        }
        guard visiblePackages.contains(package) else {
            packageStates[package] = packageStates[package] ?? PackageRuntimeState()
            return false
        }
        guard rows[package] == nil else { return true }
        orderedPackages.append(package)
        packageStates[package] = packageStates[package] ?? PackageRuntimeState()
        rebuildRows()
        return true
    }

    private func visiblePackageName(
        forProgressPackage package: String,
        allowsNewProgressPackage: Bool = false
    ) -> String? {
        if acceptsNewVisiblePackages {
            visiblePackages.insert(package)
        }
        if visiblePackages.contains(package) {
            return package
        }
        let qualifiedCandidates = [
            "brew:\(package)",
            "cask:\(package)"
        ].filter { visiblePackages.contains($0) }
        guard qualifiedCandidates.count == 1 else {
            if allowsNewProgressPackage {
                visiblePackages.insert(package)
                packageStates[package] = packageStates[package] ?? PackageRuntimeState()
                return package
            }
            packageStates[package] = packageStates[package] ?? PackageRuntimeState()
            return nil
        }
        let visiblePackage = qualifiedCandidates[0]
        packageStates[visiblePackage] = packageStates[visiblePackage]
            ?? packageStates[package]
            ?? PackageRuntimeState()
        return visiblePackage
    }

    private func rebuildRows() {
        guard let rootView = view as? RootView else { return }
        let existingRows = rows
        rows.removeAll(keepingCapacity: true)
        rootView.packageListView.subviews.forEach { subview in
            subview.removeFromSuperview()
        }
        for package in orderedPackages {
            let row = existingRows[package] ?? PackageProgressRowView(frame: .zero)
            rows[package] = row
            let state = packageStates[package] ?? PackageRuntimeState()
            row.render(
                name: package,
                stage: state.stage,
                progress: state.lastRenderedProgress,
                speed: state.lastSpeed,
                animated: false
            )
            rootView.packageListView.addSubview(row)
        }
        rootView.needsLayout = true
        resetPackageScrollPosition()
    }

    private func updateRow(package: String, stage: PackageStage, progress: CGFloat, speed: String?) {
        guard track(package) else {
            return
        }
        var state = packageStates[package] ?? PackageRuntimeState()
        state.stage = stage
        state.lastRenderedProgress = progress
        state.lastSpeed = speed
        if stage == .downloading {
            state.observedDownload = true
        } else {
            state.didLogDownloadStart = false
            state.lastDownloadUpdateAt = nil
        }
        packageStates[package] = state
        rows[package]?.render(
            name: package,
            stage: stage,
            progress: progress,
            speed: speed,
            animated: stage != .downloading
        )
    }

    private func resetPackageScrollPosition() {
        guard let rootView = view as? RootView else { return }
        rootView.packageScrollView.contentView.scroll(to: .zero)
        rootView.packageScrollView.reflectScrolledClipView(rootView.packageScrollView.contentView)
    }

    private func shouldLogDownloadStart(for package: String) -> Bool {
        var state = packageStates[package] ?? PackageRuntimeState()
        defer { packageStates[package] = state }
        if state.didLogDownloadStart {
            return false
        }
        state.didLogDownloadStart = true
        return true
    }

    private func shouldRenderDownloadUpdate(for package: String, progress: CGFloat) -> Bool {
        let now = Date()
        var state = packageStates[package] ?? PackageRuntimeState()
        defer {
            state.lastDownloadUpdateAt = now
            state.lastDownloadProgress = progress
            packageStates[package] = state
        }
        let progressDelta = abs(progress - state.lastDownloadProgress)
        if progress >= 0.99 || progressDelta >= 0.015 {
            return true
        }
        guard let lastUpdate = state.lastDownloadUpdateAt else {
            return true
        }
        return now.timeIntervalSince(lastUpdate) >= 0.05
    }

    private func setOperation(_ text: String) {
        operationAnimator?.setText(text, animated: !isTerminalState)
    }

    private func setStatus(_ text: String, color: NSColor = UIStyle.quietText) {
        guard let rootView = view as? RootView else { return }
        rootView.statusField.stringValue = text
        rootView.statusField.textColor = color
    }

    private func appendLog(_ line: String) {
        guard let rootView = view as? RootView else { return }
        let prefix = DateFormatter.localizedString(
            from: Date(),
            dateStyle: .none,
            timeStyle: .medium
        )
        let existing = rootView.logView.string
        rootView.logView.string = existing + "[\(prefix)] \(line)\n"
        rootView.logView.scrollToEndOfDocument(nil)
    }

    private func animateSuccessPulse() {
        guard let rootView = view as? RootView else { return }
        CATransaction.begin()
        CATransaction.setAnimationDuration(0.28)
        rootView.panel.layer?.shadowRadius = 28
        rootView.panel.layer?.shadowColor = UIStyle.accent.cgColor
        CATransaction.commit()
    }

    private func updateButtons(primaryTitle: String, showSecondary: Bool) {
        guard let rootView = view as? RootView else { return }
        rootView.primaryButton.title = primaryTitle
        rootView.secondaryButton.isHidden = !showSecondary
    }

    @objc private func primaryAction() {
        if isTerminalState {
            if (view as? RootView)?.primaryButton.title == "Retry" {
                onRetry?()
            } else {
                onDismiss?()
            }
            return
        }
        onDismiss?()
    }

    @objc private func secondaryAction() {
        onDismiss?()
    }

    private static func format(speed: UInt64) -> String {
        if speed >= 1_000_000 {
            return String(format: "%.1f MB/s", Double(speed) / 1_000_000)
        }
        if speed >= 1_000 {
            return String(format: "%.0f KB/s", Double(speed) / 1_000)
        }
        return "\(speed) B/s"
    }

    private static func sentenceCase(_ message: String) -> String {
        guard let first = message.first else { return message }
        return first.uppercased() + message.dropFirst()
    }

    private func downloadProgress(for progress: CGFloat) -> CGFloat {
        let normalized = max(0, min(progress, 1))
        return ProgressLayout.downloadFloor
            + normalized * (ProgressLayout.downloadCeiling - ProgressLayout.downloadFloor)
    }

    private func extractProgress(from currentProgress: CGFloat) -> CGFloat {
        max(currentProgress, ProgressLayout.extractFloor)
            .clamped(to: ProgressLayout.extractFloor ... ProgressLayout.extractCeiling)
    }
}

private extension Comparable {
    func clamped(to limits: ClosedRange<Self>) -> Self {
        min(max(self, limits.lowerBound), limits.upperBound)
    }
}
