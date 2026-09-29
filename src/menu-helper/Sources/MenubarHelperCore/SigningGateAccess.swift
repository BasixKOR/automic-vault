import Foundation

/// The three exclusive choices for the SSH Agent and GPG Signing gates.
/// Storage still uses the existing allow and denial fields.
public enum SigningGateAccess: CaseIterable, Hashable, Sendable {
    case approvalRequired, allow, deny

    public init(protection: SecretGateProtection, denial: SecretGateProtection?) {
        self = denial != nil ? .deny : protection == .noAccess ? .approvalRequired : .allow
    }

    public var denial: SecretGateProtection? { self == .deny ? .noAccess : nil }

    public func protection(for gate: SecretGate) -> SecretGateProtection {
        self == .allow ? gate.availableProtections.last ?? .noAccess : .noAccess
    }

    public func title(for gate: SecretGate) -> String {
        self == .deny ? String(localized: "Deny") : gate.protectionTitle(protection(for: gate))
    }

    public func requiresApproval(in gate: SecretGate, protection: SecretGateProtection,
                                 denial: SecretGateProtection?, usesGateDefault: Bool = false) -> Bool {
        (self == .allow && (usesGateDefault || gate.normalizedProtection(protection) == .noAccess))
            || gate.weakeningDenial(from: denial, to: self.denial)
    }
}
