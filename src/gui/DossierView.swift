import AppKit
import QuartzCore

protocol DossierViewDelegate: AnyObject {
    func dossierView(_ view: DossierView, didRequestPrimaryActionFor detail: PackageDetail)
    func dossierView(_ view: DossierView, didRequestUpdateActionFor detail: PackageDetail)
    func dossierView(_ view: DossierView, didRequestDefaultActionFor detail: PackageDetail)
    func dossierView(_ view: DossierView, didRequestMigrationActionFor detail: PackageDetail)
    func dossierView(_ view: DossierView, didRequestSecurityActionFor detail: PackageDetail)
}

private final class DossierActionButton: NSButton {
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

private final class DossierNoticeTextField: NSTextField {
    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        isEditable = false
        isSelectable = false
        isBordered = false
        drawsBackground = false
        usesSingleLineMode = false
        lineBreakMode = .byWordWrapping
        maximumNumberOfLines = 0
        cell?.wraps = true
        cell?.isScrollable = false
        wantsLayer = true
        layer?.opacity = 1
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    var attributedText: NSAttributedString? {
        get {
            attributedStringValue.length == 0 ? nil : attributedStringValue
        }
        set {
            attributedStringValue = newValue ?? NSAttributedString(string: "")
            isHidden = newValue == nil
        }
    }
}

final class DossierView: NSView {
    enum RenderAnimation {
        case none
        case full
        case incremental
    }

    private struct Metrics {
        static let panelHeaderTopInset: CGFloat = 6
        static let panelHeaderSideInset: CGFloat = 6
        static let panelHeaderLabelHeight: CGFloat = 10
        static let topInset: CGFloat = 28
        static let sideInset: CGFloat = 10
        static let spineX: CGFloat = 7
        static let textInset: CGFloat = 10
        static let titleGap: CGFloat = 0
        static let bodyGap: CGFloat = 12
        static let sectionGap: CGFloat = 18
        static let securityNoticeTopGap: CGFloat = 2
        static let securityNoticePadding: CGFloat = 10
        static let securityNoticeInnerGap: CGFloat = 8
        static let securityNoticeIconWidth: CGFloat = 34
        static let securityNoticeIconHeight: CGFloat = 40
        static let securityNoticeHeadlineTopInset: CGFloat = 6
        static let securityNoticeDetailTopGap: CGFloat = 8
        static let securityNoticeDetailHeaderHeight: CGFloat = 12
        static let securityNoticeDetailBodyTopGap: CGFloat = 7
        static let securityNoticeButtonTopGap: CGFloat = 10
        static let securityNoticeButtonHeight: CGFloat = 28
        static let securityNoticeButtonGap: CGFloat = 10
        static let securityNoticeButtonHorizontalPadding: CGFloat = 26
        static let sectionBodyGap: CGFloat = 15
        static let listSectionTopGap: CGFloat = 5
        static let popularityToDependenciesGap: CGFloat = 16
        static let lastUpdatedToDependenciesGap: CGFloat = 16
        static let dependencyColumnGap: CGFloat = 12
        static let dependencyColumnMinWidth: CGFloat = 90
        static let dependencyRowGap: CGFloat = 20
        static let dependencyToInstallGap: CGFloat = 16
        static let executableRowGap: CGFloat = 20
        static let executableToDestinationGap: CGFloat = 16
        static let destinationToInstallGap: CGFloat = 16
        static let versionSelectorGap: CGFloat = 10
        static let versionSelectorHeight: CGFloat = 26
        static let versionHintTopGap: CGFloat = 7
        static let versionHintBottomGap: CGFloat = 16
        static let installCommandTopGap: CGFloat = 6
        static let installCommandVisualLift: CGFloat = 6
        static let commandToButtonGap: CGFloat = 14
        static let actionButtonHeight: CGFloat = 28
        static let actionButtonWidth: CGFloat = 112
        static let actionButtonGap: CGFloat = 10
        static let fudge: CGFloat = 15
        static let bottomInset: CGFloat = 18
    }

    private struct DependencyCluster {
        let name: String
        let baseAlpha: CGFloat
        let isInstalled: Bool
        let layer: CATextLayer
    }

    private struct LayoutTransitionFrames {
        let commandHeader: CGRect
        let installCommand: CGRect
        let updateButton: CGRect
        let primaryActionButton: CGRect
    }

    private struct Timing {
        static let reveal: CFTimeInterval = 0.18
        static let rowStep: CFTimeInterval = 0.02
    }

    private let spineLayer = CALayer()
    private let panelHeaderLayer = CATextLayer()
    private let titleLayer = CATextLayer()
    private let metadataLayer = CATextLayer()
    private let descriptionLayer = CATextLayer()
    private let securityNoticePanelLayer = CALayer()
    private let securityNoticeIconLayer = CATextLayer()
    private let securityNoticeHeadlineLayer = CATextLayer()
    private let securityNoticeBodyField = DossierNoticeTextField(frame: .zero)
    private let securityNoticeReasonsHeaderLayer = CATextLayer()
    private let securityNoticeReasonsBodyField = DossierNoticeTextField(frame: .zero)
    private let securityNoticeCaveatsHeaderLayer = CATextLayer()
    private let securityNoticeCaveatsBodyField = DossierNoticeTextField(frame: .zero)
    private let popularityHeaderLayer = CATextLayer()
    private let popularityLayer = CATextLayer()
    private let lastUpdatedHeaderLayer = CATextLayer()
    private let lastUpdatedLayer = CATextLayer()
    private let dependenciesHeaderLayer = CATextLayer()
    private let executablesHeaderLayer = CATextLayer()
    private let installDestinationHeaderLayer = CATextLayer()
    private let installDestinationLayer = CATextLayer()
    private let versionSelectorHeaderLayer = CATextLayer()
    private let versionSelectorHintLayer = CATextLayer()
    private let versionSelector = NSPopUpButton(frame: .zero, pullsDown: false)
    private let commandHeaderLayer = CATextLayer()
    private let installCommandLayer = CATextLayer()
    private let updateButton = DossierActionButton(title: "UPDATE", target: nil, action: nil)
    private let primaryActionButton = DossierActionButton(
        title: "INSTALL",
        target: nil,
        action: nil
    )
    private let makeDefaultButton = DossierActionButton(
        title: "MAKE DEFAULT",
        target: nil,
        action: nil
    )
    private let securityLearnMoreButton = DossierActionButton(
        title: "LEARN MORE",
        target: nil,
        action: nil
    )
    private let securityApplyButton = DossierActionButton(
        title: "REINSTALL AS ISOTOPE",
        target: nil,
        action: nil
    )

    private var dependencyClusters: [DependencyCluster] = []
    private var dependencyFrames: [String: CGRect] = [:]
    private var executableClusters: [DependencyCluster] = []
    private var executableFrames: [String: CGRect] = [:]
    private var hoveredDependencyName: String?
    private var hoveredExecutableName: String?
    private var executableURLs: [String: URL] = [:]
    private var installDestinationFrame: CGRect = .zero
    private var installDestinationURL: URL?
    private var selectedVersionOptionPackageName: String?
    private var isHoveringInstallDestination = false
    private var trackingArea: NSTrackingArea?
    private var currentDetail: PackageDetail?
    private var currentLastUpdatedDate: Date?
    private var isHoveringLastUpdated = false
    private var isActionInFlight = false
    private var isEyebrowLoading = false
    private var panelHeaderAnimator: LayerGlitchTextAnimator?
    private var currentSecurityNotice: PackageSecurityNotice?

    weak var delegate: DossierViewDelegate?

    override var isFlipped: Bool {
        true
    }

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        wantsLayer = true
        layer = CALayer()
        layer?.backgroundColor = UIStyle.background.cgColor

        spineLayer.backgroundColor = UIStyle.spine.cgColor
        layer?.addSublayer(spineLayer)

        securityNoticePanelLayer.backgroundColor = UIStyle.danger.withAlphaComponent(0.08).cgColor
        securityNoticePanelLayer.borderColor = UIStyle.danger.withAlphaComponent(0.30).cgColor
        securityNoticePanelLayer.borderWidth = 1
        securityNoticePanelLayer.cornerRadius = 2
        securityNoticePanelLayer.isHidden = true
        layer?.addSublayer(securityNoticePanelLayer)

        panelHeaderLayer.contentsScale = NSScreen.main?.backingScaleFactor ?? 2
        panelHeaderLayer.alignmentMode = .left
        layer?.addSublayer(panelHeaderLayer)
        panelHeaderAnimator = LayerGlitchTextAnimator(
            layer: panelHeaderLayer,
            size: 10,
            baseColor: UIStyle.text.withAlphaComponent(0.20),
            glitchColor: UIStyle.accent.withAlphaComponent(0.66),
            weight: .regular,
            tracking: 1.8
        )
        updatePanelHeader()

        for textLayer in [
            titleLayer,
            metadataLayer,
            descriptionLayer,
            securityNoticeIconLayer,
            securityNoticeHeadlineLayer,
            securityNoticeReasonsHeaderLayer,
            securityNoticeCaveatsHeaderLayer,
            popularityHeaderLayer,
            popularityLayer,
            lastUpdatedHeaderLayer,
            lastUpdatedLayer,
            dependenciesHeaderLayer,
            executablesHeaderLayer,
            installDestinationHeaderLayer,
            installDestinationLayer,
            versionSelectorHeaderLayer,
            versionSelectorHintLayer,
            commandHeaderLayer,
            installCommandLayer
        ] {
            textLayer.contentsScale = NSScreen.main?.backingScaleFactor ?? 2
            textLayer.alignmentMode = .left
            textLayer.isWrapped = true
            textLayer.actions = [
                "bounds": NSNull(),
                "position": NSNull(),
                "string": NSNull(),
                "opacity": NSNull()
            ]
            layer?.addSublayer(textLayer)
        }

        for noticeTextField in [
            securityNoticeBodyField,
            securityNoticeReasonsBodyField,
            securityNoticeCaveatsBodyField
        ] {
            noticeTextField.isHidden = true
            addSubview(noticeTextField)
        }

        popularityHeaderLayer.string = UIStyle.sectionHeaderText("Popularity")
        lastUpdatedHeaderLayer.string = UIStyle.sectionHeaderText("Last Update")
        dependenciesHeaderLayer.string = UIStyle.sectionHeaderText("Dependencies")
        executablesHeaderLayer.string = UIStyle.sectionHeaderText("Executables")
        installDestinationHeaderLayer.string = UIStyle.sectionHeaderText("Install Destination")
        versionSelectorHeaderLayer.string = UIStyle.sectionHeaderText("Version")
        commandHeaderLayer.string = UIStyle.sectionHeaderText("Install Vector")

        updateButton.font = UIStyle.monoFont(size: 11, weight: .medium)
        updateButton.palette = DossierActionButton.Palette(
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
        updateButton.target = self
        updateButton.action = #selector(handleUpdateAction)
        updateButton.isHidden = true
        addSubview(updateButton)

        primaryActionButton.font = UIStyle.monoFont(size: 11, weight: .medium)
        primaryActionButton.palette = DossierActionButton.Palette(
            baseChrome: UIStyle.ControlChrome(
                topBackgroundColor: NSColor.white.withAlphaComponent(0.03),
                bottomBackgroundColor: NSColor.white.withAlphaComponent(0.012),
                borderColor: UIStyle.accent.withAlphaComponent(0.12),
                contentColor: UIStyle.dimText,
                topInnerStrokeColor: NSColor.white.withAlphaComponent(0.04),
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
        primaryActionButton.target = self
        primaryActionButton.action = #selector(handlePrimaryAction)
        primaryActionButton.isHidden = true
        addSubview(primaryActionButton)

        makeDefaultButton.font = UIStyle.monoFont(size: 11, weight: .medium)
        makeDefaultButton.palette = primaryActionButton.palette
        makeDefaultButton.target = self
        makeDefaultButton.action = #selector(handleMakeDefaultAction)
        makeDefaultButton.isHidden = true
        addSubview(makeDefaultButton)

        versionSelector.font = UIStyle.monoFont(size: 11, weight: .regular)
        versionSelector.target = self
        versionSelector.action = #selector(handleVersionSelection)
        versionSelector.isHidden = true
        addSubview(versionSelector)

        securityLearnMoreButton.font = UIStyle.monoFont(size: 11, weight: .medium)
        securityLearnMoreButton.palette = DossierActionButton.Palette(
            baseChrome: UIStyle.ControlChrome(
                topBackgroundColor: NSColor.white.withAlphaComponent(0.03),
                bottomBackgroundColor: NSColor.white.withAlphaComponent(0.015),
                borderColor: UIStyle.danger.withAlphaComponent(0.26),
                contentColor: UIStyle.danger.withAlphaComponent(0.92),
                topInnerStrokeColor: NSColor.white.withAlphaComponent(0.05),
                bottomInnerStrokeColor: NSColor.black.withAlphaComponent(0.18)
            ),
            hoverChrome: UIStyle.ControlChrome(
                topBackgroundColor: UIStyle.danger.withAlphaComponent(0.10),
                bottomBackgroundColor: UIStyle.danger.withAlphaComponent(0.06),
                borderColor: UIStyle.danger.withAlphaComponent(0.52),
                contentColor: UIStyle.text,
                topInnerStrokeColor: NSColor.white.withAlphaComponent(0.12),
                bottomInnerStrokeColor: NSColor.black.withAlphaComponent(0.20)
            ),
            disabledChrome: UIStyle.ControlChrome(
                topBackgroundColor: NSColor.white.withAlphaComponent(0.02),
                bottomBackgroundColor: NSColor.white.withAlphaComponent(0.01),
                borderColor: UIStyle.danger.withAlphaComponent(0.10),
                contentColor: UIStyle.danger.withAlphaComponent(0.34),
                topInnerStrokeColor: NSColor.white.withAlphaComponent(0.04),
                bottomInnerStrokeColor: NSColor.black.withAlphaComponent(0.14)
            )
        )
        securityLearnMoreButton.target = self
        securityLearnMoreButton.action = #selector(handleSecurityLearnMore)
        securityLearnMoreButton.isHidden = true
        addSubview(securityLearnMoreButton)

        securityApplyButton.font = UIStyle.monoFont(size: 11, weight: .medium)
        securityApplyButton.palette = DossierActionButton.Palette(
            baseChrome: UIStyle.ControlChrome(
                topBackgroundColor: UIStyle.danger.withAlphaComponent(0.14),
                bottomBackgroundColor: UIStyle.danger.withAlphaComponent(0.08),
                borderColor: UIStyle.danger.withAlphaComponent(0.40),
                contentColor: UIStyle.text,
                topInnerStrokeColor: NSColor.white.withAlphaComponent(0.10),
                bottomInnerStrokeColor: NSColor.black.withAlphaComponent(0.18)
            ),
            hoverChrome: UIStyle.ControlChrome(
                topBackgroundColor: UIStyle.danger.withAlphaComponent(0.22),
                bottomBackgroundColor: UIStyle.danger.withAlphaComponent(0.16),
                borderColor: UIStyle.danger.withAlphaComponent(0.64),
                contentColor: UIStyle.text,
                topInnerStrokeColor: NSColor.white.withAlphaComponent(0.14),
                bottomInnerStrokeColor: NSColor.black.withAlphaComponent(0.22)
            ),
            disabledChrome: UIStyle.ControlChrome(
                topBackgroundColor: UIStyle.danger.withAlphaComponent(0.06),
                bottomBackgroundColor: UIStyle.danger.withAlphaComponent(0.03),
                borderColor: UIStyle.danger.withAlphaComponent(0.12),
                contentColor: UIStyle.text.withAlphaComponent(0.40),
                topInnerStrokeColor: NSColor.white.withAlphaComponent(0.05),
                bottomInnerStrokeColor: NSColor.black.withAlphaComponent(0.14)
            )
        )
        securityApplyButton.target = self
        securityApplyButton.action = #selector(handleSecurityApply)
        securityApplyButton.isHidden = true
        addSubview(securityApplyButton)

        render(detail: nil)
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        if let trackingArea {
            removeTrackingArea(trackingArea)
        }
        let options: NSTrackingArea.Options = [
            .activeInKeyWindow,
            .mouseMoved,
            .mouseEnteredAndExited,
            .cursorUpdate,
            .inVisibleRect
        ]
        let area = NSTrackingArea(rect: bounds, options: options, owner: self)
        trackingArea = area
        addTrackingArea(area)
    }

    override func mouseMoved(with event: NSEvent) {
        let point = convert(event.locationInWindow, from: nil)
        let hoveredDependencyName = dependencyFrames.first(where: { $0.value.contains(point) })?.key
        let hoveredExecutableName = executableFrames.first(where: { $0.value.contains(point) })?.key
        updateHoveredDependency(name: hoveredDependencyName)
        updateHoveredExecutable(name: hoveredExecutableName)
        updateHoveredLastUpdated(isActive: lastUpdatedLayer.frame.contains(point))
        updateHoveredInstallDestination(isActive: installDestinationFrame.contains(point))
    }

    override func mouseExited(with event: NSEvent) {
        updateHoveredDependency(name: nil)
        updateHoveredExecutable(name: nil)
        updateHoveredLastUpdated(isActive: false)
        updateHoveredInstallDestination(isActive: false)
    }

    override func cursorUpdate(with event: NSEvent) {
        let point = convert(event.locationInWindow, from: nil)
        if installDestinationFrame.contains(point) || executableURL(at: point) != nil {
            NSCursor.pointingHand.set()
        } else {
            NSCursor.arrow.set()
        }
    }

    override func mouseDown(with event: NSEvent) {
        let point = convert(event.locationInWindow, from: nil)
        if let executableURL = executableURL(at: point) {
            NSWorkspace.shared.activateFileViewerSelecting([executableURL])
            return
        }
        if installDestinationFrame.contains(point), let installDestinationURL {
            NSWorkspace.shared.activateFileViewerSelecting([installDestinationURL])
            return
        }
        super.mouseDown(with: event)
    }

    override func layout() {
        super.layout()

        CATransaction.begin()
        CATransaction.setDisableActions(true)

        let contentMinX = Metrics.spineX + Metrics.textInset
        let contentWidth = max(
            bounds.width - contentMinX - Metrics.sideInset,
            120
        )
        let topY = Metrics.topInset

        spineLayer.frame = CGRect(
            x: Metrics.spineX,
            y: 0,
            width: 1,
            height: bounds.height
        )

        let panelHeaderMinX = Metrics.spineX + Metrics.textInset
        panelHeaderLayer.frame = CGRect(
            x: panelHeaderMinX,
            y: Metrics.panelHeaderTopInset,
            width: bounds.width - panelHeaderMinX - Metrics.panelHeaderSideInset,
            height: Metrics.panelHeaderLabelHeight
        )

        var cursorY = topY

        let titleHeight = heightForText(in: titleLayer, width: contentWidth, minimumHeight: 32)
        titleLayer.frame = CGRect(
            x: contentMinX,
            y: cursorY,
            width: contentWidth,
            height: titleHeight
        )
        cursorY = titleLayer.frame.maxY + Metrics.titleGap

        let metadataHeight = heightForText(
            in: metadataLayer,
            width: contentWidth,
            minimumHeight: 16
        )
        metadataLayer.frame = CGRect(
            x: contentMinX,
            y: cursorY,
            width: contentWidth,
            height: metadataHeight
        )
        cursorY = metadataLayer.frame.maxY + Metrics.bodyGap

        let descriptionHeight = heightForText(
            in: descriptionLayer,
            width: contentWidth,
            minimumHeight: 42
        )
        descriptionLayer.frame = CGRect(
            x: contentMinX,
            y: cursorY + (8 - Metrics.fudge),
            width: contentWidth,
            height: descriptionHeight
        )
        cursorY = descriptionLayer.frame.maxY + Metrics.sectionGap

        if currentSecurityNotice != nil {
            let panelMinX = contentMinX
            let panelWidth = contentWidth
            let noticeContentMinX = panelMinX + Metrics.securityNoticePadding
            let noticeContentWidth = max(panelWidth - Metrics.securityNoticePadding * 2, 120)
            var noticeCursorY = cursorY + Metrics.securityNoticeTopGap

            securityNoticeIconLayer.frame = CGRect(
                x: noticeContentMinX,
                y: noticeCursorY,
                width: Metrics.securityNoticeIconWidth,
                height: Metrics.securityNoticeIconHeight
            )
            let headlineMinX = securityNoticeIconLayer.frame.maxX
                + Metrics.securityNoticeInnerGap
            let headlineWidth = max(
                noticeContentWidth
                    - Metrics.securityNoticeIconWidth
                    - Metrics.securityNoticeInnerGap,
                40
            )

            let headlineHeight = heightForText(
                in: securityNoticeHeadlineLayer,
                width: headlineWidth,
                minimumHeight: 22
            )
            securityNoticeHeadlineLayer.frame = CGRect(
                x: headlineMinX,
                y: noticeCursorY + Metrics.securityNoticeHeadlineTopInset,
                width: headlineWidth,
                height: headlineHeight
            )
            noticeCursorY = max(
                securityNoticeIconLayer.frame.maxY,
                securityNoticeHeadlineLayer.frame.maxY
            )
                + Metrics.securityNoticeInnerGap

            let bodyHeight = heightForNoticeBody(width: noticeContentWidth)
            securityNoticeBodyField.frame = CGRect(
                x: noticeContentMinX,
                y: noticeCursorY,
                width: noticeContentWidth,
                height: bodyHeight
            )
            noticeCursorY = securityNoticeBodyField.frame.maxY

            if securityNoticeReasonsBodyField.attributedText != nil {
                noticeCursorY += Metrics.securityNoticeDetailTopGap
                securityNoticeReasonsHeaderLayer.frame = CGRect(
                    x: noticeContentMinX,
                    y: noticeCursorY,
                    width: noticeContentWidth,
                    height: Metrics.securityNoticeDetailHeaderHeight
                )
                noticeCursorY = securityNoticeReasonsHeaderLayer.frame.maxY
                    + Metrics.securityNoticeDetailBodyTopGap

                let reasonsHeight = heightForNoticeText(
                    in: securityNoticeReasonsBodyField,
                    width: noticeContentWidth,
                    minimumHeight: 16
                )
                securityNoticeReasonsBodyField.frame = CGRect(
                    x: noticeContentMinX,
                    y: noticeCursorY,
                    width: noticeContentWidth,
                    height: reasonsHeight
                )
                noticeCursorY = securityNoticeReasonsBodyField.frame.maxY
            } else {
                securityNoticeReasonsHeaderLayer.frame = .zero
                securityNoticeReasonsBodyField.frame = .zero
            }

            if securityNoticeCaveatsBodyField.attributedText != nil {
                noticeCursorY += Metrics.securityNoticeDetailTopGap
                securityNoticeCaveatsHeaderLayer.frame = CGRect(
                    x: noticeContentMinX,
                    y: noticeCursorY,
                    width: noticeContentWidth,
                    height: Metrics.securityNoticeDetailHeaderHeight
                )
                noticeCursorY = securityNoticeCaveatsHeaderLayer.frame.maxY
                    + Metrics.securityNoticeDetailBodyTopGap

                let caveatsHeight = heightForNoticeText(
                    in: securityNoticeCaveatsBodyField,
                    width: noticeContentWidth,
                    minimumHeight: 16
                )
                securityNoticeCaveatsBodyField.frame = CGRect(
                    x: noticeContentMinX,
                    y: noticeCursorY,
                    width: noticeContentWidth,
                    height: caveatsHeight
                )
                noticeCursorY = securityNoticeCaveatsBodyField.frame.maxY
            } else {
                securityNoticeCaveatsHeaderLayer.frame = .zero
                securityNoticeCaveatsBodyField.frame = .zero
            }

            noticeCursorY += Metrics.securityNoticeButtonTopGap

            let learnMoreButtonWidth = securityNoticeButtonWidth(for: securityLearnMoreButton)
            if securityApplyButton.isHidden {
                securityLearnMoreButton.frame = CGRect(
                    x: noticeContentMinX,
                    y: noticeCursorY,
                    width: noticeContentWidth,
                    height: Metrics.securityNoticeButtonHeight
                )
                securityApplyButton.frame = .zero
            } else {
                let applyButtonWidth = securityNoticeButtonWidth(for: securityApplyButton)
                let horizontalButtonWidth = max(
                    (noticeContentWidth - Metrics.securityNoticeButtonGap) / 2,
                    0
                )

                if learnMoreButtonWidth <= horizontalButtonWidth,
                   applyButtonWidth <= horizontalButtonWidth {
                    securityLearnMoreButton.frame = CGRect(
                        x: noticeContentMinX,
                        y: noticeCursorY,
                        width: horizontalButtonWidth,
                        height: Metrics.securityNoticeButtonHeight
                    )
                    securityApplyButton.frame = CGRect(
                        x: securityLearnMoreButton.frame.maxX + Metrics.securityNoticeButtonGap,
                        y: noticeCursorY,
                        width: horizontalButtonWidth,
                        height: Metrics.securityNoticeButtonHeight
                    )
                } else {
                    securityLearnMoreButton.frame = CGRect(
                        x: noticeContentMinX,
                        y: noticeCursorY,
                        width: noticeContentWidth,
                        height: Metrics.securityNoticeButtonHeight
                    )
                    securityApplyButton.frame = CGRect(
                        x: noticeContentMinX,
                        y: securityLearnMoreButton.frame.maxY + Metrics.securityNoticeButtonGap,
                        width: noticeContentWidth,
                        height: Metrics.securityNoticeButtonHeight
                    )
                }
            }
            UIStyle.layoutControlChrome(in: securityLearnMoreButton.layer)
            UIStyle.layoutControlChrome(in: securityApplyButton.layer)

            securityNoticePanelLayer.frame = CGRect(
                x: panelMinX,
                y: cursorY,
                width: panelWidth,
                height: max(
                    securityLearnMoreButton.frame.maxY,
                    securityApplyButton.frame.maxY
                ) - cursorY
                    + Metrics.securityNoticePadding
            )
            cursorY = securityNoticePanelLayer.frame.maxY + Metrics.sectionGap
        } else {
            securityNoticePanelLayer.frame = .zero
            securityNoticeIconLayer.frame = .zero
            securityNoticeHeadlineLayer.frame = .zero
            securityNoticeBodyField.frame = .zero
            securityNoticeReasonsHeaderLayer.frame = .zero
            securityNoticeReasonsBodyField.frame = .zero
            securityNoticeCaveatsHeaderLayer.frame = .zero
            securityNoticeCaveatsBodyField.frame = .zero
            securityLearnMoreButton.frame = .zero
            securityApplyButton.frame = .zero
        }

        if popularityHeaderLayer.string != nil {
            popularityHeaderLayer.frame = CGRect(
                x: contentMinX,
                y: cursorY,
                width: contentWidth,
                height: 16
            )
            cursorY = popularityHeaderLayer.frame.maxY + Metrics.sectionBodyGap - Metrics.fudge

            let popularityHeight = heightForText(
                in: popularityLayer,
                width: contentWidth,
                minimumHeight: 16
            )
            popularityLayer.frame = CGRect(
                x: contentMinX,
                y: cursorY,
                width: contentWidth,
                height: popularityHeight
            )
            cursorY = popularityLayer.frame.maxY + Metrics.popularityToDependenciesGap
        } else {
            popularityHeaderLayer.frame = .zero
            popularityLayer.frame = .zero
        }

        if lastUpdatedHeaderLayer.string != nil {
            lastUpdatedHeaderLayer.frame = CGRect(
                x: contentMinX,
                y: cursorY,
                width: contentWidth,
                height: 16
            )
            cursorY = lastUpdatedHeaderLayer.frame.maxY + Metrics.sectionBodyGap - Metrics.fudge

            let lastUpdatedHeight = heightForText(
                in: lastUpdatedLayer,
                width: contentWidth,
                minimumHeight: 16
            )
            lastUpdatedLayer.frame = CGRect(
                x: contentMinX,
                y: cursorY,
                width: contentWidth,
                height: lastUpdatedHeight
            )
            cursorY = lastUpdatedLayer.frame.maxY + Metrics.lastUpdatedToDependenciesGap
        } else {
            lastUpdatedHeaderLayer.frame = .zero
            lastUpdatedLayer.frame = .zero
        }

        if dependenciesHeaderLayer.string != nil {
            dependenciesHeaderLayer.frame = CGRect(
                x: contentMinX,
                y: cursorY,
                width: contentWidth,
                height: 16
            )
            cursorY = dependenciesHeaderLayer.frame.maxY + Metrics.sectionBodyGap
                - Metrics.fudge
                + Metrics.listSectionTopGap
            cursorY = layoutDependencies(startingAt: cursorY, minX: contentMinX, width: contentWidth)
            cursorY += Metrics.dependencyToInstallGap
        } else {
            dependenciesHeaderLayer.frame = .zero
        }

        if executablesHeaderLayer.string != nil {
            executablesHeaderLayer.frame = CGRect(
                x: contentMinX,
                y: cursorY,
                width: contentWidth,
                height: 16
            )
            cursorY = executablesHeaderLayer.frame.maxY + Metrics.sectionBodyGap
                - Metrics.fudge
                + Metrics.listSectionTopGap
            cursorY = layoutExecutables(startingAt: cursorY, minX: contentMinX, width: contentWidth)
        } else {
            executablesHeaderLayer.frame = .zero
        }

        if installDestinationHeaderLayer.string != nil, installDestinationURL != nil {
            cursorY += Metrics.executableToDestinationGap
            installDestinationHeaderLayer.frame = CGRect(
                x: contentMinX,
                y: cursorY,
                width: contentWidth,
                height: 16
            )
            cursorY = installDestinationHeaderLayer.frame.maxY + Metrics.sectionBodyGap

            let installDestinationHeight = heightForText(
                in: installDestinationLayer,
                width: contentWidth,
                minimumHeight: 16
            )
            installDestinationFrame = CGRect(
                x: contentMinX,
                y: cursorY - Metrics.fudge,
                width: contentWidth,
                height: installDestinationHeight
            )
            installDestinationLayer.frame = installDestinationFrame
            cursorY = installDestinationLayer.frame.maxY + Metrics.destinationToInstallGap
        } else {
            installDestinationHeaderLayer.frame = .zero
            installDestinationLayer.frame = .zero
            installDestinationFrame = .zero
            cursorY += Metrics.executableToDestinationGap
        }

        if versionSelectorHeaderLayer.string != nil, versionSelector.isHidden == false {
            versionSelectorHeaderLayer.frame = CGRect(
                x: contentMinX,
                y: cursorY,
                width: contentWidth,
                height: 16
            )
            cursorY = versionSelectorHeaderLayer.frame.maxY + Metrics.versionSelectorGap
            versionSelector.frame = CGRect(
                x: contentMinX,
                y: cursorY,
                width: contentWidth,
                height: Metrics.versionSelectorHeight
            )
            cursorY = versionSelector.frame.maxY + Metrics.versionHintTopGap
            let hintHeight = heightForText(
                in: versionSelectorHintLayer,
                width: contentWidth,
                minimumHeight: 14
            )
            versionSelectorHintLayer.frame = CGRect(
                x: contentMinX,
                y: cursorY,
                width: contentWidth,
                height: hintHeight
            )
            cursorY = versionSelectorHintLayer.frame.maxY + Metrics.versionHintBottomGap
        } else {
            versionSelectorHeaderLayer.frame = .zero
            versionSelectorHintLayer.frame = .zero
            versionSelector.frame = .zero
        }

        let showsActionControls = commandHeaderLayer.string != nil
            || installCommandLayer.string != nil
            || !updateButton.isHidden
            || !primaryActionButton.isHidden
            || !makeDefaultButton.isHidden
        if showsActionControls {
            commandHeaderLayer.frame = CGRect(
                x: contentMinX,
                y: cursorY,
                width: contentWidth,
                height: 16
            )
            cursorY = commandHeaderLayer.frame.maxY + Metrics.installCommandTopGap

            let commandHeight = heightForText(
                in: installCommandLayer,
                width: contentWidth,
                minimumHeight: 16
            )
            installCommandLayer.frame = CGRect(
                x: contentMinX,
                y: cursorY - Metrics.installCommandVisualLift,
                width: contentWidth,
                height: commandHeight
            )
            let buttonY = installCommandLayer.frame.maxY + Metrics.commandToButtonGap
            let availableButtonWidth = max(contentWidth, 0)
            if updateButton.isHidden {
                updateButton.frame = .zero
                primaryActionButton.frame = CGRect(
                    x: contentMinX,
                    y: buttonY,
                    width: min(
                        primaryActionButtonWidth(maximum: availableButtonWidth),
                        availableButtonWidth
                    ),
                    height: Metrics.actionButtonHeight
                )
                if makeDefaultButton.isHidden {
                    makeDefaultButton.frame = .zero
                } else {
                    makeDefaultButton.frame = CGRect(
                        x: primaryActionButton.frame.maxX + Metrics.actionButtonGap,
                        y: buttonY,
                        width: min(
                            Metrics.actionButtonWidth,
                            max(
                                availableButtonWidth
                                    - primaryActionButton.frame.width
                                    - Metrics.actionButtonGap,
                                0
                            )
                        ),
                        height: Metrics.actionButtonHeight
                    )
                }
            } else {
                let splitButtonWidth = min(
                    Metrics.actionButtonWidth,
                    max((availableButtonWidth - Metrics.actionButtonGap * 2) / 3, 0)
                )
                updateButton.frame = CGRect(
                    x: contentMinX,
                    y: buttonY,
                    width: splitButtonWidth,
                    height: Metrics.actionButtonHeight
                )
                primaryActionButton.frame = CGRect(
                    x: updateButton.frame.maxX + Metrics.actionButtonGap,
                    y: buttonY,
                    width: splitButtonWidth,
                    height: Metrics.actionButtonHeight
                )
                if makeDefaultButton.isHidden {
                    makeDefaultButton.frame = .zero
                } else {
                    makeDefaultButton.frame = CGRect(
                        x: primaryActionButton.frame.maxX + Metrics.actionButtonGap,
                        y: buttonY,
                        width: splitButtonWidth,
                        height: Metrics.actionButtonHeight
                    )
                }
            }
        } else {
            commandHeaderLayer.frame = .zero
            installCommandLayer.frame = .zero
            updateButton.frame = .zero
            primaryActionButton.frame = .zero
            makeDefaultButton.frame = .zero
        }
        UIStyle.layoutControlChrome(in: updateButton.layer)
        UIStyle.layoutControlChrome(in: primaryActionButton.layer)
        UIStyle.layoutControlChrome(in: makeDefaultButton.layer)

        let contentHeight = max(
            cursorY,
            updateButton.frame.maxY,
            primaryActionButton.frame.maxY,
            makeDefaultButton.frame.maxY
        ) + Metrics.bottomInset
        let targetHeight = max(superview?.bounds.height ?? 0, contentHeight)
        if abs(frame.height - targetHeight) > 0.5 {
            frame.size.height = targetHeight
        }

        CATransaction.commit()
    }

    func render(detail: PackageDetail?, animation: RenderAnimation = .none) {
        let previousVisibleSections = visibleSections()
        let previousLayoutFrames = layoutTransitionFrames()
        currentDetail = detail
        if let detail {
            applySectionVisibility(for: detail)
            currentLastUpdatedDate = detail.lastUpdatedAt.flatMap {
                ISO8601DateFormatter().date(from: $0)
            }
            titleLayer.string = UIStyle.attributedMonoText(
                detail.packageName,
                size: 22.8,
                color: UIStyle.text.withAlphaComponent(0.96),
                weight: .regular,
                tracking: -0.15
            )
            metadataLayer.string = UIStyle.attributedMonoText(
                metadataText(for: detail),
                size: 11,
                color: UIStyle.text.withAlphaComponent(0.50),
                tracking: 0.45
            )
            descriptionLayer.string = NSAttributedString(
                string: detail.primaryDescription,
                attributes: [
                    .font: UIStyle.monoFont(size: 12),
                    .foregroundColor: UIStyle.text.withAlphaComponent(0.84),
                    .kern: 0.1,
                    .paragraphStyle: UIStyle.wrapParagraphStyle(lineHeightMultiple: 1.28)
                ]
            )
            currentSecurityNotice = detail.securityNotice
            applySecurityNoticeStyle()
            popularityLayer.string = popularityText(for: detail)
            applyLastUpdatedStyle()
            configureVersionSelector(for: detail)
            let actionDetail = selectedActionDetail(from: detail)
            if detail.isSystemDetectorOnlyHazard {
                installCommandLayer.string = nil
                installDestinationURL = nil
                installDestinationLayer.string = nil
                updateButton.isHidden = true
                updateButton.isEnabled = false
                primaryActionButton.isHidden = true
                primaryActionButton.isEnabled = false
                makeDefaultButton.isHidden = true
                makeDefaultButton.isEnabled = false
            } else {
                installCommandLayer.string = UIStyle.attributedMonoText(
                    actionDetail.installCommand,
                    size: 12,
                    color: UIStyle.text.withAlphaComponent(0.66),
                    tracking: 0.1
                )
                installDestinationURL = actionDetail.installed
                    ? URL(fileURLWithPath: actionDetail.installRoot, isDirectory: true)
                    : nil
                applyInstallDestinationStyle()
                updateButton.isHidden = !(actionDetail.installed && actionDetail.isOutdated)
                    || actionDetail.isHomebrewCaskManaged
                    || actionDetail.isNpmSkillsManaged
                updateButton.isEnabled = !isActionInFlight
                primaryActionButton.title = actionDetail.installed ? "UNINSTALL" : "INSTALL"
                if detail.homebrewMigration != nil {
                    primaryActionButton.title = "MIGRATE"
                } else if detail.isHomebrewMigrationCandidate && !actionDetail.installed {
                    primaryActionButton.title = "MIGRATE TO AUTOMIC VAULT"
                } else if detail.isUnsupportedHomebrewInstall && !actionDetail.installed {
                    primaryActionButton.title = "UNSUPPORTED TAP"
                }
                if showsHomebrewMigrationButton(for: actionDetail) {
                    makeDefaultButton.title = "MIGRATE"
                    makeDefaultButton.isHidden = false
                } else {
                    makeDefaultButton.title = "MAKE DEFAULT"
                    makeDefaultButton.isHidden = !showsMakeDefaultButton(for: actionDetail)
                }
                makeDefaultButton.isEnabled = !isActionInFlight
                primaryActionButton.isHidden = false
                primaryActionButton.isEnabled = !isActionInFlight
            }
            renderDependencies(
                detail.dependencies,
                installedDependencies: installedPackDependencies(for: detail)
            )
            renderExecutables(detail.executablePaths, error: detail.executablePathsError)
        } else {
            titleLayer.string = nil
            metadataLayer.string = nil
            descriptionLayer.string = nil
            currentSecurityNotice = nil
            securityNoticePanelLayer.isHidden = true
            securityNoticeIconLayer.string = nil
            securityNoticeHeadlineLayer.string = nil
            securityNoticeBodyField.attributedText = nil
            securityNoticeReasonsHeaderLayer.string = nil
            securityNoticeReasonsBodyField.attributedText = nil
            securityNoticeCaveatsHeaderLayer.string = nil
            securityNoticeCaveatsBodyField.attributedText = nil
            popularityHeaderLayer.string = nil
            popularityLayer.string = nil
            lastUpdatedHeaderLayer.string = nil
            lastUpdatedLayer.string = nil
            dependenciesHeaderLayer.string = nil
            executablesHeaderLayer.string = nil
            installDestinationHeaderLayer.string = nil
            commandHeaderLayer.string = nil
            installCommandLayer.string = nil
            installDestinationURL = nil
            currentLastUpdatedDate = nil
            isHoveringLastUpdated = false
            isHoveringInstallDestination = false
            installDestinationHeaderLayer.frame = .zero
            installDestinationLayer.frame = .zero
            installDestinationFrame = .zero
            installDestinationLayer.string = nil
            versionSelectorHeaderLayer.string = nil
            versionSelectorHintLayer.string = nil
            versionSelector.removeAllItems()
            versionSelector.isHidden = true
            selectedVersionOptionPackageName = nil
            updateButton.isHidden = true
            updateButton.isEnabled = false
            primaryActionButton.isHidden = true
            primaryActionButton.isEnabled = false
            makeDefaultButton.isHidden = true
            makeDefaultButton.isEnabled = false
            securityLearnMoreButton.isHidden = true
            securityApplyButton.isHidden = true
            clearDependencies()
            clearExecutables()
        }

        needsLayout = true
        layoutSubtreeIfNeeded()
        switch animation {
        case .none:
            break
        case .full:
            animateReveal(layers: visibleLayers())
        case .incremental:
            let currentVisibleSections = visibleSections()
            let newlyVisibleSections = currentVisibleSections.subtracting(previousVisibleSections)
            animateReveal(layers: visibleLayers(for: newlyVisibleSections))
            animateLayoutTransition(from: previousLayoutFrames)
        }
    }

    func setActionInFlight(_ active: Bool) {
        isActionInFlight = active
        let isEnabled = active == false && currentDetail != nil
        updateButton.isEnabled = isEnabled
        primaryActionButton.isEnabled = isEnabled
        makeDefaultButton.isEnabled = isEnabled
        if let notice = currentSecurityNotice {
            securityApplyButton.isEnabled = isEnabled && securityApplyButtonIsEnabled(notice: notice)
        } else {
            securityApplyButton.isEnabled = false
        }
    }

    func setEyebrowLoading(_ active: Bool) {
        guard isEyebrowLoading != active else { return }
        isEyebrowLoading = active
        updatePanelHeader()
    }

    private func applySectionVisibility(for detail: PackageDetail) {
        popularityHeaderLayer.string = showsPopularity(for: detail)
            ? UIStyle.sectionHeaderText("Popularity")
            : nil
        lastUpdatedHeaderLayer.string = showsLastUpdated(for: detail)
            ? UIStyle.sectionHeaderText("Last Update")
            : nil
        dependenciesHeaderLayer.string = showsDependencies(for: detail)
            ? UIStyle.sectionHeaderText("Dependencies")
            : nil
        executablesHeaderLayer.string = showsExecutables(for: detail)
            ? UIStyle.sectionHeaderText("Executables")
            : nil
        installDestinationHeaderLayer.string = showsInstallDestination(for: detail)
            ? UIStyle.sectionHeaderText("Install Destination")
            : nil
        versionSelectorHeaderLayer.string = detail.versionOptions.isEmpty
            ? nil
            : UIStyle.sectionHeaderText("Version")
        versionSelectorHintLayer.string = detail.versionOptions.isEmpty
            ? nil
            : versionSelectorHintText(for: detail)
        commandHeaderLayer.string = detail.isSystemDetectorOnlyHazard
            ? nil
            : UIStyle.sectionHeaderText("Install Vector")
    }

    private func applySecurityNoticeStyle() {
        guard let notice = currentSecurityNotice else {
            securityNoticePanelLayer.isHidden = true
            securityNoticeIconLayer.string = nil
            securityNoticeHeadlineLayer.string = nil
            securityNoticeBodyField.attributedText = nil
            securityNoticeReasonsHeaderLayer.string = nil
            securityNoticeReasonsBodyField.attributedText = nil
            securityNoticeCaveatsHeaderLayer.string = nil
            securityNoticeCaveatsBodyField.attributedText = nil
            securityLearnMoreButton.isHidden = true
            securityApplyButton.isHidden = true
            return
        }

        securityNoticePanelLayer.isHidden = false
        securityNoticeIconLayer.string = UIStyle.attributedMonoText(
            "☢",
            size: 28,
            color: UIStyle.danger.withAlphaComponent(0.92),
            tracking: 0
        )
        securityNoticeHeadlineLayer.string = UIStyle.attributedMonoText(
            notice.headline,
            size: 16,
            color: UIStyle.danger.withAlphaComponent(0.98),
            weight: .medium,
            tracking: 0.2
        )
        securityNoticeBodyField.attributedText = securityNoticeBodyText(from: notice.body)
        securityNoticeReasonsHeaderLayer.string = notice.reasons.isEmpty
            ? nil
            : UIStyle.sectionHeaderText("Detection")
        securityNoticeReasonsBodyField.attributedText = securityNoticeReasonsText(notice.reasons)
        securityNoticeCaveatsHeaderLayer.string = notice.caveats == nil
            ? nil
            : UIStyle.sectionHeaderText("Caveats")
        securityNoticeCaveatsBodyField.attributedText = securityNoticeCaveatsText(notice.caveats)
        securityLearnMoreButton.isHidden = false
        securityLearnMoreButton.isEnabled = true
        securityApplyButton.title = securityApplyButtonTitle()
        securityApplyButton.isHidden = notice.applyPackageName == nil
        securityApplyButton.isEnabled = !isActionInFlight
            && securityApplyButtonIsEnabled(notice: notice)
    }

    private func updatePanelHeader() {
        panelHeaderAnimator?.setText("DOSSIER", animated: isEyebrowLoading)
    }

    private func securityNoticeBodyText(from markdown: String) -> NSAttributedString {
        let normalizedMarkdown = normalizedSecurityNoticeMarkdown(markdown)
        let parsed: NSAttributedString
        do {
            let attributed = try AttributedString(
                markdown: normalizedMarkdown,
                options: AttributedString.MarkdownParsingOptions(
                    interpretedSyntax: .inlineOnlyPreservingWhitespace,
                    failurePolicy: .returnPartiallyParsedIfPossible
                )
            )
            parsed = NSAttributedString(attributed)
        } catch {
            parsed = NSAttributedString(string: normalizedMarkdown)
        }

        let styled = NSMutableAttributedString(attributedString: parsed)
        let fullRange = NSRange(location: 0, length: styled.length)
        let baseFont = UIStyle.monoFont(size: 11)
        let mediumFont = UIStyle.monoFont(size: 11, weight: .medium)
        let italicFont = NSFontManager.shared.convert(
            baseFont,
            toHaveTrait: .italicFontMask
        )

        styled.addAttributes(
            [
                .font: baseFont,
                .foregroundColor: UIStyle.text.withAlphaComponent(0.82),
                .kern: 0.12,
                .paragraphStyle: UIStyle.wrapParagraphStyle(
                    lineHeightMultiple: 1.24,
                    paragraphSpacing: 2,
                    lineBreakMode: .byWordWrapping
                )
            ],
            range: fullRange
        )

        styled.enumerateAttribute(
            .inlinePresentationIntent,
            in: fullRange,
            options: []
        ) { value, range, _ in
            guard let intent = value as? Int else { return }
            if intent & 4 != 0 {
                styled.addAttributes(
                    [
                        .font: mediumFont,
                        .foregroundColor: UIStyle.danger.withAlphaComponent(0.98)
                    ],
                    range: range
                )
            } else if intent & 2 != 0 {
                styled.addAttribute(.font, value: mediumFont, range: range)
            } else if intent & 1 != 0 {
                styled.addAttribute(.font, value: italicFont, range: range)
            }
        }

        styled.removeAttribute(.inlinePresentationIntent, range: fullRange)
        styled.removeAttribute(NSAttributedString.Key("NSPresentationIntent"), range: fullRange)
        styled.removeAttribute(NSAttributedString.Key("NSListItemDelimiter"), range: fullRange)
        return styled
    }

    private func securityNoticeCaveatsText(
        _ caveats: PackageSecurityNotice.Caveats?
    ) -> NSAttributedString? {
        guard let caveats else {
            return nil
        }

        switch caveats {
        case .paragraph(let paragraph):
            return securityNoticeBodyText(from: paragraph)
        case .bullets(let bullets):
            let markdown = bullets
                .map { "- \(normalizedCaveatBullet($0))" }
                .joined(separator: "\n")
            return securityNoticeBodyText(from: markdown)
        }
    }

    private func securityNoticeReasonsText(_ reasons: [String]) -> NSAttributedString? {
        guard reasons.isEmpty == false else {
            return nil
        }
        let markdown = reasons
            .map { "- \(normalizedCaveatBullet($0))" }
            .joined(separator: "\n")
        return securityNoticeBodyText(from: markdown)
    }

    private func normalizedCaveatBullet(_ caveat: String) -> String {
        caveat
            .components(separatedBy: .whitespacesAndNewlines)
            .filter { $0.isEmpty == false }
            .joined(separator: " ")
    }

    private func normalizedSecurityNoticeMarkdown(_ markdown: String) -> String {
        let lines = markdown
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .components(separatedBy: .newlines)
        var blocks: [[String]] = []
        var currentBlock: [String] = []

        for line in lines {
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            if trimmed.isEmpty {
                if currentBlock.isEmpty == false {
                    blocks.append(currentBlock)
                    currentBlock = []
                }
            } else {
                currentBlock.append(trimmed)
            }
        }

        if currentBlock.isEmpty == false {
            blocks.append(currentBlock)
        }

        return blocks
            .map { block in
                if block.contains(where: isMarkdownListItem) {
                    return block.joined(separator: "\n")
                }
                return block.joined(separator: " ")
            }
            .joined(separator: "\n\n")
    }

    private func isMarkdownListItem(_ line: String) -> Bool {
        if line.range(
            of: #"^(\d+[\.)]|[-*+])\s+"#,
            options: .regularExpression
        ) != nil {
            return true
        }
        return false
    }

    private func clearDependencies() {
        dependencyClusters.forEach { $0.layer.removeFromSuperlayer() }
        dependencyClusters.removeAll()
        dependencyFrames.removeAll()
        hoveredDependencyName = nil
    }

    private func clearExecutables() {
        executableClusters.forEach { $0.layer.removeFromSuperlayer() }
        executableClusters.removeAll()
        executableFrames.removeAll()
        executableURLs.removeAll()
        hoveredExecutableName = nil
    }

    private func renderDependencies(
        _ dependencies: [String],
        installedDependencies: Set<String>
    ) {
        clearDependencies()

        guard dependencies.isEmpty == false else { return }

        for (index, dependency) in dependencies.enumerated() {
            let layer = CATextLayer()
            layer.contentsScale = NSScreen.main?.backingScaleFactor ?? 2
            layer.alignmentMode = .left
            layer.isWrapped = false
            layer.truncationMode = .end
            layer.actions = [
                "foregroundColor": NSNull(),
                "opacity": NSNull()
            ]
            self.layer?.addSublayer(layer)

            let alpha = min(0.80, 0.70 + CGFloat(index % 3) * 0.04)
            let cluster = DependencyCluster(
                name: dependency,
                baseAlpha: alpha,
                isInstalled: installedDependencies.contains(dependency),
                layer: layer
            )
            dependencyClusters.append(cluster)
            applyStyle(to: cluster, hovered: false)
        }
    }

    private func installedPackDependencies(for detail: PackageDetail) -> Set<String> {
        guard isInstallPack(detail),
              let installPackageNames = detail.installPackageNames else {
            return []
        }

        let missingPackageNames = Set(
            installPackageNames.flatMap(Self.packageDependencyIdentifiers)
        )
        return Set(detail.dependencies.filter { missingPackageNames.contains($0) == false })
    }

    private func isInstallPack(_ detail: PackageDetail) -> Bool {
        switch detail.packageName {
        case PackageRecommendation.agenticToolingPackName,
             PackageRecommendation.agentPackName,
             PackageRecommendation.unixPlusPlusPackName:
            return true
        default:
            return false
        }
    }

    private static func packageDependencyIdentifiers(_ packageName: String) -> [String] {
        var identifiers = [packageName]
        for prefix in ["brew:", "cask:", "npm:", "pip:"] {
            if let unqualified = packageName.strippingPrefix(prefix) {
                identifiers.append(unqualified)
            }
        }
        return identifiers
    }

    private func renderExecutables(_ executables: [String], error: String?) {
        clearExecutables()

        let displayValues: [String]
        if let error {
            displayValues = ["unavailable (\(error))"]
        } else {
            displayValues = executables
        }

        guard displayValues.isEmpty == false else { return }

        for (index, executable) in displayValues.enumerated() {
            let layer = CATextLayer()
            layer.contentsScale = NSScreen.main?.backingScaleFactor ?? 2
            layer.alignmentMode = .left
            layer.isWrapped = false
            layer.truncationMode = .middle
            layer.actions = [
                "foregroundColor": NSNull(),
                "opacity": NSNull()
            ]
            self.layer?.addSublayer(layer)

            let alpha = error != nil
                ? CGFloat(0.56)
                : min(0.80, 0.70 + CGFloat(index % 3) * 0.04)
            let cluster = DependencyCluster(
                name: executable,
                baseAlpha: alpha,
                isInstalled: false,
                layer: layer
            )
            executableClusters.append(cluster)
            if error == nil, executables.isEmpty == false, executable.hasPrefix("/") {
                executableURLs[executable] = URL(fileURLWithPath: executable)
            }
            applyStyle(to: cluster, hovered: false)
        }
    }

    private func layoutDependencies(
        startingAt startY: CGFloat,
        minX: CGFloat,
        width: CGFloat
    ) -> CGFloat {
        dependencyFrames.removeAll()
        guard !dependencyClusters.isEmpty else {
            return startY + Metrics.sectionGap
        }

        let columnCount = dependencyColumnCount(for: width)
        let totalGapWidth = CGFloat(columnCount - 1) * Metrics.dependencyColumnGap
        let columnWidth = floor((width - totalGapWidth) / CGFloat(columnCount))
        var contentBottom = startY

        for (index, cluster) in dependencyClusters.enumerated() {
            let row = index / columnCount
            let column = index % columnCount
            let x = minX + CGFloat(column) * (columnWidth + Metrics.dependencyColumnGap)
            let y = startY + CGFloat(row) * Metrics.dependencyRowGap
            let frame = CGRect(
                x: x,
                y: y - 1,
                width: columnWidth,
                height: 15
            )
            cluster.layer.frame = frame
            dependencyFrames[cluster.name] = frame
            contentBottom = max(contentBottom, frame.maxY)
        }

        return contentBottom
    }

    private func layoutExecutables(
        startingAt startY: CGFloat,
        minX: CGFloat,
        width: CGFloat
    ) -> CGFloat {
        executableFrames.removeAll()
        guard !executableClusters.isEmpty else {
            return startY + Metrics.sectionGap
        }

        var contentBottom = startY
        for (index, cluster) in executableClusters.enumerated() {
            let y = startY + CGFloat(index) * Metrics.executableRowGap
            let frame = CGRect(
                x: minX,
                y: y - 1,
                width: width,
                height: 15
            )
            cluster.layer.frame = frame
            executableFrames[cluster.name] = frame
            contentBottom = max(contentBottom, frame.maxY)
        }

        return contentBottom
    }

    private func updateHoveredDependency(name: String?) {
        guard hoveredDependencyName != name else { return }
        hoveredDependencyName = name
        for cluster in dependencyClusters {
            applyStyle(to: cluster, hovered: cluster.name == name)
        }
    }

    private func updateHoveredExecutable(name: String?) {
        guard hoveredExecutableName != name else { return }
        hoveredExecutableName = name
        for cluster in executableClusters {
            applyStyle(to: cluster, hovered: cluster.name == name)
        }
        if name == nil, isHoveringInstallDestination == false {
            NSCursor.arrow.set()
        } else if let name, executableURLs[name] != nil {
            NSCursor.pointingHand.set()
        }
    }

    private func updateHoveredInstallDestination(isActive: Bool) {
        guard isHoveringInstallDestination != isActive else { return }
        isHoveringInstallDestination = isActive
        applyInstallDestinationStyle()
        if isActive {
            NSCursor.pointingHand.set()
        } else {
            NSCursor.arrow.set()
        }
    }

    private func updateHoveredLastUpdated(isActive: Bool) {
        guard isHoveringLastUpdated != isActive else { return }
        isHoveringLastUpdated = isActive
        applyLastUpdatedStyle()
    }

    private func applyStyle(to cluster: DependencyCluster, hovered: Bool) {
        let alpha = hovered ? 1.0 : cluster.baseAlpha
        if executableURLs[cluster.name] != nil {
            let attributes: [NSAttributedString.Key: Any] = [
                .font: UIStyle.monoFont(size: 11, weight: hovered ? .medium : .regular),
                .foregroundColor: hovered
                    ? UIStyle.accent.withAlphaComponent(0.98)
                    : UIStyle.accent.withAlphaComponent(cluster.baseAlpha),
                .kern: 0.2,
                .underlineStyle: NSUnderlineStyle.single.rawValue
            ]
            cluster.layer.string = NSAttributedString(
                string: cluster.name,
                attributes: attributes
            )
        } else {
            let baseColor = cluster.isInstalled ? UIStyle.accent : UIStyle.text
            cluster.layer.string = UIStyle.attributedMonoText(
                cluster.name,
                size: 11,
                color: baseColor.withAlphaComponent(alpha),
                tracking: 0.2
            )
        }
        cluster.layer.opacity = Float(alpha)
    }

    private func executableURL(at point: CGPoint) -> URL? {
        guard let hoveredExecutableName = executableFrames.first(where: { $0.value.contains(point) })?.key
        else {
            return nil
        }
        return executableURLs[hoveredExecutableName]
    }

    private func applyLastUpdatedStyle() {
        guard let currentLastUpdatedDate else {
            lastUpdatedLayer.string = nil
            return
        }

        let text = isHoveringLastUpdated
            ? DateFormatter.dossierLastUpdatedExact.string(from: currentLastUpdatedDate)
            : RelativeTimeFormatter.dossierLastUpdated(from: currentLastUpdatedDate)

        lastUpdatedLayer.string = UIStyle.attributedMonoText(
            text,
            size: 11,
            color: UIStyle.text.withAlphaComponent(0.70),
            tracking: 0.2
        )
    }

    private func applyInstallDestinationStyle() {
        guard let installDestinationURL else {
            installDestinationLayer.string = nil
            installDestinationLayer.opacity = 0
            return
        }

        let baseAlpha: CGFloat = 0.70
        let alpha = isHoveringInstallDestination ? CGFloat(0.98) : baseAlpha
        let attributes: [NSAttributedString.Key: Any] = [
            .font: UIStyle.monoFont(size: 11, weight: .medium),
            .foregroundColor: UIStyle.accent.withAlphaComponent(alpha),
            .kern: 0.15,
            .underlineStyle: NSUnderlineStyle.single.rawValue
        ]
        installDestinationLayer.string = NSAttributedString(
            string: installDestinationURL.path,
            attributes: attributes
        )
        installDestinationLayer.opacity = Float(alpha)
    }

    private func animateReveal(layers: [CALayer]) {
        for (index, layer) in layers.enumerated() {
            animateReveal(
                layer: layer,
                delay: CFTimeInterval(index) * Timing.rowStep
            )
        }
    }

    private func animateReveal(layer: CALayer, delay: CFTimeInterval) {
        let opacityAnimation = CABasicAnimation(keyPath: "opacity")
        opacityAnimation.fromValue = 0.0
        opacityAnimation.toValue = layer.opacity
        opacityAnimation.beginTime = CACurrentMediaTime() + delay
        opacityAnimation.duration = Timing.reveal
        opacityAnimation.fillMode = .backwards
        opacityAnimation.timingFunction = CAMediaTimingFunction(name: .easeOut)

        let transformAnimation = CABasicAnimation(keyPath: "transform.translation.y")
        transformAnimation.fromValue = 5
        transformAnimation.toValue = 0
        transformAnimation.beginTime = opacityAnimation.beginTime
        transformAnimation.duration = Timing.reveal
        transformAnimation.fillMode = .backwards
        transformAnimation.timingFunction = CAMediaTimingFunction(name: .easeOut)

        layer.add(opacityAnimation, forKey: "dossierRevealOpacity")
        layer.add(transformAnimation, forKey: "dossierRevealTransform")
    }

    private func layoutTransitionFrames() -> LayoutTransitionFrames {
        LayoutTransitionFrames(
            commandHeader: commandHeaderLayer.frame,
            installCommand: installCommandLayer.frame,
            updateButton: updateButton.frame,
            primaryActionButton: primaryActionButton.frame
        )
    }

    private func animateLayoutTransition(from oldFrames: LayoutTransitionFrames) {
        animateVerticalShift(
            layer: commandHeaderLayer,
            from: oldFrames.commandHeader,
            to: commandHeaderLayer.frame
        )
        animateVerticalShift(
            layer: installCommandLayer,
            from: oldFrames.installCommand,
            to: installCommandLayer.frame
        )
        if let buttonLayer = updateButton.layer {
            animateVerticalShift(
                layer: buttonLayer,
                from: oldFrames.updateButton,
                to: updateButton.frame
            )
        }
        if let buttonLayer = primaryActionButton.layer {
            animateVerticalShift(
                layer: buttonLayer,
                from: oldFrames.primaryActionButton,
                to: primaryActionButton.frame
            )
        }
    }

    private func animateVerticalShift(layer: CALayer, from oldFrame: CGRect, to newFrame: CGRect) {
        guard oldFrame.isEmpty == false, newFrame.isEmpty == false else { return }

        let deltaY = oldFrame.midY - newFrame.midY
        guard abs(deltaY) > 0.5 else { return }

        let animation = CABasicAnimation(keyPath: "transform.translation.y")
        animation.fromValue = deltaY
        animation.toValue = 0
        animation.duration = Timing.reveal
        animation.timingFunction = CAMediaTimingFunction(name: .easeOut)
        layer.add(animation, forKey: "dossierLayoutShift")
    }

    private func metadataText(for detail: PackageDetail) -> String {
        let version = detail.installedVersion ?? detail.latestVersion ?? "unversioned"
        let source = detail.source?.displayLabel.lowercased() ?? "vendor"
        let status = detail.installed ? "installed" : "available"
        return "\(version) · \(source) · \(status)"
    }

    private func configureVersionSelector(for detail: PackageDetail) {
        guard detail.versionOptions.isEmpty == false else {
            versionSelector.removeAllItems()
            versionSelector.isHidden = true
            versionSelectorHintLayer.string = nil
            selectedVersionOptionPackageName = nil
            return
        }
        versionSelectorHintLayer.string = versionSelectorHintText(for: detail)
        versionSelector.removeAllItems()
        for option in detail.versionOptions {
            versionSelector.addItem(withTitle: option.menuTitle)
            versionSelector.lastItem?.representedObject = option.packageName
        }
        let preferred = selectedVersionOptionPackageName.flatMap { packageName in
            detail.versionOptions.first(where: { $0.packageName == packageName })
        }
            ?? detail.versionOptions.first(where: \.stubActive)
            ?? detail.versionOptions.first(where: \.installed)
            ?? detail.versionOptions.first(where: \.isRecommended)
            ?? detail.versionOptions.first(where: \.isLatest)
            ?? detail.versionOptions.first
        selectedVersionOptionPackageName = preferred?.packageName
        if let packageName = preferred?.packageName,
           let index = detail.versionOptions.firstIndex(where: { $0.packageName == packageName }) {
            versionSelector.selectItem(at: index)
        }
        versionSelector.isHidden = false
    }

    private func versionSelectorHintText(for detail: PackageDetail) -> NSAttributedString {
        let supportsSideBySideStubs = detail.versionOptions.contains(where: \.supportsSideBySideStubs)
        let text = supportsSideBySideStubs
            ? "multiple versions can be installed side by side"
            : "multiple versions can be installed, but only one stubs into /usr/local/bin"
        return UIStyle.attributedMonoText(
            text,
            size: 10.5,
            color: UIStyle.text.withAlphaComponent(0.54),
            tracking: 0.1
        )
    }

    private func selectedVersionOption(from detail: PackageDetail) -> PackageVersionOption? {
        if let packageName = selectedVersionOptionPackageName,
           let option = detail.versionOptions.first(where: { $0.packageName == packageName }) {
            return option
        }
        return detail.versionOptions.first
    }

    private func selectedActionDetail(from detail: PackageDetail) -> PackageDetail {
        guard let option = selectedVersionOption(from: detail) else {
            return detail
        }
        return detail.selecting(versionOption: option)
    }

    private func showsMakeDefaultButton(for detail: PackageDetail) -> Bool {
        guard let option = selectedVersionOption(from: detail) else {
            return false
        }
        return option.installed
            && option.stubActive == false
            && option.supportsSideBySideStubs == false
    }

    private func showsHomebrewMigrationButton(for detail: PackageDetail) -> Bool {
        detail.installed && detail.isHomebrewMigrationCandidate
    }

    private func primaryActionButtonWidth(maximum: CGFloat) -> CGFloat {
        let attributes: [NSAttributedString.Key: Any] = [
            .font: primaryActionButton.font ?? UIStyle.monoFont(size: 11, weight: .medium),
            .kern: 0.0
        ]
        let titleWidth = ceil(
            (primaryActionButton.title as NSString).size(withAttributes: attributes).width
        )
        return min(max(Metrics.actionButtonWidth, titleWidth + 28), maximum)
    }

    private func popularityText(for detail: PackageDetail) -> NSAttributedString? {
        guard let popularity = detail.popularity else {
            return nil
        }

        let text =
            "\(popularity.installsPer365DaysText) i/yr" +
            " · rank \(popularity.rankText)"

        return UIStyle.attributedMonoText(
            text,
            size: 11,
            color: UIStyle.text.withAlphaComponent(0.70),
            tracking: 0.2
        )
    }

    private func showsPopularity(for detail: PackageDetail) -> Bool {
        detail.popularity != nil
    }

    private func showsLastUpdated(for detail: PackageDetail) -> Bool {
        detail.lastUpdatedAt != nil
    }

    private func showsDependencies(for detail: PackageDetail) -> Bool {
        detail.dependencies.isEmpty == false
    }

    private func showsExecutables(for detail: PackageDetail) -> Bool {
        detail.executablePaths.isEmpty == false || detail.executablePathsError != nil
    }

    private func showsInstallDestination(for detail: PackageDetail) -> Bool {
        detail.installed
    }

    private func visibleSections() -> Set<String> {
        var sections: Set<String> = []
        if titleLayer.string != nil { sections.insert("title") }
        if metadataLayer.string != nil { sections.insert("metadata") }
        if descriptionLayer.string != nil { sections.insert("description") }
        if currentSecurityNotice != nil { sections.insert("securityNotice") }
        if popularityHeaderLayer.string != nil { sections.insert("popularity") }
        if lastUpdatedHeaderLayer.string != nil { sections.insert("lastUpdated") }
        if dependenciesHeaderLayer.string != nil { sections.insert("dependencies") }
        if executablesHeaderLayer.string != nil { sections.insert("executables") }
        if installDestinationHeaderLayer.string != nil { sections.insert("installDestination") }
        if versionSelectorHeaderLayer.string != nil { sections.insert("version") }
        if commandHeaderLayer.string != nil { sections.insert("command") }
        return sections
    }

    private func visibleLayers() -> [CALayer] {
        visibleLayers(for: visibleSections())
    }

    private func noticeTextFieldLayers() -> [CALayer] {
        [
            securityNoticeBodyField,
            securityNoticeReasonsBodyField,
            securityNoticeCaveatsBodyField
        ]
            .filter { !$0.isHidden }
            .compactMap(\.layer)
    }

    private func visibleLayers(for sections: Set<String>) -> [CALayer] {
        var layers: [CALayer] = []

        if sections.contains("title")
            || sections.contains("metadata")
            || sections.contains("description")
        {
            layers.append(contentsOf: [titleLayer, metadataLayer, descriptionLayer])
        }
        if sections.contains("popularity") {
            layers.append(contentsOf: [popularityHeaderLayer, popularityLayer])
        }
        if sections.contains("securityNotice") {
            layers.append(contentsOf: [
                securityNoticePanelLayer,
                securityNoticeIconLayer,
                securityNoticeHeadlineLayer,
                securityNoticeReasonsHeaderLayer,
                securityNoticeCaveatsHeaderLayer
            ])
            layers.append(contentsOf: noticeTextFieldLayers())
            if let buttonLayer = securityLearnMoreButton.layer {
                layers.append(buttonLayer)
            }
            if let buttonLayer = securityApplyButton.layer {
                layers.append(buttonLayer)
            }
        }
        if sections.contains("lastUpdated") {
            layers.append(contentsOf: [lastUpdatedHeaderLayer, lastUpdatedLayer])
        }
        if sections.contains("dependencies") {
            layers.append(dependenciesHeaderLayer)
            layers.append(contentsOf: dependencyClusters.map(\.layer))
        }
        if sections.contains("executables") {
            layers.append(executablesHeaderLayer)
            layers.append(contentsOf: executableClusters.map(\.layer))
        }
        if sections.contains("installDestination") {
            layers.append(contentsOf: [installDestinationHeaderLayer, installDestinationLayer])
        }
        if sections.contains("version") {
            layers.append(contentsOf: [versionSelectorHeaderLayer, versionSelectorHintLayer])
        }
        if sections.contains("command") {
            layers.append(contentsOf: [commandHeaderLayer, installCommandLayer])
        }

        return layers.filter { $0.frame.isEmpty == false }
    }

    private func dependencyColumnCount(for width: CGFloat) -> Int {
        let threeColumnWidth =
            Metrics.dependencyColumnMinWidth * 3 + Metrics.dependencyColumnGap * 2
        if width >= threeColumnWidth {
            return 3
        }

        let twoColumnWidth =
            Metrics.dependencyColumnMinWidth * 2 + Metrics.dependencyColumnGap
        if width >= twoColumnWidth {
            return 2
        }

        return 1
    }

    private func heightForText(
        in layer: CATextLayer,
        width: CGFloat,
        minimumHeight: CGFloat
    ) -> CGFloat {
        guard let attributedText = layer.string as? NSAttributedString else {
            return minimumHeight
        }
        let measured = attributedText.boundingRect(
            with: CGSize(width: width, height: .greatestFiniteMagnitude),
            options: [.usesLineFragmentOrigin, .usesFontLeading]
        )
        return max(minimumHeight, ceil(measured.height) + 2)
    }

    private func heightForNoticeBody(width: CGFloat) -> CGFloat {
        heightForNoticeText(in: securityNoticeBodyField, width: width, minimumHeight: 34)
    }

    private func heightForNoticeText(
        in field: DossierNoticeTextField,
        width: CGFloat,
        minimumHeight: CGFloat
    ) -> CGFloat {
        guard let attributedText = field.attributedText else {
            return minimumHeight
        }
        let measured = attributedText.boundingRect(
            with: CGSize(width: width, height: .greatestFiniteMagnitude),
            options: [.usesLineFragmentOrigin, .usesFontLeading]
        )
        return max(minimumHeight, ceil(measured.height) + 4)
    }

    private func securityNoticeButtonWidth(for button: NSButton) -> CGFloat {
        let attributes: [NSAttributedString.Key: Any] = [
            .font: button.font ?? UIStyle.monoFont(size: 11, weight: .medium),
            .kern: 0.0
        ]
        let titleWidth = ceil((button.title as NSString).size(withAttributes: attributes).width)
        return titleWidth + Metrics.securityNoticeButtonHorizontalPadding * 2
    }

    private func securityApplyButtonIsEnabled(notice: PackageSecurityNotice) -> Bool {
        guard let currentDetail else {
            return false
        }
        guard let applyPackageName = notice.applyPackageName else {
            return false
        }
        if let source = currentDetail.source, case .isotope = source {
            return true
        }
        return currentDetail.helperPackageName != applyPackageName
    }

    private func securityApplyButtonTitle() -> String {
        if let source = currentDetail?.source, case .isotope = source {
            return "Secure Secrets"
        }
        return "Convert to Isotope"
    }

    @objc private func handlePrimaryAction() {
        guard let currentDetail, isActionInFlight == false else { return }
        delegate?.dossierView(self, didRequestPrimaryActionFor: selectedActionDetail(from: currentDetail))
    }

    @objc private func handleUpdateAction() {
        guard let currentDetail,
              isActionInFlight == false,
              currentDetail.isOutdated else {
            return
        }
        delegate?.dossierView(self, didRequestUpdateActionFor: selectedActionDetail(from: currentDetail))
    }

    @objc private func handleMakeDefaultAction() {
        guard let currentDetail, isActionInFlight == false else { return }
        let actionDetail = selectedActionDetail(from: currentDetail)
        if showsHomebrewMigrationButton(for: actionDetail) {
            delegate?.dossierView(self, didRequestMigrationActionFor: actionDetail)
            return
        }
        delegate?.dossierView(self, didRequestDefaultActionFor: actionDetail)
    }

    @objc private func handleVersionSelection() {
        guard let item = versionSelector.selectedItem,
              let packageName = item.representedObject as? String else {
            return
        }
        selectedVersionOptionPackageName = packageName
        if let currentDetail {
            render(detail: currentDetail, animation: .none)
        }
    }

    @objc private func handleSecurityLearnMore() {
        guard let currentSecurityNotice else { return }
        NSWorkspace.shared.open(currentSecurityNotice.learnMoreURL)
    }

    @objc private func handleSecurityApply() {
        guard let currentDetail, isActionInFlight == false else { return }
        delegate?.dossierView(self, didRequestSecurityActionFor: currentDetail)
    }
}

private enum RelativeTimeFormatter {
    static func dossierLastUpdated(from date: Date) -> String {
        let calendar = Calendar.autoupdatingCurrent
        let now = Date()
        let startOfToday = calendar.startOfDay(for: now)

        if date >= startOfToday {
            return "today"
        }

        let hoursAgo = calendar.dateComponents([.hour], from: date, to: now).hour ?? 0
        if hoursAgo < 24 {
            return hoursAgo == 1 ? "1 hour ago" : "\(hoursAgo) hours ago"
        }

        let startOfDate = calendar.startOfDay(for: date)
        let daysAgo = max(
            1,
            calendar.dateComponents([.day], from: startOfDate, to: startOfToday).day ?? 1
        )
        return daysAgo == 1 ? "1 day ago" : "\(daysAgo) days ago"
    }
}

private extension DateFormatter {
    static let dossierLastUpdatedExact: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter
    }()
}
