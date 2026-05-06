import AppKit
import CoreText
import QuartzCore

enum UIStyle {
    struct ControlChrome {
        let topBackgroundColor: NSColor
        let bottomBackgroundColor: NSColor
        let borderColor: NSColor
        let contentColor: NSColor
        let topInnerStrokeColor: NSColor?
        let bottomInnerStrokeColor: NSColor?
    }

    static let background = NSColor(calibratedWhite: 0.055, alpha: 1.0)
    static let surface = NSColor(calibratedWhite: 0.07, alpha: 1.0)
    static let text = NSColor(calibratedWhite: 0.92, alpha: 1.0)
    static let dimText = NSColor(calibratedWhite: 0.92, alpha: 0.58)
    static let quietText = NSColor(calibratedWhite: 0.92, alpha: 0.40)
    static let accent = NSColor(calibratedRed: 0.42, green: 0.86, blue: 0.66, alpha: 1.0)
    static let accentShadow = NSColor(calibratedRed: 0.42, green: 0.86, blue: 0.66, alpha: 0.16)
    static let warning = NSColor(calibratedRed: 0.95, green: 0.72, blue: 0.20, alpha: 1.0)
    static let warningShadow = NSColor(calibratedRed: 0.95, green: 0.72, blue: 0.20, alpha: 0.16)
    static let danger = NSColor(calibratedRed: 0.92, green: 0.31, blue: 0.35, alpha: 1.0)
    static let separator = NSColor(calibratedWhite: 1.0, alpha: 0.048)
    static let spine = NSColor(calibratedWhite: 1.0, alpha: 0.07)
    static let webOverlay = NSColor(calibratedWhite: 0.0, alpha: 0.18)
    static let controlCornerRadius: CGFloat = 2

    private static let controlGradientLayerName = "UIStyleControlGradient"
    private static let controlTopStrokeLayerName = "UIStyleControlTopStroke"
    private static let controlBottomStrokeLayerName = "UIStyleControlBottomStroke"

    static func monoFont(size: CGFloat, weight: NSFont.Weight = .regular) -> NSFont {
        if let mono = NSFont(name: "SFMono-Regular", size: size) {
            return mono
        }
        return NSFont.monospacedSystemFont(ofSize: size, weight: weight)
    }

    static func attributedMonoText(
        _ string: String,
        size: CGFloat,
        color: NSColor,
        weight: NSFont.Weight = .regular,
        tracking: CGFloat = 0.2
    ) -> NSAttributedString {
        let font = monoFont(size: size, weight: weight)
        return NSAttributedString(
            string: string,
            attributes: [
                .font: font,
                .foregroundColor: color,
                .kern: tracking,
                .paragraphStyle: wrapParagraphStyle()
            ]
        )
    }

    static func wrapParagraphStyle() -> NSParagraphStyle {
        wrapParagraphStyle(lineHeightMultiple: 1.0)
    }

    static func wrapParagraphStyle(
        lineHeightMultiple: CGFloat = 1.0,
        paragraphSpacing: CGFloat = 0
    ) -> NSParagraphStyle {
        let style = NSMutableParagraphStyle()
        style.lineBreakMode = .byCharWrapping
        style.hyphenationFactor = 0
        style.lineHeightMultiple = lineHeightMultiple
        style.paragraphSpacing = paragraphSpacing
        return style
    }

    static func sectionHeaderText(_ string: String) -> NSAttributedString {
        attributedMonoText(
            string.uppercased(),
            size: 10,
            color: quietText,
            weight: .regular,
            tracking: 1.2
        )
    }

    static func applyControlChrome(to layer: CALayer?, chrome: ControlChrome) {
        guard let layer else { return }
        layer.cornerRadius = controlCornerRadius
        layer.borderWidth = 1
        layer.borderColor = chrome.borderColor.cgColor
        layer.masksToBounds = true
        layer.backgroundColor = chrome.bottomBackgroundColor.cgColor

        let gradientLayer = ensureControlGradientLayer(
            named: controlGradientLayerName,
            in: layer
        )
        gradientLayer.colors = [
            chrome.topBackgroundColor.cgColor,
            chrome.bottomBackgroundColor.cgColor
        ]
        gradientLayer.startPoint = CGPoint(x: 0.5, y: 1.0)
        gradientLayer.endPoint = CGPoint(x: 0.5, y: 0.0)
        gradientLayer.cornerRadius = controlCornerRadius

        configureControlStrokeLayer(
            named: controlTopStrokeLayerName,
            color: chrome.topInnerStrokeColor,
            in: layer
        )
        configureControlStrokeLayer(
            named: controlBottomStrokeLayerName,
            color: chrome.bottomInnerStrokeColor,
            in: layer
        )
        layoutControlChrome(in: layer)
    }

    static func layoutControlChrome(in layer: CALayer?) {
        guard let layer else { return }
        layer.sublayers?.forEach { sublayer in
            switch sublayer.name {
            case controlGradientLayerName:
                sublayer.frame = layer.bounds
            case controlTopStrokeLayerName:
                sublayer.frame = CGRect(
                    x: 0,
                    y: max(layer.bounds.height - 1, 0),
                    width: layer.bounds.width,
                    height: 1
                )
            case controlBottomStrokeLayerName:
                sublayer.frame = CGRect(
                    x: 0,
                    y: 0,
                    width: layer.bounds.width,
                    height: 1
                )
            default:
                break
            }
        }
    }

    private static func configureControlStrokeLayer(
        named name: String,
        color: NSColor?,
        in layer: CALayer
    ) {
        if let color {
            let strokeLayer = ensureControlSublayer(named: name, in: layer)
            strokeLayer.backgroundColor = color.cgColor
            strokeLayer.isHidden = false
        } else {
            ensureControlSublayer(named: name, in: layer).isHidden = true
        }
    }

    private static func ensureControlSublayer(named name: String, in layer: CALayer) -> CALayer {
        if let existing = layer.sublayers?.first(where: { $0.name == name }) {
            return existing
        }
        let sublayer = CALayer()
        sublayer.name = name
        layer.addSublayer(sublayer)
        return sublayer
    }

    private static func ensureControlGradientLayer(
        named name: String,
        in layer: CALayer
    ) -> CAGradientLayer {
        let matchingLayers = layer.sublayers?.filter { $0.name == name } ?? []

        if let gradientLayer = matchingLayers.first(where: { $0 is CAGradientLayer }) as? CAGradientLayer {
            for duplicate in matchingLayers where duplicate !== gradientLayer {
                duplicate.removeFromSuperlayer()
            }
            return gradientLayer
        }

        for duplicate in matchingLayers {
            duplicate.removeFromSuperlayer()
        }

        let gradientLayer = CAGradientLayer()
        gradientLayer.name = name
        layer.addSublayer(gradientLayer)
        return gradientLayer
    }
}
