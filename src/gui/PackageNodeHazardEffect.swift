import AppKit
import QuartzCore

final class PackageNodeHazardEffect {
    private struct Metrics {
        static let sparkEmitterWidth: CGFloat = 1
        static let sparkEmitterHeight: CGFloat = 1
        static let sparkEmitterCenterXOffset: CGFloat = 0
        static let sparkEmitterCenterYOffset: CGFloat = -5.0
        static let smokeEmitterWidth: CGFloat = 34
        static let smokeEmitterHeight: CGFloat = 4
        static let smokeEmitterYOffset: CGFloat = -3
    }

    let sparkEmitterLayer = CAEmitterLayer()
    private let sparkBurstEmitterLayer = CAEmitterLayer()
    private let smokeEmitterLayer = CAEmitterLayer()
    private let sourceTextLayer = CATextLayer()
    private var renderedSource: PackageSecurityNotice.Source?
    private var sparkBurstGeneration = 0

    init() {
        sparkEmitterLayer.emitterShape = .rectangle
        sparkEmitterLayer.renderMode = .additive
        sparkEmitterLayer.birthRate = 0
        sparkEmitterLayer.isHidden = true
        sparkEmitterLayer.emitterCells = []

        sparkBurstEmitterLayer.emitterShape = .rectangle
        sparkBurstEmitterLayer.renderMode = .additive
        sparkBurstEmitterLayer.birthRate = 0
        sparkBurstEmitterLayer.isHidden = true
        sparkBurstEmitterLayer.emitterCells = []

        smokeEmitterLayer.emitterShape = .rectangle
        smokeEmitterLayer.renderMode = .unordered
        smokeEmitterLayer.birthRate = 0
        smokeEmitterLayer.isHidden = true
        smokeEmitterLayer.emitterCells = []
        smokeEmitterLayer.zPosition = 1

        sourceTextLayer.contentsScale = NSScreen.main?.backingScaleFactor ?? 2
        sourceTextLayer.alignmentMode = .left
        sourceTextLayer.isWrapped = true
        sourceTextLayer.isHidden = true
        sourceTextLayer.zPosition = 2
    }

    func install(in layer: CALayer) {
        layer.addSublayer(sparkEmitterLayer)
        layer.addSublayer(sparkBurstEmitterLayer)
    }

    func installSmoke(in layer: CALayer) {
        layer.addSublayer(smokeEmitterLayer)
        layer.addSublayer(sourceTextLayer)
    }

    func removeSmokeFromSuperlayer() {
        smokeEmitterLayer.removeFromSuperlayer()
        sourceTextLayer.removeFromSuperlayer()
    }

    func layout(in bounds: CGRect, symbolFrame: CGRect?) {
        sparkEmitterLayer.frame = bounds
        sparkBurstEmitterLayer.frame = bounds
        guard let symbolFrame else {
            return
        }
        let emitterPosition = CGPoint(
            x: symbolFrame.midX + Metrics.sparkEmitterCenterXOffset,
            y: symbolFrame.midY + Metrics.sparkEmitterCenterYOffset
        )
        let emitterSize = CGSize(
            width: Metrics.sparkEmitterWidth,
            height: Metrics.sparkEmitterHeight
        )
        sparkEmitterLayer.emitterPosition = emitterPosition
        sparkEmitterLayer.emitterSize = emitterSize
        sparkBurstEmitterLayer.emitterPosition = emitterPosition
        sparkBurstEmitterLayer.emitterSize = emitterSize
    }

    func layoutSmoke(
        in bounds: CGRect,
        sourceFrame: CGRect?
    ) {
        smokeEmitterLayer.frame = bounds
        guard let sourceFrame else {
            return
        }
        smokeEmitterLayer.emitterPosition = CGPoint(
            x: sourceFrame.midX,
            y: sourceFrame.midY + Metrics.smokeEmitterYOffset
        )
        smokeEmitterLayer.emitterSize = CGSize(
            width: Metrics.smokeEmitterWidth,
            height: Metrics.smokeEmitterHeight
        )
    }

    func layoutProtectedSource(
        frame: CGRect?,
        text: Any?,
        isActive: Bool
    ) {
        guard isActive,
              let frame,
              let text else {
            sourceTextLayer.isHidden = true
            sourceTextLayer.string = nil
            return
        }

        sourceTextLayer.isHidden = false
        sourceTextLayer.frame = frame
        sourceTextLayer.string = text
    }

    func update(source: PackageSecurityNotice.Source?) {
        let isActive = source != nil
        sparkEmitterLayer.isHidden = !isActive
        sparkBurstEmitterLayer.isHidden = !isActive
        smokeEmitterLayer.isHidden = !isActive
        if !isActive {
            renderedSource = nil
            sparkEmitterLayer.birthRate = 0
            sparkBurstEmitterLayer.birthRate = 0
            smokeEmitterLayer.birthRate = 0
            sparkEmitterLayer.emitterCells = []
            sparkBurstEmitterLayer.emitterCells = []
            smokeEmitterLayer.emitterCells = []
            sparkEmitterLayer.removeAnimation(forKey: "hazardSparkPulse")
            sparkBurstEmitterLayer.removeAnimation(forKey: "hazardSparkBurst")
            smokeEmitterLayer.removeAnimation(forKey: "hazardSmokeBillow")
            return
        }

        if renderedSource != source {
            renderedSource = source
            sparkEmitterLayer.emitterCells = Self.makeSparkCells(for: source)
            sparkBurstEmitterLayer.emitterCells = Self.makeSparkBurstCells(for: source)
            smokeEmitterLayer.emitterCells = Self.makeSmokeCells(for: source)
            sparkEmitterLayer.removeAnimation(forKey: "hazardSparkPulse")
            sparkBurstEmitterLayer.removeAnimation(forKey: "hazardSparkBurst")
            smokeEmitterLayer.removeAnimation(forKey: "hazardSmokeBillow")
        }

        sparkEmitterLayer.birthRate = 1
        smokeEmitterLayer.birthRate = 1

        if sparkEmitterLayer.animation(forKey: "hazardSparkPulse") == nil {
            let pulse = CAKeyframeAnimation(keyPath: "birthRate")
            pulse.values = [0, 0, 7, 0, 0, 11, 2, 0, 8, 0, 0, 13, 3, 0]
            pulse.keyTimes = [
                0, 0.10, 0.14, 0.18, 0.34, 0.39, 0.43,
                0.48, 0.59, 0.64, 0.78, 0.84, 0.89, 1
            ]
            pulse.duration = 2.2
            pulse.repeatCount = .infinity
            pulse.timingFunctions = [
                CAMediaTimingFunction(name: .linear),
                CAMediaTimingFunction(name: .easeOut),
                CAMediaTimingFunction(name: .easeIn),
                CAMediaTimingFunction(name: .linear),
                CAMediaTimingFunction(name: .linear),
                CAMediaTimingFunction(name: .easeOut),
                CAMediaTimingFunction(name: .easeIn),
                CAMediaTimingFunction(name: .linear),
                CAMediaTimingFunction(name: .easeOut),
                CAMediaTimingFunction(name: .easeIn)
            ]
            sparkEmitterLayer.add(pulse, forKey: "hazardSparkPulse")
        }

        if smokeEmitterLayer.animation(forKey: "hazardSmokeBillow") == nil {
            let billow = CAKeyframeAnimation(keyPath: "birthRate")
            billow.values = [1.6, 2.8, 2.2, 3.7, 2.5, 4.1, 2.9, 3.4]
            billow.keyTimes = [0, 0.12, 0.25, 0.42, 0.58, 0.76, 0.90, 1]
            billow.duration = 3.4
            billow.repeatCount = .infinity
            billow.timingFunctions = [
                CAMediaTimingFunction(name: .easeInEaseOut),
                CAMediaTimingFunction(name: .easeInEaseOut),
                CAMediaTimingFunction(name: .easeInEaseOut),
                CAMediaTimingFunction(name: .easeInEaseOut),
                CAMediaTimingFunction(name: .easeInEaseOut),
                CAMediaTimingFunction(name: .easeInEaseOut),
                CAMediaTimingFunction(name: .easeInEaseOut)
            ]
            smokeEmitterLayer.add(billow, forKey: "hazardSmokeBillow")
        }
    }

    func triggerSparkBurst(source: PackageSecurityNotice.Source?) {
        guard source != nil else { return }
        sparkBurstEmitterLayer.isHidden = false
        sparkBurstEmitterLayer.emitterCells = Self.makeSparkBurstCells(for: source)
        sparkBurstEmitterLayer.removeAnimation(forKey: "hazardSparkBurst")
        sparkBurstGeneration += 1
        let generation = sparkBurstGeneration

        CATransaction.begin()
        CATransaction.setDisableActions(true)
        sparkBurstEmitterLayer.birthRate = 1
        CATransaction.commit()

        DispatchQueue.main.asyncAfter(deadline: .now() + 0.08) { [weak self] in
            guard let self, self.sparkBurstGeneration == generation else {
                return
            }
            CATransaction.begin()
            CATransaction.setDisableActions(true)
            self.sparkBurstEmitterLayer.birthRate = 0
            CATransaction.commit()
        }
    }

    private static func makeSparkCells(
        for source: PackageSecurityNotice.Source?
    ) -> [CAEmitterCell] {
        switch source {
        case .isotope:
            return [makeSparkCell(), makeTracerCell()]
        case .enrichmentManifest:
            return [makeSparkCell(), makeEmberCell()]
        case .none:
            return []
        }
    }

    private static func makeSparkBurstCells(
        for source: PackageSecurityNotice.Source?
    ) -> [CAEmitterCell] {
        switch source {
        case .isotope:
            return [
                makeSparkBurstCell(
                    birthRate: 140,
                    velocity: 185,
                    velocityRange: 230,
                    scale: 0.15,
                    alpha: 0.94
                ),
                makeSparkBurstCell(
                    birthRate: 62,
                    velocity: 275,
                    velocityRange: 150,
                    scale: 0.08,
                    alpha: 0.82
                )
            ]
        case .enrichmentManifest:
            return [
                makeSparkBurstCell(
                    birthRate: 115,
                    velocity: 155,
                    velocityRange: 210,
                    scale: 0.13,
                    alpha: 0.84
                ),
                makeSparkBurstCell(
                    birthRate: 49,
                    velocity: 220,
                    velocityRange: 140,
                    scale: 0.09,
                    alpha: 0.70
                )
            ]
        case .none:
            return []
        }
    }

    private static func makeSmokeCells(
        for source: PackageSecurityNotice.Source?
    ) -> [CAEmitterCell] {
        switch source {
        case .isotope:
            return [
                makeSmokeCell(
                    birthRate: 4.8,
                    velocity: 46,
                    drift: -12,
                    scale: 0.88,
                    alpha: 0.27
                ),
                makeSmokeCell(
                    birthRate: 3.4,
                    velocity: 34,
                    drift: 18,
                    scale: 1.12,
                    alpha: 0.25
                )
            ]
        case .enrichmentManifest:
            return [
                makeSmokeCell(
                    birthRate: 5.6,
                    velocity: 42,
                    drift: -10,
                    scale: 1.02,
                    alpha: 0.285
                ),
                makeSmokeCell(
                    birthRate: 3.8,
                    velocity: 31,
                    drift: 20,
                    scale: 1.24,
                    alpha: 0.265
                )
            ]
        case .none:
            return []
        }
    }

    private static func makeSmokeCell(
        birthRate: Float,
        velocity: CGFloat,
        drift: CGFloat,
        scale: CGFloat,
        alpha: CGFloat
    ) -> CAEmitterCell {
        let cell = CAEmitterCell()
        cell.contents = smokeParticleImage()
        cell.birthRate = birthRate
        cell.lifetime = 8.0
        cell.lifetimeRange = 2.0
        cell.velocity = velocity
        cell.velocityRange = 22
        cell.emissionLongitude = -.pi / 2
        cell.emissionRange = .pi * 0.18
        cell.yAcceleration = -16
        cell.xAcceleration = drift
        cell.scale = scale
        cell.scaleRange = 0.58
        cell.scaleSpeed = 0.22
        cell.alphaSpeed = -0.055
        cell.spin = 0.22
        cell.spinRange = 0.85
        cell.color = NSColor(
            calibratedRed: 0.11,
            green: 0.12,
            blue: 0.12,
            alpha: alpha
        ).cgColor
        return cell
    }

    private static func makeSparkCell() -> CAEmitterCell {
        let cell = CAEmitterCell()
        cell.contents = sparkParticleImage()
        cell.birthRate = 1
        cell.lifetime = 2.9
        cell.lifetimeRange = 1.5
        cell.velocity = 150
        cell.velocityRange = 170
        cell.emissionLongitude = .pi / 2
        cell.emissionRange = .pi * 0.54
        cell.yAcceleration = 90
        cell.xAcceleration = -8
        cell.scale = 0.16
        cell.scaleRange = 0.20
        cell.alphaSpeed = -0.50
        cell.spin = 3.0
        cell.spinRange = 5.2
        cell.color = NSColor(
            calibratedRed: 1.0,
            green: 0.20,
            blue: 0.08,
            alpha: 0.96
        ).cgColor
        return cell
    }

    private static func makeSparkBurstCell(
        birthRate: Float,
        velocity: CGFloat,
        velocityRange: CGFloat,
        scale: CGFloat,
        alpha: CGFloat
    ) -> CAEmitterCell {
        let cell = CAEmitterCell()
        cell.contents = sparkParticleImage()
        cell.birthRate = birthRate
        cell.lifetime = 0.70
        cell.lifetimeRange = 0.18
        cell.velocity = velocity
        cell.velocityRange = velocityRange
        cell.emissionLongitude = .pi / 2
        cell.emissionRange = .pi * 0.74
        cell.yAcceleration = 120
        cell.xAcceleration = -12
        cell.scale = scale
        cell.scaleRange = scale * 1.15
        cell.alphaSpeed = -1.45
        cell.spin = 4.5
        cell.spinRange = 7.0
        cell.color = NSColor(
            calibratedRed: 1.0,
            green: 0.26,
            blue: 0.06,
            alpha: alpha
        ).cgColor
        return cell
    }

    private static func makeEmberCell() -> CAEmitterCell {
        let cell = CAEmitterCell()
        cell.contents = sparkParticleImage()
        cell.birthRate = 0.22
        cell.lifetime = 1.7
        cell.lifetimeRange = 1.0
        cell.velocity = 55
        cell.velocityRange = 95
        cell.emissionLongitude = .pi / 2
        cell.emissionRange = .pi * 0.72
        cell.yAcceleration = 35
        cell.xAcceleration = 4
        cell.scale = 0.12
        cell.scaleRange = 0.14
        cell.alphaSpeed = -0.58
        cell.spin = 1.2
        cell.spinRange = 4.0
        cell.color = NSColor(
            calibratedRed: 1.0,
            green: 0.38,
            blue: 0.10,
            alpha: 0.62
        ).cgColor
        return cell
    }

    private static func makeTracerCell() -> CAEmitterCell {
        let cell = CAEmitterCell()
        cell.contents = sparkParticleImage()
        cell.birthRate = 0.34
        cell.lifetime = 1.15
        cell.lifetimeRange = 0.55
        cell.velocity = 210
        cell.velocityRange = 110
        cell.emissionLongitude = .pi / 2
        cell.emissionRange = .pi * 0.34
        cell.yAcceleration = 60
        cell.xAcceleration = -28
        cell.scale = 0.09
        cell.scaleRange = 0.08
        cell.alphaSpeed = -0.72
        cell.spin = 4.8
        cell.spinRange = 6.0
        cell.color = NSColor(
            calibratedRed: 0.95,
            green: 0.06,
            blue: 0.04,
            alpha: 0.82
        ).cgColor
        return cell
    }

    private static func sparkParticleImage() -> CGImage? {
        let size = NSSize(width: 5, height: 5)
        let image = NSImage(size: size)
        image.lockFocus()
        let rect = NSRect(origin: .zero, size: size)
        NSColor(
            calibratedRed: 1.0,
            green: 0.44,
            blue: 0.10,
            alpha: 0.96
        ).setFill()
        rect.fill()
        let innerRect = rect.insetBy(dx: 1.5, dy: 1.5)
        NSColor(
            calibratedRed: 1.0,
            green: 0.58,
            blue: 0.20,
            alpha: 0.96
        ).setFill()
        innerRect.fill()
        image.unlockFocus()
        return image.cgImage(
            forProposedRect: nil,
            context: nil,
            hints: nil
        )
    }

    private static func smokeParticleImage() -> CGImage? {
        let size = NSSize(width: 72, height: 72)
        let image = NSImage(size: size)
        image.lockFocus()
        NSColor.clear.setFill()
        NSRect(origin: .zero, size: size).fill()

        let lobes = [
            NSRect(x: 8, y: 18, width: 40, height: 36),
            NSRect(x: 24, y: 10, width: 38, height: 42),
            NSRect(x: 18, y: 26, width: 46, height: 34),
            NSRect(x: 6, y: 30, width: 34, height: 28)
        ]
        for (index, rect) in lobes.enumerated() {
            let alpha = 0.28 - CGFloat(index) * 0.035
            NSColor.white.withAlphaComponent(alpha).setFill()
            NSBezierPath(ovalIn: rect).fill()
        }

        NSColor.white.withAlphaComponent(0.12).setFill()
        NSBezierPath(
            ovalIn: NSRect(x: 3, y: 8, width: 66, height: 58)
        ).fill()

        image.unlockFocus()
        return image.cgImage(
            forProposedRect: nil,
            context: nil,
            hints: nil
        )
    }
}
