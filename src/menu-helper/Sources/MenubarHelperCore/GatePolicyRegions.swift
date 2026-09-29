import Foundation

/// Presentation coordinates over this gate's supported presets, never authorization authority.
public struct GatePolicyRegions: Equatable {
    public let levels: [SecretGateProtection]
    public let columnCount: Int
    public let allowEnd: Int
    public let denyStart: Int
    public let deniesUnknown: Bool

    public init(gate: SecretGate, protection: SecretGateProtection, denial: SecretGateProtection?) {
        levels = Array(gate.availableProtections.dropFirst())
        columnCount = levels.count + (gate.supportsUnknownDenial ? 1 : 0)
        allowEnd = gate.availableProtections.firstIndex(of: gate.normalizedProtection(protection)) ?? 0
        // An invalid stored denial must not be pictured as allowing access.
        denyStart = denial == .unknownOnly ? (gate.supportsUnknownDenial ? levels.count : 0) : denial.map { max(0, (gate.availableProtections.firstIndex(of: $0) ?? 0) - 1) }
            ?? columnCount
        deniesUnknown = denial != nil
    }

    public var effectiveAllowEnd: Int { min(allowEnd, denyStart) }

    public func snappedBoundary(fraction: Double) -> Int {
        guard fraction.isFinite else { return 0 }
        return Int((min(1, max(0, fraction)) * Double(columnCount)).rounded())
    }

    public func protection(at boundary: Int) -> SecretGateProtection {
        boundary <= 0 ? .noAccess : levels[min(boundary, levels.count) - 1]
    }

    public func denial(at boundary: Int) -> SecretGateProtection? {
        if boundary >= columnCount { return nil }
        if boundary == levels.count { return .unknownOnly }
        return boundary <= 0 ? .noAccess : levels[boundary]
    }
}
