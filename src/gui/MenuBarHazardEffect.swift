import AppKit
import QuartzCore

final class MenuBarHazardEffect {
    private struct Metrics {
        static let overflowX: CGFloat = 26
        static let overflowY: CGFloat = 96
    }

    private let sparkEmitterLayer = CAEmitterLayer()
    private var isInstalled = false

    init() {
        sparkEmitterLayer.emitterShape = .rectangle
        sparkEmitterLayer.renderMode = .additive
        sparkEmitterLayer.birthRate = 0
        sparkEmitterLayer.isHidden = true
        sparkEmitterLayer.emitterCells = Self.makeSparkCells()
        sparkEmitterLayer.zPosition = 10
    }

    func install(in view: NSView) {
        view.wantsLayer = true
        guard let layer = view.layer else { return }
        layer.masksToBounds = false
        if sparkEmitterLayer.superlayer !== layer {
            sparkEmitterLayer.removeFromSuperlayer()
            layer.addSublayer(sparkEmitterLayer)
        }
        isInstalled = true
        layout(in: view.bounds)
    }

    func layout(in bounds: CGRect) {
        let overflowFrame = bounds.insetBy(
            dx: -Metrics.overflowX,
            dy: -Metrics.overflowY
        )
        sparkEmitterLayer.frame = overflowFrame
        sparkEmitterLayer.emitterPosition = CGPoint(
            x: bounds.midX - overflowFrame.minX + 6,
            y: bounds.midY - overflowFrame.minY + 6
        )
        sparkEmitterLayer.emitterSize = CGSize(width: 1, height: 1)
    }

    func update(isActive: Bool) {
        guard isInstalled else { return }
        sparkEmitterLayer.isHidden = !isActive
        if !isActive {
            sparkEmitterLayer.birthRate = 0
            sparkEmitterLayer.removeAnimation(forKey: "menuHazardSparkPulse")
            return
        }

        sparkEmitterLayer.birthRate = 1
        if sparkEmitterLayer.animation(forKey: "menuHazardSparkPulse") == nil {
            let pulse = CAKeyframeAnimation(keyPath: "birthRate")
            pulse.values = [0, 2, 14, 4, 1, 20, 6, 2, 16, 3, 1, 24, 7, 1]
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
            sparkEmitterLayer.add(pulse, forKey: "menuHazardSparkPulse")
        }
    }

    private static func makeSparkCells() -> [CAEmitterCell] {
        [makeSparkCell(), makeTracerCell()]
    }

    private static func makeSparkCell() -> CAEmitterCell {
        let cell = CAEmitterCell()
        cell.contents = sparkParticleImage()
        cell.birthRate = 1.8
        cell.lifetime = 5.6
        cell.lifetimeRange = 2.2
        cell.velocity = 96
        cell.velocityRange = 112
        cell.emissionLongitude = .pi / 2
        cell.emissionRange = .pi * 0.54
        cell.yAcceleration = 72
        cell.xAcceleration = -4
        cell.scale = 0.09
        cell.scaleRange = 0.11
        cell.alphaSpeed = -0.20
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

    private static func makeTracerCell() -> CAEmitterCell {
        let cell = CAEmitterCell()
        cell.contents = sparkParticleImage()
        cell.birthRate = 0.72
        cell.lifetime = 3.2
        cell.lifetimeRange = 1.1
        cell.velocity = 128
        cell.velocityRange = 68
        cell.emissionLongitude = .pi / 2
        cell.emissionRange = .pi * 0.34
        cell.yAcceleration = 58
        cell.xAcceleration = -14
        cell.scale = 0.06
        cell.scaleRange = 0.05
        cell.alphaSpeed = -0.28
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
}
