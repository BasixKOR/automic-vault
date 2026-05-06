import AppKit
import QuartzCore

protocol PackageFieldViewDelegate: AnyObject {
    func packageFieldView(_ view: PackageFieldView, didSelect package: PackagePresentation)
}

final class PackageFieldView: NSView {
    enum NavigationDirection {
        case left
        case right
        case up
        case down
    }

    private struct Metrics {
        static let panelHeaderTopInset: CGFloat = 6
        static let panelHeaderSpineX: CGFloat = 7
        static let panelHeaderTextInset: CGFloat = 10
        static let panelHeaderLabelHeight: CGFloat = 10
        static let leadingInset: CGFloat = 24
        static let trailingInset: CGFloat = 8
        static let bottomInset: CGFloat = 16
        static let topInset: CGFloat = 28
        static let helpTopInset: CGFloat = 32
        static let helpHeight: CGFloat = 54
        static let helpBottomGap: CGFloat = 30
        static let commandSectionLabelHeight: CGFloat = 14
        static let quoteTopGap: CGFloat = 14
        static let quoteHeight: CGFloat = 18
        static let preferredNodeWidth: CGFloat = 142
        static let minNodeWidth: CGFloat = 128
        static let maxNodeWidth: CGFloat = 156
        static let horizontalGap: CGFloat = 10
        static let nodeHeight: CGFloat = 56
        static let rowGap: CGFloat = 20
        static let sectionGap: CGFloat = 36
        static let dividerOffset: CGFloat = 26
        static let sectionLabelOffset: CGFloat = 8
        static let nameVersionGapLines = 1
        static let inlineDescriptionGap = "  "
        static let inlineMetadataGap = "   "
        static let hazardSymbolGapFontSize: CGFloat = 5
        static let hazardSymbolFontSize: CGFloat = 15
        static let isotopeSymbolSize: CGFloat = 13
        static let homebrewSymbolFontSize: CGFloat = 13
        static let versionBaselineOffset: CGFloat = -2
        static let descriptionBaselineOffset: CGFloat = 0
        static let bracketInsetX: CGFloat = 4
        static let bracketInsetY: CGFloat = 4
        static let bracketArmLength: CGFloat = 7
        static let hoverBracketOpacity: Float = 0.26
        static let selectedBracketOpacity: Float = 0.30
        static let gridInfluenceOpacity: Float = 0.20
        static let selectionGridOpacity: Float = 0.11
        static let hoverGridOpacity: Float = 0.17
        static let gridGlowRadius: CGFloat = 3
        static let cursorInfluenceHalfWidth: CGFloat = 75
        static let cursorInfluenceHalfHeight: CGFloat = 75
        static let cursorLerpFactor: CGFloat = 0.18
        static let cursorFadeInStep: CGFloat = 0.18
        static let cursorFadeOutStep: CGFloat = 0.12
        static let hoverCaptureDistance: CGFloat = 20
        static let rowBaselineRatio: CGFloat = 0.62
    }

    private struct Timing {
        static let quick: CFTimeInterval = 0.14
        static let standard: CFTimeInterval = 0.18
        static let delayed: CFTimeInterval = 0.20
        static let hoverFadeOut: CFTimeInterval = 0.18
        static let selectionFadeOut: CFTimeInterval = 0.28
    }

    private struct Motion {
        static let insertionYOffset: CGFloat = -10
    }

    private final class PackageNodeLayer: CALayer {
        let textLayer = CATextLayer()
        let isotopeSymbolLayer = CALayer()
        let hazardEffect = PackageNodeHazardEffect()
        var package: PackagePresentation
        var basePosition = CGPoint.zero
        var baseOpacity: Float = 1.0
        var isHovered = false
        var isSelectedNode = false
        var matchesSearch = true
        var isEntering = false
        var titleColor = UIStyle.text.withAlphaComponent(0.88)
        var versionColor = UIStyle.text.withAlphaComponent(0.58)
        var descriptionColor = UIStyle.text.withAlphaComponent(0.50)
        private var renderedWidth: CGFloat = 0
        private var renderedPackage: PackagePresentation?
        private var renderedTitleColor: NSColor?
        private var renderedVersionColor: NSColor?
        private var renderedDescriptionColor: NSColor?
        private var renderedTextBounds = CGRect.zero
        private var renderedIsotopeSymbolFrame: CGRect?
        private var renderedHazardSymbolFrame: CGRect?
        private var renderedInstalledIsotopeState = false
        private var renderedHomebrewMigrationCandidateState = false
        private var renderedHazardState = false
        private var renderedHazardSource: PackageSecurityNotice.Source?

        init(package: PackagePresentation) {
            self.package = package
            super.init()
            configureLayer()
            updateText()
        }

        override init(layer: Any) {
            guard let layer = layer as? PackageNodeLayer else {
                fatalError("Unsupported layer copy")
            }
            package = layer.package
            super.init(layer: layer)
            basePosition = layer.basePosition
            baseOpacity = layer.baseOpacity
            isHovered = layer.isHovered
            isSelectedNode = layer.isSelectedNode
            matchesSearch = layer.matchesSearch
            isEntering = layer.isEntering
        }

        required init?(coder: NSCoder) {
            fatalError("init(coder:) has not been implemented")
        }

        override func layoutSublayers() {
            super.layoutSublayers()
            textLayer.frame = CGRect(
                x: 0,
                y: 0,
                width: bounds.width,
                height: bounds.height
            )
            updateTextIfNeeded(force: true)
        }

        private func configureLayer() {
            anchorPoint = CGPoint(x: 0.5, y: 0.5)
            actions = [
                "position": NSNull(),
                "opacity": NSNull(),
                "transform": NSNull(),
                "shadowOpacity": NSNull(),
                "shadowRadius": NSNull()
            ]
            shadowOffset = .zero
            shadowColor = UIStyle.accentShadow.cgColor
            shadowRadius = 0
            shadowOpacity = 0

            textLayer.contentsScale = NSScreen.main?.backingScaleFactor ?? 2
            textLayer.alignmentMode = .left
            textLayer.isWrapped = true
            if textLayer.superlayer == nil {
                addSublayer(textLayer)
            }
            isotopeSymbolLayer.contentsScale = NSScreen.main?.backingScaleFactor ?? 2
            isotopeSymbolLayer.actions = [
                "contents": NSNull(),
                "position": NSNull(),
                "bounds": NSNull(),
                "opacity": NSNull()
            ]
            if isotopeSymbolLayer.superlayer == nil {
                addSublayer(isotopeSymbolLayer)
            }
            hazardEffect.install(in: self)
            updateHazardAppearance()
        }

        func updateText() {
            setNeedsLayout()
        }

        func hazardSymbolFrameInParent() -> CGRect? {
            layoutIfNeeded()
            guard let renderedHazardSymbolFrame else {
                return nil
            }
            return renderedHazardSymbolFrame.offsetBy(
                dx: frame.minX,
                dy: frame.minY
            )
        }

        func textFrameInParent() -> CGRect {
            layoutIfNeeded()
            return textLayer.frame.offsetBy(
                dx: frame.minX,
                dy: frame.minY
            )
        }

        func hazardTitleFrameInParent() -> CGRect? {
            layoutIfNeeded()
            guard package.hasPlainTextSecretAlert else {
                return nil
            }
            let title = attributedTitle(includeStatusSymbols: true)
            let titleBounds = title.boundingRect(
                with: CGSize(
                    width: CGFloat.greatestFiniteMagnitude,
                    height: CGFloat.greatestFiniteMagnitude
                ),
                options: [.usesLineFragmentOrigin, .usesFontLeading]
            )
            return CGRect(
                x: textLayer.frame.minX + frame.minX,
                y: renderedTextBounds.minY + frame.minY,
                width: ceil(titleBounds.width),
                height: ceil(titleBounds.height)
            )
        }

        var renderedText: Any? {
            textLayer.string
        }

        private func updateTextIfNeeded(force: Bool) {
            guard bounds.width > 0 else { return }
            let needsRebuild = force
                || renderedWidth != bounds.width
                || renderedPackage != package
                || renderedTitleColor != titleColor
                || renderedVersionColor != versionColor
                || renderedDescriptionColor != descriptionColor
                || renderedInstalledIsotopeState != package.isInstalledIsotope
                || renderedHomebrewMigrationCandidateState != package.isHomebrewInstall
                || renderedHazardState != package.hasPlainTextSecretAlert
                || renderedHazardSource != package.plainTextSecretAlertSource
            guard needsRebuild else { return }
            rebuildText(maxWidth: bounds.width)
            renderedWidth = bounds.width
            renderedPackage = package
            renderedTitleColor = titleColor
            renderedVersionColor = versionColor
            renderedDescriptionColor = descriptionColor
            renderedInstalledIsotopeState = package.isInstalledIsotope
            renderedHomebrewMigrationCandidateState = package.isHomebrewInstall
            renderedHazardState = package.hasPlainTextSecretAlert
            renderedHazardSource = package.plainTextSecretAlertSource
            updateHazardAppearance()
        }

        private func rebuildText(maxWidth: CGFloat) {
            let title = attributedTitle(includeStatusSymbols: true)
            let rendered = NSMutableAttributedString(attributedString: title)
            if shouldInlineSecondaryText() || !titleFitsSingleLine(title, maxWidth: maxWidth) {
                rendered.append(NSAttributedString(string: inlineSecondaryTextGap()))
            } else {
                rendered.append(
                    NSAttributedString(
                        string: String(repeating: "\n", count: Metrics.nameVersionGapLines)
                    )
                )
            }
            rendered.append(attributedSecondaryText())
            textLayer.string = rendered
            let measured = rendered.boundingRect(
                with: CGSize(width: maxWidth, height: CGFloat.greatestFiniteMagnitude),
                options: [.usesLineFragmentOrigin, .usesFontLeading]
            )
            let width = min(maxWidth, ceil(measured.width))
            let height = min(bounds.height, ceil(measured.height))
            let y = round((bounds.height - height) / 2)
            let textRect = CGRect(
                x: textLayer.frame.minX,
                y: y,
                width: width,
                height: height
            )
            renderedTextBounds = textRect
            renderedIsotopeSymbolFrame = isotopeSymbolFrame(textTopY: y)
            renderedHazardSymbolFrame = hazardSymbolFrame(textTopY: y)
            layoutIsotopeSymbol()
            hazardEffect.layout(in: bounds, symbolFrame: renderedHazardSymbolFrame)
        }

        private func attributedTitle(includeStatusSymbols: Bool) -> NSAttributedString {
            let title = NSMutableAttributedString(
                string: package.displayName,
                attributes: [
                    .font: UIStyle.monoFont(size: 13),
                    .foregroundColor: titleColor,
                    .kern: 0.2
                ]
            )
            guard includeStatusSymbols else {
                return title
            }

            if package.isInstalledIsotope {
                title.append(statusSymbolGap())
                title.append(installedIsotopeSymbolSpacer())
            }

            if package.isHomebrewInstall {
                title.append(statusSymbolGap())
                title.append(homebrewWarningSymbol())
            }

            if package.hasPlainTextSecretAlert {
                title.append(statusSymbolGap())
                title.append(hazardSymbol())
            }
            return title
        }

        private func hazardSymbolFrame(textTopY: CGFloat) -> CGRect? {
            guard package.hasPlainTextSecretAlert else {
                return nil
            }
            let prefix = NSMutableAttributedString(
                attributedString: attributedTitle(includeStatusSymbols: false)
            )
            if package.isInstalledIsotope {
                prefix.append(statusSymbolGap())
                prefix.append(installedIsotopeSymbolSpacer())
            }
            if package.isHomebrewInstall {
                prefix.append(statusSymbolGap())
                prefix.append(homebrewWarningSymbol())
            }
            let gap = statusSymbolGap()
            let symbol = hazardSymbol()
            let prefixWidth = ceil(
                prefix.boundingRect(
                    with: CGSize(
                        width: CGFloat.greatestFiniteMagnitude,
                        height: CGFloat.greatestFiniteMagnitude
                    ),
                    options: [.usesLineFragmentOrigin, .usesFontLeading]
                ).width
            )
            let gapWidth = ceil(
                gap.boundingRect(
                    with: CGSize(
                        width: CGFloat.greatestFiniteMagnitude,
                        height: CGFloat.greatestFiniteMagnitude
                    ),
                    options: [.usesLineFragmentOrigin, .usesFontLeading]
                ).width
            )
            let symbolBounds = symbol.boundingRect(
                with: CGSize(
                    width: CGFloat.greatestFiniteMagnitude,
                    height: CGFloat.greatestFiniteMagnitude
                ),
                options: [.usesLineFragmentOrigin, .usesFontLeading]
            )
            return CGRect(
                x: textLayer.frame.minX + prefixWidth + gapWidth,
                y: textTopY,
                width: ceil(symbolBounds.width),
                height: ceil(symbolBounds.height)
            )
        }

        private func statusSymbolGap() -> NSAttributedString {
            NSAttributedString(
                string: " ",
                attributes: [
                    .font: UIStyle.monoFont(size: Metrics.hazardSymbolGapFontSize),
                    .kern: 0
                ]
            )
        }

        private func installedIsotopeSymbolSpacer() -> NSAttributedString {
            NSAttributedString(
                string: " ",
                attributes: [
                    .font: NSFont.systemFont(
                        ofSize: Metrics.isotopeSymbolSize,
                        weight: .medium
                    ),
                    .foregroundColor: NSColor.clear,
                    .kern: 0
                ]
            )
        }

        private func homebrewWarningSymbol() -> NSAttributedString {
            NSAttributedString(
                string: "⚠",
                attributes: [
                    .font: UIStyle.monoFont(
                        size: Metrics.homebrewSymbolFontSize,
                        weight: .medium
                    ),
                    .foregroundColor: UIStyle.warning,
                    .kern: 0.2
                ]
            )
        }

        private func isotopeSymbolFrame(textTopY: CGFloat) -> CGRect? {
            guard package.isInstalledIsotope else {
                return nil
            }
            let prefix = NSMutableAttributedString(
                attributedString: attributedTitle(includeStatusSymbols: false)
            )
            let gap = statusSymbolGap()
            let spacer = installedIsotopeSymbolSpacer()
            let titleBounds = prefix.boundingRect(
                with: CGSize(
                    width: CGFloat.greatestFiniteMagnitude,
                    height: CGFloat.greatestFiniteMagnitude
                ),
                options: [.usesLineFragmentOrigin, .usesFontLeading]
            )
            let prefixWidth = ceil(
                prefix.boundingRect(
                    with: CGSize(
                        width: CGFloat.greatestFiniteMagnitude,
                        height: CGFloat.greatestFiniteMagnitude
                    ),
                    options: [.usesLineFragmentOrigin, .usesFontLeading]
                ).width
            )
            let gapWidth = ceil(
                gap.boundingRect(
                    with: CGSize(
                        width: CGFloat.greatestFiniteMagnitude,
                        height: CGFloat.greatestFiniteMagnitude
                    ),
                    options: [.usesLineFragmentOrigin, .usesFontLeading]
                ).width
            )
            let spacerBounds = spacer.boundingRect(
                with: CGSize(
                    width: CGFloat.greatestFiniteMagnitude,
                    height: CGFloat.greatestFiniteMagnitude
                ),
                options: [.usesLineFragmentOrigin, .usesFontLeading]
            )
            let firstLineOffset = max(
                0,
                renderedTextBounds.height - ceil(titleBounds.height)
            )
            let titleBaselineOffset = abs(UIStyle.monoFont(size: 13).descender)
            return CGRect(
                x: textLayer.frame.minX + prefixWidth + gapWidth,
                y: textTopY - firstLineOffset + titleBaselineOffset,
                width: ceil(spacerBounds.width),
                height: Metrics.isotopeSymbolSize
            )
        }

        private func layoutIsotopeSymbol() {
            guard package.isInstalledIsotope,
                  let frame = renderedIsotopeSymbolFrame,
                  let image = installedIsotopeSymbolImage() else {
                isotopeSymbolLayer.contents = nil
                isotopeSymbolLayer.frame = .zero
                return
            }

            isotopeSymbolLayer.contents = image
            isotopeSymbolLayer.frame = CGRect(
                x: frame.minX,
                y: frame.minY,
                width: Metrics.isotopeSymbolSize,
                height: Metrics.isotopeSymbolSize
            )
        }

        private func installedIsotopeSymbolImage() -> CGImage? {
            let symbolConfiguration = NSImage.SymbolConfiguration(
                pointSize: Metrics.isotopeSymbolSize,
                weight: .medium,
                scale: .small
            ).applying(
                NSImage.SymbolConfiguration(paletteColors: [UIStyle.accent])
            )

            guard let image = NSImage(
                systemSymbolName: "lock.fill",
                accessibilityDescription: "Installed isotope"
            )?.withSymbolConfiguration(symbolConfiguration) else {
                return nil
            }

            image.isTemplate = false
            return image.cgImage(
                forProposedRect: nil,
                context: nil,
                hints: nil
            )
        }

        private func hazardSymbol() -> NSAttributedString {
            NSAttributedString(
                string: "☢",
                attributes: [
                    .font: UIStyle.monoFont(
                        size: Metrics.hazardSymbolFontSize,
                        weight: .medium
                    ),
                    .foregroundColor: UIStyle.danger,
                    .kern: 0.2
                ]
            )
        }

        private func titleFitsSingleLine(
            _ title: NSAttributedString,
            maxWidth: CGFloat
        ) -> Bool {
            let measured = title.boundingRect(
                with: CGSize(
                    width: CGFloat.greatestFiniteMagnitude,
                    height: CGFloat.greatestFiniteMagnitude
                ),
                options: [.usesLineFragmentOrigin, .usesFontLeading]
            )
            return ceil(measured.width) <= maxWidth
        }

        private func inlineSecondaryTextGap() -> String {
            shouldInlineSecondaryText()
                ? Metrics.inlineDescriptionGap
                : Metrics.inlineMetadataGap
        }

        private func shouldInlineSecondaryText() -> Bool {
            switch package.item {
            case .installed:
                return false
            case .recommendation:
                return true
            case .available(let result):
                guard let description = result.description?.trimmingCharacters(
                    in: .whitespacesAndNewlines
                ) else {
                    return false
                }
                return !description.isEmpty
            case .command:
                return true
            }
        }

        private func attributedSecondaryText() -> NSAttributedString {
            switch package.item {
            case .installed(let record):
                if record.isOutdated, let latestVersion = record.latestVersion {
                    let current = NSMutableAttributedString(
                        attributedString: UIStyle.attributedMonoText(
                            "v\(record.version)",
                            size: 10,
                            color: UIStyle.danger
                        )
                    )
                    current.addAttribute(
                        .baselineOffset,
                        value: Metrics.versionBaselineOffset,
                        range: NSRange(location: 0, length: current.length)
                    )
                    let suffix = NSMutableAttributedString(
                        attributedString: UIStyle.attributedMonoText(
                            " -> \(latestVersion)",
                            size: 10,
                            color: versionColor
                        )
                    )
                    suffix.addAttribute(
                        .baselineOffset,
                        value: Metrics.versionBaselineOffset,
                        range: NSRange(location: 0, length: suffix.length)
                    )
                    let text = NSMutableAttributedString(attributedString: current)
                    text.append(suffix)
                    return text
                }
            case .recommendation(let recommendation):
                if recommendation.isOutdated,
                   let installedVersion = recommendation.installedVersion,
                   let latestVersion = recommendation.latestVersion {
                    let current = NSMutableAttributedString(
                        attributedString: UIStyle.attributedMonoText(
                            "v\(installedVersion)",
                            size: 10,
                            color: UIStyle.danger
                        )
                    )
                    current.addAttribute(
                        .baselineOffset,
                        value: Metrics.versionBaselineOffset,
                        range: NSRange(location: 0, length: current.length)
                    )
                    let suffix = NSMutableAttributedString(
                        attributedString: UIStyle.attributedMonoText(
                            " -> \(latestVersion)",
                            size: 10,
                            color: versionColor
                        )
                    )
                    suffix.addAttribute(
                        .baselineOffset,
                        value: Metrics.versionBaselineOffset,
                        range: NSRange(location: 0, length: suffix.length)
                    )
                    let text = NSMutableAttributedString(attributedString: current)
                    text.append(suffix)
                    return text
                }
                fallthrough
            case .available:
                let text = NSMutableAttributedString(
                    attributedString: UIStyle.attributedMonoText(
                        package.listSecondaryText,
                        size: 10,
                        color: descriptionColor
                    )
                )
                text.addAttribute(
                    .baselineOffset,
                    value: Metrics.descriptionBaselineOffset,
                    range: NSRange(location: 0, length: text.length)
                )
                return text
            case .command:
                let text = NSMutableAttributedString(
                    attributedString: UIStyle.attributedMonoText(
                        package.listSecondaryText,
                        size: 10,
                        color: descriptionColor
                    )
                )
                text.addAttribute(
                    .baselineOffset,
                    value: Metrics.descriptionBaselineOffset,
                    range: NSRange(location: 0, length: text.length)
                )
                return text
            }

            let text = NSMutableAttributedString(
                attributedString: UIStyle.attributedMonoText(
                    package.listSecondaryText,
                    size: 10,
                    color: versionColor
                )
            )
            text.addAttribute(
                .baselineOffset,
                value: Metrics.versionBaselineOffset,
                range: NSRange(location: 0, length: text.length)
            )
            return text
        }

        func focusBounds() -> CGRect {
            renderedTextBounds
        }

        private func updateHazardAppearance() {
            hazardEffect.update(source: package.plainTextSecretAlertSource)
        }
    }

    private final class LatentGridLayer: CALayer {
        private enum Axis {
            case vertical
            case horizontal
        }

        private final class GridLineLayer: CAShapeLayer {
            let axis: Axis
            let anchor: CGFloat

            init(axis: Axis, anchor: CGFloat) {
                self.axis = axis
                self.anchor = anchor
                super.init()
            }

            override init(layer: Any) {
                guard let layer = layer as? GridLineLayer else {
                    fatalError("Unsupported layer copy")
                }
                axis = layer.axis
                anchor = layer.anchor
                super.init(layer: layer)
            }

            required init?(coder: NSCoder) {
                fatalError("init(coder:) has not been implemented")
            }
        }

        private var verticalLineLayers: [GridLineLayer] = []
        private var horizontalLineLayers: [GridLineLayer] = []
        private var hoverVerticalAnchor: CGFloat?
        private var hoverHorizontalAnchor: CGFloat?
        private var selectedVerticalAnchor: CGFloat?
        private var selectedHorizontalAnchor: CGFloat?
        private var hoverIsHazard = false
        private var selectedIsHazard = false
        private var cursorPoint: CGPoint?
        private var cursorInfluenceStrength: CGFloat = 0

        override init() {
            super.init()
            actions = [
                "position": NSNull(),
                "bounds": NSNull(),
                "opacity": NSNull()
            ]
            zPosition = 0
        }

        override init(layer: Any) {
            super.init(layer: layer)
            if let layer = layer as? LatentGridLayer {
                hoverVerticalAnchor = layer.hoverVerticalAnchor
                hoverHorizontalAnchor = layer.hoverHorizontalAnchor
                selectedVerticalAnchor = layer.selectedVerticalAnchor
                selectedHorizontalAnchor = layer.selectedHorizontalAnchor
                hoverIsHazard = layer.hoverIsHazard
                selectedIsHazard = layer.selectedIsHazard
                cursorPoint = layer.cursorPoint
                cursorInfluenceStrength = layer.cursorInfluenceStrength
            }
        }

        required init?(coder: NSCoder) {
            fatalError("init(coder:) has not been implemented")
        }

        func updateGrid(
            verticalAnchors: [CGFloat],
            horizontalAnchors: [CGFloat],
            hoverVerticalAnchor: CGFloat?,
            hoverHorizontalAnchor: CGFloat?,
            selectedVerticalAnchor: CGFloat?,
            selectedHorizontalAnchor: CGFloat?,
            hoverIsHazard: Bool,
            selectedIsHazard: Bool
        ) {
            self.hoverVerticalAnchor = hoverVerticalAnchor
            self.hoverHorizontalAnchor = hoverHorizontalAnchor
            self.selectedVerticalAnchor = selectedVerticalAnchor
            self.selectedHorizontalAnchor = selectedHorizontalAnchor
            self.hoverIsHazard = hoverIsHazard
            self.selectedIsHazard = selectedIsHazard
            syncLineLayers(
                &verticalLineLayers,
                anchors: verticalAnchors,
                axis: .vertical
            )
            syncLineLayers(
                &horizontalLineLayers,
                anchors: horizontalAnchors,
                axis: .horizontal
            )
            updatePaths()
            updateVisuals()
        }

        func updateCursorFocus(point: CGPoint?, influenceStrength: CGFloat) {
            cursorPoint = point
            cursorInfluenceStrength = influenceStrength
            updateVisuals()
        }

        override func layoutSublayers() {
            super.layoutSublayers()
            updatePaths()
            updateVisuals()
        }

        private func syncLineLayers(
            _ lineLayers: inout [GridLineLayer],
            anchors: [CGFloat],
            axis: Axis
        ) {
            while lineLayers.count > anchors.count {
                lineLayers.removeLast().removeFromSuperlayer()
            }

            for (index, anchor) in anchors.enumerated() {
                let lineLayer: GridLineLayer
                if index < lineLayers.count {
                    lineLayer = lineLayers[index]
                    if abs(lineLayer.anchor - anchor) > 0.5 {
                        lineLayer.removeFromSuperlayer()
                        let replacement = makeLineLayer(axis: axis, anchor: anchor)
                        lineLayers[index] = replacement
                        addSublayer(replacement)
                        continue
                    }
                } else {
                    let created = makeLineLayer(axis: axis, anchor: anchor)
                    lineLayers.append(created)
                    addSublayer(created)
                }
            }
        }

        private func makeLineLayer(axis: Axis, anchor: CGFloat) -> GridLineLayer {
            let lineLayer = GridLineLayer(axis: axis, anchor: anchor)
            lineLayer.fillColor = nil
            lineLayer.strokeColor = UIStyle.accent.cgColor
            lineLayer.lineWidth = pixelWidth()
            lineLayer.lineCap = .butt
            lineLayer.opacity = 0
            lineLayer.shadowColor = UIStyle.accent.cgColor
            lineLayer.shadowOffset = .zero
            lineLayer.shadowRadius = 0
            lineLayer.shadowOpacity = 0
            lineLayer.actions = [
                "path": NSNull(),
                "opacity": NSNull(),
                "strokeColor": NSNull(),
                "shadowColor": NSNull(),
                "shadowOpacity": NSNull(),
                "shadowRadius": NSNull()
            ]
            return lineLayer
        }

        private func updatePaths() {
            let stroke = pixelWidth()

            for lineLayer in verticalLineLayers {
                let x = aligned(lineLayer.anchor)
                lineLayer.lineWidth = stroke
                let path = CGMutablePath()
                path.move(to: CGPoint(x: x, y: 0))
                path.addLine(to: CGPoint(x: x, y: bounds.height))
                lineLayer.path = path
                lineLayer.frame = bounds
            }

            for lineLayer in horizontalLineLayers {
                let y = aligned(lineLayer.anchor)
                lineLayer.lineWidth = stroke
                let path = CGMutablePath()
                path.move(to: CGPoint(x: 0, y: y))
                path.addLine(to: CGPoint(x: bounds.width, y: y))
                lineLayer.path = path
                lineLayer.frame = bounds
            }
        }

        private func updateVisuals() {
            for lineLayer in verticalLineLayers {
                updateVisuals(
                    for: lineLayer,
                    distance: cursorPoint.map { abs($0.x - lineLayer.anchor) } ?? .greatestFiniteMagnitude,
                    hoverMatch: hoverVerticalAnchor.map { abs($0 - lineLayer.anchor) < 0.5 } ?? false,
                    selectionMatch: selectedVerticalAnchor.map { abs($0 - lineLayer.anchor) < 0.5 } ?? false,
                    range: Metrics.cursorInfluenceHalfWidth
                )
            }

            for lineLayer in horizontalLineLayers {
                updateVisuals(
                    for: lineLayer,
                    distance: cursorPoint.map { abs($0.y - lineLayer.anchor) } ?? .greatestFiniteMagnitude,
                    hoverMatch: hoverHorizontalAnchor.map { abs($0 - lineLayer.anchor) < 0.5 } ?? false,
                    selectionMatch: selectedHorizontalAnchor.map { abs($0 - lineLayer.anchor) < 0.5 } ?? false,
                    range: Metrics.cursorInfluenceHalfHeight
                )
            }
        }

        private func updateVisuals(
            for lineLayer: GridLineLayer,
            distance: CGFloat,
            hoverMatch: Bool,
            selectionMatch: Bool,
            range: CGFloat
        ) {
            let baseOpacity: Float
            if hoverMatch {
                baseOpacity = Metrics.hoverGridOpacity
            } else if selectionMatch {
                baseOpacity = Metrics.selectionGridOpacity
            } else {
                baseOpacity = 0
            }
            let cursorOpacity: Float
            let glowStrength: Float

            if hoverMatch, distance <= range, cursorInfluenceStrength > 0 {
                let normalized = 1 - (distance / range)
                let eased = normalized * normalized
                cursorOpacity = Float(eased * cursorInfluenceStrength) * Metrics.gridInfluenceOpacity
                glowStrength = Float(eased * cursorInfluenceStrength)
            } else {
                cursorOpacity = 0
                glowStrength = 0
            }

            CATransaction.begin()
            CATransaction.setDisableActions(true)
            let color = gridColor(
                hoverMatch: hoverMatch,
                selectionMatch: selectionMatch
            )
            lineLayer.strokeColor = color.cgColor
            lineLayer.shadowColor = color.cgColor
            lineLayer.opacity = baseOpacity + cursorOpacity
            lineLayer.shadowRadius = glowStrength > 0 ? Metrics.gridGlowRadius : 0
            lineLayer.shadowOpacity = min(glowStrength * 0.28, 0.28)
            CATransaction.commit()
        }

        private func gridColor(hoverMatch: Bool, selectionMatch: Bool) -> NSColor {
            if hoverMatch, hoverIsHazard {
                return UIStyle.danger
            }
            if selectionMatch, selectedIsHazard {
                return UIStyle.danger
            }
            return UIStyle.accent
        }

        private func pixelWidth() -> CGFloat {
            1 / (NSScreen.main?.backingScaleFactor ?? 2)
        }

        private func aligned(_ value: CGFloat) -> CGFloat {
            let scale = NSScreen.main?.backingScaleFactor ?? 2
            return round(value * scale) / scale
        }
    }

    private final class CornerBracketLayer: CALayer {
        private let hoverLayer = CAShapeLayer()
        private let selectionLayer = CAShapeLayer()

        override init() {
            super.init()
            configureLayer(hoverLayer, opacity: Metrics.hoverBracketOpacity)
            configureLayer(selectionLayer, opacity: Metrics.selectedBracketOpacity)
        }

        override init(layer: Any) {
            super.init(layer: layer)
            configureLayer(hoverLayer, opacity: Metrics.hoverBracketOpacity)
            configureLayer(selectionLayer, opacity: Metrics.selectedBracketOpacity)
        }

        required init?(coder: NSCoder) {
            fatalError("init(coder:) has not been implemented")
        }

        func update(
            hoverFrame: CGRect?,
            selectedFrame: CGRect?,
            hoverIsHazard: Bool,
            selectedIsHazard: Bool
        ) {
            let normalizedSelected = selectedFrame.map(normalizedBracketFrame)
            let normalizedHover = hoverFrame.map(normalizedBracketFrame)

            CATransaction.begin()
            CATransaction.setDisableActions(true)
            updateColor(for: selectionLayer, isHazard: selectedIsHazard)
            selectionLayer.path = normalizedSelected.map(bracketPath(for:))
            selectionLayer.opacity = normalizedSelected == nil ? 0 : Metrics.selectedBracketOpacity

            if normalizedHover == normalizedSelected {
                hoverLayer.path = nil
                hoverLayer.opacity = 0
            } else {
                updateColor(for: hoverLayer, isHazard: hoverIsHazard)
                hoverLayer.path = normalizedHover.map(bracketPath(for:))
                hoverLayer.opacity = normalizedHover == nil ? 0 : Metrics.hoverBracketOpacity
            }
            CATransaction.commit()
        }

        private func configureLayer(_ layer: CAShapeLayer, opacity: Float) {
            layer.fillColor = nil
            layer.strokeColor = UIStyle.accent.cgColor
            layer.lineWidth = pixelWidth()
            layer.lineCap = .square
            layer.lineJoin = .miter
            layer.opacity = opacity
            layer.actions = [
                "path": NSNull(),
                "opacity": NSNull(),
                "strokeColor": NSNull()
            ]
            addSublayer(layer)
        }

        private func updateColor(for layer: CAShapeLayer, isHazard: Bool) {
            let color = isHazard ? UIStyle.danger : UIStyle.accent
            layer.strokeColor = color.cgColor
        }

        private func normalizedBracketFrame(_ frame: CGRect) -> CGRect {
            frame.insetBy(
                dx: -Metrics.bracketInsetX,
                dy: -Metrics.bracketInsetY
            ).integral
        }

        private func bracketPath(for frame: CGRect) -> CGPath {
            let path = CGMutablePath()
            let arm = Metrics.bracketArmLength

            path.move(to: CGPoint(x: frame.minX, y: frame.minY + arm))
            path.addLine(to: CGPoint(x: frame.minX, y: frame.minY))
            path.addLine(to: CGPoint(x: frame.minX + arm, y: frame.minY))

            path.move(to: CGPoint(x: frame.maxX - arm, y: frame.minY))
            path.addLine(to: CGPoint(x: frame.maxX, y: frame.minY))
            path.addLine(to: CGPoint(x: frame.maxX, y: frame.minY + arm))

            path.move(to: CGPoint(x: frame.minX, y: frame.maxY - arm))
            path.addLine(to: CGPoint(x: frame.minX, y: frame.maxY))
            path.addLine(to: CGPoint(x: frame.minX + arm, y: frame.maxY))

            path.move(to: CGPoint(x: frame.maxX - arm, y: frame.maxY))
            path.addLine(to: CGPoint(x: frame.maxX, y: frame.maxY))
            path.addLine(to: CGPoint(x: frame.maxX, y: frame.maxY - arm))

            return path
        }

        private func pixelWidth() -> CGFloat {
            1 / (NSScreen.main?.backingScaleFactor ?? 2)
        }
    }

    weak var delegate: PackageFieldViewDelegate?

    private let contentLayer = CALayer()
    private let panelHeaderLayer = CATextLayer()
    private let panelHeaderCountLayer = CATextLayer()
    private let latentGridLayer = LatentGridLayer()
    private let cornerBracketLayer = CornerBracketLayer()
    private let hazardSmokeBracketLayer = CornerBracketLayer()
    private let sectionDividerLayer = CALayer()
    private let sectionLabelLayer = CATextLayer()
    private let emptyStateLayer = CATextLayer()
    private let commandPaletteQuoteLayer = CATextLayer()
    private var panelHeaderAnimator: LayerGlitchTextAnimator?
    private var trackingArea: NSTrackingArea?
    private var nodeLayers: [String: PackageNodeLayer] = [:]
    private var orderedPackages: [PackagePresentation] = []
    private var selectedPackageID: String?
    private var hoveredPackageID: String?
    private var searchQuery = ""
    private var panelHeaderTitle = "INSTALLED"
    private var panelHeaderCount: Int?
    private var panelHeaderLoading = false
    private var cursorAnimationTimer: Timer?
    private var cursorTargetPoint: CGPoint?
    private var cursorDisplayPoint: CGPoint?
    private var cursorInfluenceStrength: CGFloat = 0
    private var secondarySectionTitle = "DISCOVERY"
    private var secondarySectionCount: Int?
    private var commandPaletteHelpText: NSAttributedString?
    private var commandPaletteQuoteText: NSAttributedString?
    private weak var hazardSmokeHostLayer: CALayer?
    private weak var hazardSmokeCoordinateView: NSView?

    private struct VisualEntry {
        let packageID: String
        let row: Int
        let column: Int
    }

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        wantsLayer = true
        layer = CALayer()
        layer?.backgroundColor = UIStyle.background.cgColor
        panelHeaderLayer.contentsScale = NSScreen.main?.backingScaleFactor ?? 2
        panelHeaderLayer.alignmentMode = .left
        layer?.addSublayer(panelHeaderLayer)
        panelHeaderCountLayer.contentsScale = NSScreen.main?.backingScaleFactor ?? 2
        panelHeaderCountLayer.alignmentMode = .left
        layer?.addSublayer(panelHeaderCountLayer)
        panelHeaderAnimator = LayerGlitchTextAnimator(
            layer: panelHeaderLayer,
            size: 10,
            baseColor: UIStyle.text.withAlphaComponent(0.20),
            glitchColor: UIStyle.accent.withAlphaComponent(0.66),
            weight: .regular,
            tracking: 1.8
        )
        updatePanelHeader(isLoading: false)
        layer?.addSublayer(contentLayer)
        contentLayer.addSublayer(latentGridLayer)
        layer?.addSublayer(sectionDividerLayer)
        layer?.addSublayer(sectionLabelLayer)
        layer?.addSublayer(cornerBracketLayer)
        layer?.addSublayer(emptyStateLayer)
        layer?.addSublayer(commandPaletteQuoteLayer)
        contentLayer.actions = ["sublayers": NSNull()]
        sectionDividerLayer.backgroundColor = UIStyle.separator.cgColor
        sectionLabelLayer.contentsScale = NSScreen.main?.backingScaleFactor ?? 2
        sectionLabelLayer.alignmentMode = .left
        sectionLabelLayer.string = discoverySectionLabel()
        emptyStateLayer.contentsScale = NSScreen.main?.backingScaleFactor ?? 2
        emptyStateLayer.alignmentMode = .left
        emptyStateLayer.isWrapped = true
        emptyStateLayer.string = nil
        emptyStateLayer.isHidden = true
        commandPaletteQuoteLayer.contentsScale = NSScreen.main?.backingScaleFactor ?? 2
        commandPaletteQuoteLayer.alignmentMode = .left
        commandPaletteQuoteLayer.isWrapped = true
        commandPaletteQuoteLayer.string = nil
        commandPaletteQuoteLayer.isHidden = true
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    deinit {
        cursorAnimationTimer?.invalidate()
    }

    override var isFlipped: Bool {
        true
    }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        if let trackingArea {
            removeTrackingArea(trackingArea)
        }
        let options: NSTrackingArea.Options = [.activeInKeyWindow, .mouseMoved, .mouseEnteredAndExited, .inVisibleRect]
        trackingArea = NSTrackingArea(rect: bounds, options: options, owner: self, userInfo: nil)
        addTrackingArea(trackingArea!)
    }

    override func layout() {
        super.layout()
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        let panelHeaderMinX = Metrics.panelHeaderSpineX + Metrics.panelHeaderTextInset
        panelHeaderLayer.frame = CGRect(
            x: panelHeaderMinX,
            y: Metrics.panelHeaderTopInset,
            width: max(bounds.width - panelHeaderMinX - Metrics.trailingInset, 120),
            height: Metrics.panelHeaderLabelHeight
        )
        let panelHeaderWidth = panelHeaderTextWidth(panelHeaderTitle)
        panelHeaderCountLayer.frame = CGRect(
            x: panelHeaderMinX + panelHeaderWidth + 6,
            y: Metrics.panelHeaderTopInset,
            width: max(bounds.width - panelHeaderMinX - panelHeaderWidth - 6 - Metrics.trailingInset, 0),
            height: Metrics.panelHeaderLabelHeight
        )
        contentLayer.frame = bounds
        latentGridLayer.frame = contentLayer.bounds
        cornerBracketLayer.frame = bounds
        emptyStateLayer.frame = CGRect(
            x: Metrics.leadingInset,
            y: Metrics.helpTopInset,
            width: bounds.width - Metrics.leadingInset - Metrics.trailingInset,
            height: Metrics.helpHeight
        )
        commandPaletteQuoteLayer.frame = commandPaletteQuoteFrame()
        relayoutNodes(animated: false)
        CATransaction.commit()
    }

    func contentHeight(forWidth width: CGFloat) -> CGFloat {
        let usableWidth = max(
            width - Metrics.leadingInset - Metrics.trailingInset,
            Metrics.minNodeWidth
        )
        let layout = makeGridLayout(usableWidth: usableWidth)
        let dividerIndex = orderedPackages.firstIndex(where: { $0.item.isAvailable })
        let installedCount = dividerIndex ?? orderedPackages.count
        let availableCount = max(orderedPackages.count - installedCount, 0)
        let installedRows = installedCount > 0 ? (installedCount + layout.columns - 1) / layout.columns : 0
        let availableRows = availableCount > 0 ? (availableCount + layout.columns - 1) / layout.columns : 0
        let rowStride = Metrics.nodeHeight + Metrics.rowGap
        let dividerGap = shouldShowSecondarySectionLabel(
            dividerIndex: dividerIndex,
            installedCount: installedCount
        ) ? Metrics.sectionGap : 0
        let helpHeight = commandPaletteHelpText == nil
            ? 0
            : Metrics.helpTopInset
                + Metrics.helpHeight
                + Metrics.helpBottomGap
                + Metrics.commandSectionLabelHeight
                - Metrics.topInset
        let rowsHeight = CGFloat(max(installedRows + availableRows, 0)) * rowStride
        let quoteHeight = commandPaletteQuoteText == nil
            ? 0
            : Metrics.quoteTopGap + Metrics.quoteHeight
        let totalHeight = Metrics.topInset
            + helpHeight
            + rowsHeight
            + dividerGap
            + quoteHeight
            + Metrics.bottomInset
        return max(totalHeight, 160)
    }

    func installHazardSmoke(in layer: CALayer, coordinateView: NSView) {
        hazardSmokeHostLayer = layer
        hazardSmokeCoordinateView = coordinateView
        if hazardSmokeBracketLayer.superlayer !== layer {
            hazardSmokeBracketLayer.removeFromSuperlayer()
            hazardSmokeBracketLayer.zPosition = 4
            layer.addSublayer(hazardSmokeBracketLayer)
        }
        for nodeLayer in nodeLayers.values {
            nodeLayer.hazardEffect.installSmoke(in: layer)
        }
        updateHazardSmokeGeometry()
    }

    func refreshHazardSmokeLayout() {
        updateHazardSmokeBracketsForCurrentState()
        updateHazardSmokeGeometry()
    }

    func apply(
        packages: [PackagePresentation],
        selectedPackageName: String?,
        searchQuery: String,
        secondarySectionTitle: String,
        secondarySectionCount: Int?,
        panelHeaderTitle: String,
        panelHeaderCount: Int?,
        commandPaletteHelpText: NSAttributedString? = nil,
        commandPaletteQuoteText: NSAttributedString? = nil
    ) {
        let previous = Set(orderedPackages.map(\.selectionID))
        let incoming = Set(packages.map(\.selectionID))
        let removedPackageNames = previous.subtracting(incoming)
        let insertedPackageNames = Set(
            packages.lazy
                .map(\.selectionID)
                .filter { previous.contains($0) == false }
        )
        let recommendationPackageNames = Set(packages.compactMap { package in
            if case .recommendation = package.item {
                return package.selectionID
            }
            return nil
        })
        let insertedRecommendationsOnly = insertedPackageNames.isEmpty == false
            && insertedPackageNames.isSubset(of: recommendationPackageNames)
        let shouldAnimateTransition = (
            max(previous.count, packages.count) <= 40
                && insertedPackageNames.count <= 24
        ) || (
            removedPackageNames.isEmpty
                && insertedRecommendationsOnly
                && insertedPackageNames.count <= 8
        )
        let shouldAnimateRelayout = shouldAnimateTransition && removedPackageNames.isEmpty
        self.orderedPackages = packages
        self.selectedPackageID = selectedPackageName
        self.searchQuery = searchQuery
        self.secondarySectionTitle = secondarySectionTitle
        self.secondarySectionCount = secondarySectionCount
        self.panelHeaderTitle = panelHeaderTitle
        self.panelHeaderCount = panelHeaderCount
        self.commandPaletteHelpText = commandPaletteHelpText
        self.commandPaletteQuoteText = commandPaletteQuoteText
        emptyStateLayer.string = commandPaletteHelpText
        emptyStateLayer.isHidden = commandPaletteHelpText == nil
        commandPaletteQuoteLayer.string = commandPaletteQuoteText
        commandPaletteQuoteLayer.isHidden = commandPaletteQuoteText == nil
        sectionLabelLayer.string = discoverySectionLabel()
        updatePanelHeader(isLoading: panelHeaderLoading)

        for removedName in removedPackageNames {
            guard let layer = nodeLayers.removeValue(forKey: removedName) else { continue }
            if shouldAnimateTransition {
                freezePresentationState(for: layer)
                animate(layer: layer, keyPath: "opacity", from: layer.opacity, to: 0, duration: 0.18)
                CATransaction.begin()
                CATransaction.setDisableActions(true)
                layer.opacity = 0
                CATransaction.commit()
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.20) {
                    layer.removeAllAnimations()
                    layer.hazardEffect.removeSmokeFromSuperlayer()
                    layer.removeFromSuperlayer()
                }
            } else {
                layer.removeAllAnimations()
                layer.hazardEffect.removeSmokeFromSuperlayer()
                layer.removeFromSuperlayer()
            }
        }

        for package in packages where nodeLayers[package.selectionID] == nil {
            let layer = PackageNodeLayer(package: package)
            layer.bounds = CGRect(
                x: 0,
                y: 0,
                width: Metrics.preferredNodeWidth,
                height: Metrics.nodeHeight
            )
            layer.opacity = 0
            layer.isEntering = true
            if shouldAnimateTransition,
               let blurFilter = CIFilter(name: "CIGaussianBlur") {
                blurFilter.setDefaults()
                blurFilter.setValue(2.0, forKey: kCIInputRadiusKey)
                layer.filters = [blurFilter]
            }
            contentLayer.addSublayer(layer)
            if let hazardSmokeHostLayer {
                layer.hazardEffect.installSmoke(in: hazardSmokeHostLayer)
            }
            nodeLayers[package.selectionID] = layer
        }

        for package in packages {
            guard let layer = nodeLayers[package.selectionID] else { continue }
            layer.package = package
            layer.baseOpacity = Float(0.55 + package.freshness * 0.35)
            // Security notices are computed from catalog state, so a package can
            // need a hazard refresh even when its stored presentation is equal.
            layer.updateText()
        }

        relayoutNodes(animated: shouldAnimateRelayout)

        for package in packages where insertedPackageNames.contains(package.selectionID) {
            guard let layer = nodeLayers[package.selectionID] else { continue }
            if shouldAnimateTransition {
                animateBlurOut(on: layer)
            } else {
                layer.filters = nil
            }
        }
    }

    private func discoverySectionLabel() -> NSAttributedString {
        let label = NSMutableAttributedString(
            attributedString: UIStyle.attributedMonoText(
                secondarySectionTitle,
                size: 10,
                color: UIStyle.dimText,
                weight: .medium,
                tracking: 0.8
            )
        )
        if let secondarySectionCount {
            label.append(
                UIStyle.attributedMonoText(
                    " \(secondarySectionCount)",
                    size: 10,
                    color: UIStyle.quietText,
                    weight: .medium,
                    tracking: 0.8
                )
            )
        }
        return label
    }

    func updateSearch(query: String) {
        searchQuery = query
        let lowered = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        for package in orderedPackages {
            guard let layer = nodeLayers[package.selectionID] else { continue }
            layer.matchesSearch = lowered.isEmpty || package.displayName.lowercased().contains(lowered)
            applyVisualState(to: layer, animated: true)
        }
    }

    func setEyebrowLoading(_ active: Bool) {
        panelHeaderLoading = active
        updatePanelHeader(isLoading: active)
    }

    private func updatePanelHeader(isLoading: Bool) {
        panelHeaderAnimator?.setText(panelHeaderTitle, animated: isLoading)
        if let panelHeaderCount {
            panelHeaderCountLayer.string = UIStyle.attributedMonoText(
                " \(panelHeaderCount)",
                size: 10,
                color: UIStyle.quietText,
                weight: .medium,
                tracking: 0.8
            )
            panelHeaderCountLayer.isHidden = false
        } else {
            panelHeaderCountLayer.string = nil
            panelHeaderCountLayer.isHidden = true
        }
    }

    private func panelHeaderTextWidth(_ text: String) -> CGFloat {
        let attributed = UIStyle.attributedMonoText(
            text,
            size: 10,
            color: UIStyle.text.withAlphaComponent(0.20),
            weight: .regular,
            tracking: 1.8
        )
        let bounds = attributed.boundingRect(
            with: CGSize(
                width: CGFloat.greatestFiniteMagnitude,
                height: Metrics.panelHeaderLabelHeight
            ),
            options: [.usesLineFragmentOrigin, .usesFontLeading]
        )
        return ceil(bounds.width)
    }

    func adjacentPackageName(
        from currentPackageName: String?,
        direction: NavigationDirection
    ) -> String? {
        let entries = visualEntries()
        guard !entries.isEmpty else { return nil }

        guard let currentPackageName,
              let current = entries.first(where: { $0.packageID == currentPackageName }) else {
            return entries.first?.packageID
        }

        switch direction {
        case .left:
            return entries
                .filter { $0.row == current.row && $0.column < current.column }
                .max(by: { $0.column < $1.column })?
                .packageID
        case .right:
            return entries
                .filter { $0.row == current.row && $0.column > current.column }
                .min(by: { $0.column < $1.column })?
                .packageID
        case .up:
            return nearestPackageName(
                in: entries,
                from: current,
                rowComparator: { $0 < current.row },
                bestRow: max
            )
        case .down:
            return nearestPackageName(
                in: entries,
                from: current,
                rowComparator: { $0 > current.row },
                bestRow: min
            )
        }
    }

    func frameForPackage(named packageName: String) -> CGRect? {
        nodeLayers[packageName]?.frame
    }

    override func mouseMoved(with event: NSEvent) {
        let point = convert(event.locationInWindow, from: nil)
        updateCursorTarget(point)
        updateHovered(name: hoveredPackageName(at: point))
    }

    override func mouseExited(with event: NSEvent) {
        updateCursorTarget(nil)
        updateHovered(name: nil)
    }

    override func mouseDown(with event: NSEvent) {
        let point = convert(event.locationInWindow, from: nil)
        guard let package = orderedPackages.first(where: { package in
            guard let layer = nodeLayers[package.selectionID] else { return false }
            return layer.frame.contains(point)
        }) else {
            return
        }
        selectedPackageID = package.selectionID
        updateCursorTarget(point)
        for package in orderedPackages {
            if let layer = nodeLayers[package.selectionID] {
                applyVisualState(to: layer, animated: true)
            }
        }
        updateOverlayGeometry()
        delegate?.packageFieldView(self, didSelect: package)
    }

    private func updateHovered(name: String?) {
        guard hoveredPackageID != name else { return }
        hoveredPackageID = name
        if let name,
           let layer = nodeLayers[name],
           layer.package.hasPlainTextSecretAlert {
            layer.hazardEffect.triggerSparkBurst(
                source: layer.package.plainTextSecretAlertSource
            )
        }
        for package in orderedPackages {
            if let layer = nodeLayers[package.selectionID] {
                applyVisualState(to: layer, animated: true)
            }
        }
        updateOverlayGeometry()
    }

    private func hoveredPackageName(at point: CGPoint) -> String? {
        var closestName: String?
        var closestDistance = CGFloat.greatestFiniteMagnitude

        for package in orderedPackages {
            guard let layer = nodeLayers[package.selectionID] else { continue }
            let distance = squaredDistance(from: point, to: layer.frame)
            if distance < closestDistance {
                closestDistance = distance
                closestName = package.selectionID
            }
        }

        let threshold = Metrics.hoverCaptureDistance * Metrics.hoverCaptureDistance
        guard closestDistance <= threshold else { return nil }
        return closestName
    }

    private func squaredDistance(from point: CGPoint, to rect: CGRect) -> CGFloat {
        let dx: CGFloat
        if point.x < rect.minX {
            dx = rect.minX - point.x
        } else if point.x > rect.maxX {
            dx = point.x - rect.maxX
        } else {
            dx = 0
        }

        let dy: CGFloat
        if point.y < rect.minY {
            dy = rect.minY - point.y
        } else if point.y > rect.maxY {
            dy = point.y - rect.maxY
        } else {
            dy = 0
        }

        return dx * dx + dy * dy
    }

    private func relayoutNodes(animated: Bool) {
        let usableWidth = max(
            bounds.width - Metrics.leadingInset - Metrics.trailingInset,
            Metrics.minNodeWidth
        )
        let layout = makeGridLayout(usableWidth: usableWidth)
        let columns = layout.columns
        let dividerIndex = orderedPackages.firstIndex(where: { $0.item.isAvailable })
        let installedCount = dividerIndex ?? orderedPackages.count
        let dividerVisible = shouldShowSecondarySectionLabel(
            dividerIndex: dividerIndex,
            installedCount: installedCount
        )
        let installedRows = (installedCount + columns - 1) / columns
        let rowStride = Metrics.nodeHeight + Metrics.rowGap
        let nodeTopInset = commandPaletteHelpText == nil
            ? Metrics.topInset
            : Metrics.helpTopInset
                + Metrics.helpHeight
                + Metrics.helpBottomGap
                + Metrics.commandSectionLabelHeight

        for (index, package) in orderedPackages.enumerated() {
            guard let layer = nodeLayers[package.selectionID] else { continue }
            let visualIndex: Int
            let sectionOffset: CGFloat
            if let dividerIndex, index >= dividerIndex, dividerVisible {
                visualIndex = index - dividerIndex
                sectionOffset = Metrics.sectionGap
                    + CGFloat(installedRows) * rowStride
            } else {
                visualIndex = index
                sectionOffset = 0
            }

            let column = visualIndex % columns
            let row = visualIndex / columns
            layer.bounds = CGRect(x: 0, y: 0, width: layout.nodeWidth, height: Metrics.nodeHeight)
            let x = Metrics.leadingInset
                + CGFloat(column) * (layout.nodeWidth + Metrics.horizontalGap)
            let y = nodeTopInset
                + CGFloat(row) * rowStride
                + sectionOffset
            let position = CGPoint(x: x + layer.bounds.width / 2, y: y + layer.bounds.height / 2)
            layer.basePosition = position

            if animated, layer.isEntering {
                CATransaction.begin()
                CATransaction.setDisableActions(true)
                layer.position = CGPoint(
                    x: position.x,
                    y: position.y + Motion.insertionYOffset
                )
                CATransaction.commit()
            } else if !animated {
                layer.position = positionForCurrentState(of: layer)
            }
            applyVisualState(to: layer, animated: animated)
        }

        CATransaction.begin()
        CATransaction.setDisableActions(true)
        if commandPaletteHelpText != nil {
            sectionDividerLayer.isHidden = true
            sectionLabelLayer.isHidden = false
            sectionLabelLayer.string = UIStyle.sectionHeaderText("Available Commands")
            sectionLabelLayer.frame = CGRect(
                x: Metrics.leadingInset,
                y: nodeTopInset - Metrics.commandSectionLabelHeight - 6,
                width: max(bounds.width - Metrics.leadingInset - Metrics.trailingInset, 0),
                height: Metrics.commandSectionLabelHeight
            )
        } else {
            sectionDividerLayer.isHidden = !dividerVisible
            sectionLabelLayer.isHidden = !dividerVisible
        }
        if dividerVisible && commandPaletteHelpText == nil {
            let sectionBreakY = nodeTopInset
                + CGFloat(installedRows) * rowStride
            let dividerY = sectionBreakY + Metrics.dividerOffset
            sectionDividerLayer.frame = CGRect(
                x: Metrics.leadingInset,
                y: dividerY,
                width: max(bounds.width - Metrics.leadingInset - Metrics.trailingInset, 0),
                height: 1
            )
            sectionLabelLayer.frame = CGRect(
                x: Metrics.leadingInset,
                y: sectionBreakY + Metrics.sectionLabelOffset,
                width: 140,
                height: 16
            )
        }
        commandPaletteQuoteLayer.frame = commandPaletteQuoteFrame()
        CATransaction.commit()

        updateOverlayGeometry()
    }

    private func commandPaletteQuoteFrame() -> CGRect {
        guard commandPaletteQuoteText != nil else {
            return CGRect(
                x: Metrics.leadingInset,
                y: Metrics.topInset,
                width: max(bounds.width - Metrics.leadingInset - Metrics.trailingInset, 0),
                height: Metrics.quoteHeight
            )
        }

        let usableWidth = max(
            bounds.width - Metrics.leadingInset - Metrics.trailingInset,
            Metrics.minNodeWidth
        )
        let layout = makeGridLayout(usableWidth: usableWidth)
        let rows = orderedPackages.isEmpty
            ? 0
            : (orderedPackages.count + layout.columns - 1) / layout.columns
        let nodeTopInset = Metrics.helpTopInset
            + Metrics.helpHeight
            + Metrics.helpBottomGap
            + Metrics.commandSectionLabelHeight
        let rowsHeight = CGFloat(rows) * (Metrics.nodeHeight + Metrics.rowGap)
        return CGRect(
            x: Metrics.leadingInset,
            y: nodeTopInset + rowsHeight + Metrics.quoteTopGap,
            width: max(bounds.width - Metrics.leadingInset - Metrics.trailingInset, 0),
            height: Metrics.quoteHeight
        )
    }

    private func visualEntries() -> [VisualEntry] {
        guard !orderedPackages.isEmpty else { return [] }
        let usableWidth = max(
            bounds.width - Metrics.leadingInset - Metrics.trailingInset,
            Metrics.minNodeWidth
        )
        let layout = makeGridLayout(usableWidth: usableWidth)
        let columns = layout.columns
        let dividerIndex = orderedPackages.firstIndex(where: { $0.item.isAvailable })
        let installedCount = dividerIndex ?? orderedPackages.count
        let installedRows = installedCount > 0
            ? (installedCount + columns - 1) / columns
            : 0
        let dividerVisible = shouldShowSecondarySectionLabel(
            dividerIndex: dividerIndex,
            installedCount: installedCount
        )
        let availableRowOffset = dividerVisible ? installedRows + 1 : 0

        return orderedPackages.enumerated().map { index, package in
            let visualIndex: Int
            let rowOffset: Int
            if let dividerIndex, index >= dividerIndex, dividerVisible {
                visualIndex = index - dividerIndex
                rowOffset = availableRowOffset
            } else {
                visualIndex = index
                rowOffset = 0
            }

            return VisualEntry(
                packageID: package.selectionID,
                row: rowOffset + (visualIndex / columns),
                column: visualIndex % columns
            )
        }
    }

    private func shouldShowSecondarySectionLabel(
        dividerIndex: Int?,
        installedCount: Int
    ) -> Bool {
        guard dividerIndex != nil else {
            return false
        }
        return installedCount > 0 || secondarySectionTitle == "RECOMMENDATIONS"
    }

    private func nearestPackageName(
        in entries: [VisualEntry],
        from current: VisualEntry,
        rowComparator: (Int) -> Bool,
        bestRow: (Int, Int) -> Int
    ) -> String? {
        let candidateRows = Set(entries.lazy.map(\.row).filter(rowComparator))
        guard let targetRow = candidateRows.reduce(Int?.none, { partial, row in
            guard let partial else { return row }
            return bestRow(partial, row)
        }) else {
            return nil
        }

        return entries
            .filter { $0.row == targetRow }
            .min(by: { lhs, rhs in
                let lhsDistance = abs(lhs.column - current.column)
                let rhsDistance = abs(rhs.column - current.column)
                if lhsDistance == rhsDistance {
                    return lhs.column < rhs.column
                }
                return lhsDistance < rhsDistance
            })?
            .packageID
    }

    private func makeGridLayout(usableWidth: CGFloat) -> (columns: Int, nodeWidth: CGFloat) {
        var columns = max(
            Int((usableWidth + Metrics.horizontalGap) / (Metrics.preferredNodeWidth + Metrics.horizontalGap)),
            1
        )

        while columns > 1 {
            let candidateWidth = floor(
                (usableWidth - CGFloat(columns - 1) * Metrics.horizontalGap) / CGFloat(columns)
            )
            if candidateWidth <= Metrics.maxNodeWidth || candidateWidth < Metrics.minNodeWidth {
                break
            }
            let nextWidth = floor(
                (usableWidth - CGFloat(columns) * Metrics.horizontalGap) / CGFloat(columns + 1)
            )
            if nextWidth < Metrics.minNodeWidth {
                break
            }
            columns += 1
        }

        let nodeWidth = max(
            Metrics.minNodeWidth,
            min(
                Metrics.maxNodeWidth,
                floor((usableWidth - CGFloat(columns - 1) * Metrics.horizontalGap) / CGFloat(columns))
            )
        )

        return (columns, nodeWidth)
    }

    private func applyVisualState(to layer: PackageNodeLayer, animated: Bool) {
        layer.isHovered = hoveredPackageID == layer.package.selectionID
        layer.isSelectedNode = selectedPackageID == layer.package.selectionID

        let targetOpacity: Float
        let targetTitleColor: NSColor
        let targetVersionColor: NSColor
        let targetDescriptionColor: NSColor

        if layer.matchesSearch {
            targetOpacity = layer.isSelectedNode ? 1.0 : max(layer.baseOpacity, 0.68)
        } else {
            targetOpacity = 0.1
        }

        let accentColor = layer.package.isHomebrewInstall
            ? UIStyle.warning
            : UIStyle.text
        if layer.isSelectedNode {
            targetTitleColor = accentColor.withAlphaComponent(0.98)
            targetVersionColor = accentColor.withAlphaComponent(0.72)
            targetDescriptionColor = UIStyle.text.withAlphaComponent(0.64)
        } else if layer.isHovered {
            targetTitleColor = accentColor.withAlphaComponent(0.93)
            targetVersionColor = accentColor.withAlphaComponent(0.62)
            targetDescriptionColor = UIStyle.text.withAlphaComponent(0.56)
        } else {
            targetTitleColor = accentColor.withAlphaComponent(0.88)
            targetVersionColor = accentColor.withAlphaComponent(0.58)
            targetDescriptionColor = UIStyle.text.withAlphaComponent(0.50)
        }

        let position = positionForCurrentState(of: layer)

        if animated {
            animate(
                layer: layer,
                keyPath: "opacity",
                from: layer.opacity,
                to: targetOpacity,
                duration: Timing.standard
            )
            animatePosition(layer: layer, to: position)
        }

        CATransaction.begin()
        CATransaction.setDisableActions(true)
        layer.opacity = targetOpacity
        layer.transform = CATransform3DIdentity
        layer.shadowOpacity = 0
        layer.shadowRadius = 0
        layer.position = position
        layer.isEntering = false
        let colorsChanged = layer.titleColor != targetTitleColor
            || layer.versionColor != targetVersionColor
            || layer.descriptionColor != targetDescriptionColor
        layer.titleColor = targetTitleColor
        layer.versionColor = targetVersionColor
        layer.descriptionColor = targetDescriptionColor
        if colorsChanged {
            layer.updateText()
        }
        CATransaction.commit()
    }

    private func positionForCurrentState(of layer: PackageNodeLayer) -> CGPoint {
        layer.basePosition
    }

    private func animateBlurOut(on layer: PackageNodeLayer) {
        guard let filter = layer.filters?.first as? CIFilter else {
            return
        }

        let animation = CABasicAnimation(keyPath: "filters.gaussianBlur.inputRadius")
        animation.fromValue = 2.0
        animation.toValue = 0.0
        animation.duration = Timing.standard
        animation.timingFunction = CAMediaTimingFunction(name: .easeOut)
        layer.add(animation, forKey: "filters.gaussianBlur.inputRadius")

        CATransaction.begin()
        CATransaction.setDisableActions(true)
        filter.setValue(0.0, forKey: kCIInputRadiusKey)
        layer.filters = [filter]
        CATransaction.commit()
    }

    private func animatePosition(layer: CALayer, to target: CGPoint) {
        animate(
            layer: layer,
            keyPath: "position",
            from: layer.presentation()?.position ?? layer.position,
            to: target,
            duration: Timing.standard
        )
    }

    private func animate(layer: CALayer, keyPath: String, from: Any, to: Any, duration: CFTimeInterval) {
        let animation = CABasicAnimation(keyPath: keyPath)
        animation.fromValue = from
        animation.toValue = to
        animation.duration = duration
        animation.timingFunction = CAMediaTimingFunction(name: .easeOut)
        layer.add(animation, forKey: keyPath)
    }

    private func freezePresentationState(for layer: CALayer) {
        guard let presentation = layer.presentation() else {
            layer.removeAllAnimations()
            return
        }

        CATransaction.begin()
        CATransaction.setDisableActions(true)
        layer.position = presentation.position
        layer.opacity = presentation.opacity
        layer.transform = presentation.transform
        layer.shadowOpacity = presentation.shadowOpacity
        layer.shadowRadius = presentation.shadowRadius
        CATransaction.commit()
        layer.removeAllAnimations()
    }

    private func updateCursorTarget(_ point: CGPoint?) {
        cursorTargetPoint = point
        if cursorDisplayPoint == nil {
            cursorDisplayPoint = point
        }
        ensureCursorAnimationTimer()
    }

    private func ensureCursorAnimationTimer() {
        guard cursorAnimationTimer == nil else { return }
        let timer = Timer(timeInterval: 1.0 / 60.0, repeats: true) { [weak self] _ in
            self?.stepCursorInfluence()
        }
        RunLoop.main.add(timer, forMode: .common)
        cursorAnimationTimer = timer
    }

    private func stepCursorInfluence() {
        var keepAnimating = false

        if let target = cursorTargetPoint {
            if let display = cursorDisplayPoint {
                let next = CGPoint(
                    x: display.x + (target.x - display.x) * Metrics.cursorLerpFactor,
                    y: display.y + (target.y - display.y) * Metrics.cursorLerpFactor
                )
                cursorDisplayPoint = next
                keepAnimating = distance(from: next, to: target) > 0.5
            } else {
                cursorDisplayPoint = target
            }
            let nextInfluence = min(cursorInfluenceStrength + Metrics.cursorFadeInStep, 1)
            keepAnimating = keepAnimating || abs(nextInfluence - cursorInfluenceStrength) > 0.001
            cursorInfluenceStrength = nextInfluence
        } else {
            let nextInfluence = max(cursorInfluenceStrength - Metrics.cursorFadeOutStep, 0)
            keepAnimating = nextInfluence > 0.001
            cursorInfluenceStrength = nextInfluence
            if !keepAnimating {
                cursorDisplayPoint = nil
            }
        }

        latentGridLayer.updateCursorFocus(
            point: cursorDisplayPoint,
            influenceStrength: cursorInfluenceStrength
        )

        if !keepAnimating {
            cursorAnimationTimer?.invalidate()
            cursorAnimationTimer = nil
        }
    }

    private func distance(from lhs: CGPoint, to rhs: CGPoint) -> CGFloat {
        let dx = lhs.x - rhs.x
        let dy = lhs.y - rhs.y
        return sqrt(dx * dx + dy * dy)
    }

    private func updateOverlayGeometry() {
        let entries = visualEntries()
        var columnAnchors: [Int: CGFloat] = [:]
        var rowAnchors: [Int: CGFloat] = [:]

        for entry in entries {
            guard let layer = nodeLayers[entry.packageID] else { continue }
            columnAnchors[entry.column] = layer.frame.minX
            rowAnchors[entry.row] = rowBaseline(for: layer.frame)
        }

        let sortedColumnAnchors = columnAnchors
            .sorted(by: { $0.key < $1.key })
            .map(\.value)
        let sortedRowAnchors = rowAnchors
            .sorted(by: { $0.key < $1.key })
            .map(\.value)

        let selectedFrame = selectedPackageID.flatMap { packageFrameForFocus(named: $0) }
        let hoveredFrame = hoveredPackageID.flatMap { packageFrameForFocus(named: $0) }
        let hoveredAnchorFrame = hoveredPackageID.flatMap { nodeLayers[$0]?.frame }
        let selectedAnchorFrame = selectedPackageID.flatMap { nodeLayers[$0]?.frame }
        let hoveredIsHazard = hoveredPackageID
            .flatMap { nodeLayers[$0]?.package.hasPlainTextSecretAlert }
            ?? false
        let selectedIsHazard = selectedPackageID
            .flatMap { nodeLayers[$0]?.package.hasPlainTextSecretAlert }
            ?? false

        latentGridLayer.updateGrid(
            verticalAnchors: sortedColumnAnchors,
            horizontalAnchors: sortedRowAnchors,
            hoverVerticalAnchor: hoveredAnchorFrame?.minX,
            hoverHorizontalAnchor: hoveredAnchorFrame.map(rowBaseline(for:)),
            selectedVerticalAnchor: selectedAnchorFrame?.minX,
            selectedHorizontalAnchor: selectedAnchorFrame.map(rowBaseline(for:)),
            hoverIsHazard: hoveredIsHazard,
            selectedIsHazard: selectedIsHazard
        )
        latentGridLayer.updateCursorFocus(
            point: cursorDisplayPoint,
            influenceStrength: cursorInfluenceStrength
        )
        cornerBracketLayer.update(
            hoverFrame: hoveredFrame,
            selectedFrame: selectedFrame,
            hoverIsHazard: hoveredIsHazard,
            selectedIsHazard: selectedIsHazard
        )
        updateHazardSmokeBrackets(
            hoverFrame: hoveredFrame,
            selectedFrame: selectedFrame,
            hoverIsHazard: hoveredIsHazard,
            selectedIsHazard: selectedIsHazard
        )
        updateHazardSmokeGeometry()
    }

    private func updateHazardSmokeBrackets(
        hoverFrame: CGRect?,
        selectedFrame: CGRect?,
        hoverIsHazard: Bool,
        selectedIsHazard: Bool
    ) {
        guard let hazardSmokeHostLayer,
              let hazardSmokeCoordinateView else {
            return
        }
        hazardSmokeBracketLayer.frame = hazardSmokeHostLayer.bounds
        hazardSmokeBracketLayer.update(
            hoverFrame: hoverFrame.map { convert($0, to: hazardSmokeCoordinateView) },
            selectedFrame: selectedFrame.map { convert($0, to: hazardSmokeCoordinateView) },
            hoverIsHazard: hoverIsHazard,
            selectedIsHazard: selectedIsHazard
        )
    }

    private func updateHazardSmokeBracketsForCurrentState() {
        let hoveredFrame = hoveredPackageID.flatMap { packageFrameForFocus(named: $0) }
        let selectedFrame = selectedPackageID.flatMap { packageFrameForFocus(named: $0) }
        let hoveredIsHazard = hoveredPackageID
            .flatMap { nodeLayers[$0]?.package.hasPlainTextSecretAlert }
            ?? false
        let selectedIsHazard = selectedPackageID
            .flatMap { nodeLayers[$0]?.package.hasPlainTextSecretAlert }
            ?? false
        updateHazardSmokeBrackets(
            hoverFrame: hoveredFrame,
            selectedFrame: selectedFrame,
            hoverIsHazard: hoveredIsHazard,
            selectedIsHazard: selectedIsHazard
        )
    }

    private func updateHazardSmokeGeometry() {
        guard let hazardSmokeHostLayer,
              let hazardSmokeCoordinateView else {
            return
        }
        for package in orderedPackages {
            guard let layer = nodeLayers[package.selectionID] else { continue }
            let titleFrame = layer.hazardTitleFrameInParent().map {
                convert($0, to: hazardSmokeCoordinateView)
            }
            let textFrame = convert(
                layer.textFrameInParent(),
                to: hazardSmokeCoordinateView
            )
            layer.hazardEffect.layoutSmoke(
                in: hazardSmokeHostLayer.bounds,
                sourceFrame: titleFrame
            )
            layer.hazardEffect.layoutProtectedSource(
                frame: textFrame,
                text: layer.renderedText,
                isActive: layer.package.hasPlainTextSecretAlert
            )
        }
    }

    private func packageFrameForFocus(named packageName: String) -> CGRect? {
        guard let layer = nodeLayers[packageName] else { return nil }
        let focusBounds = layer.focusBounds()
        return focusBounds.offsetBy(
            dx: layer.frame.minX,
            dy: layer.frame.minY
        )
    }

    private func rowBaseline(for frame: CGRect) -> CGFloat {
        frame.minY + floor(frame.height * Metrics.rowBaselineRatio)
    }
}
