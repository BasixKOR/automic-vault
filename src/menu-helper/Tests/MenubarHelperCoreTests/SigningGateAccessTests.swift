import Security
import Testing
@testable import MenubarHelperCore

@Test func signingGateAccessTransitionsAreExclusiveAndRequireApproval() {
    for id in ["ssh-agent", "gpg-signing"] {
        let gate = SecretGate(id: id, keyPatterns: ["SECRET"], routes: [],
                              defaultProtection: .noAccess, appPolicies: [])
        for requirement: String? in [nil, "launcher"] {
            for old in SigningGateAccess.allCases {
                for new in SigningGateAccess.allCases {
                    var record = SecretGatePolicyRecord(gateID: id, requirement: requirement,
                        protection: old.protection(for: gate), runtimeRequirement: .hardenedAllowingLibraryValidationDisabled)
                    record.denialThreshold = old.denial
                    var records = [record]
                    let needsApproval = new.requiresApproval(in: gate, protection: record.protection,
                                                              denial: record.denialThreshold)
                    #expect(needsApproval == ((new == .allow && old != .allow) || (old == .deny && new != .deny)))
                    let status = updateSecretGateDenialThreshold(new.denial, requirement: requirement, in: gate,
                        runtimeRequirement: .hardened, approvedDenialThreshold: needsApproval ? old.denial : nil,
                        signingAccess: new, records: &records)
                    #expect(status == errSecSuccess)
                    #expect(records.count == 1)
                    #expect(records[0].protection == new.protection(for: gate))
                    #expect(records[0].denialThreshold == new.denial)
                    #expect(records[0].runtimeRequirement == record.runtimeRequirement)
                    #expect(SigningGateAccess(protection: records[0].protection, denial: records[0].denialThreshold) == new)
                }
            }
        }
        // Legacy overlap and denial-only rows must become explicit Approval Required,
        // never revive their stored allow level or inherit a broad default.
        for inherited in [false, true] {
            var record = SecretGatePolicyRecord(gateID: id, requirement: "launcher",
                                                protection: .fullExceptSecretDumps)
            record.denialThreshold = .noAccess
            record.usesGateDefault = inherited
            var records = [record]
            #expect(SigningGateAccess(protection: record.protection, denial: record.denialThreshold) == .deny)
            for approved: SecretGateProtection? in [nil, gate.availableProtections.last] {
                #expect(updateSecretGateDenialThreshold(nil, requirement: "launcher", in: gate,
                    runtimeRequirement: .hardened, approvedDenialThreshold: approved,
                    signingAccess: .approvalRequired, records: &records) == errSecAuthFailed)
                #expect(records == [record])
            }
            #expect(updateSecretGateDenialThreshold(nil, requirement: "launcher", in: gate,
                runtimeRequirement: .hardened, approvedDenialThreshold: .noAccess,
                signingAccess: .approvalRequired, records: &records) == errSecSuccess)
            #expect(records[0].protection == .noAccess)
            #expect(records[0].denialThreshold == nil)
            #expect(records[0].usesGateDefault == nil)
        }
        #expect(SigningGateAccess.allow.requiresApproval(in: gate,
            protection: .fullExceptSecretDumps, denial: nil, usesGateDefault: true))
        var records: [SecretGatePolicyRecord] = []
        #expect(updateSecretGateDenialThreshold(nil, requirement: nil, in: gate,
            runtimeRequirement: .hardened, approvedDenialThreshold: nil,
            signingAccess: .deny, records: &records) == errSecParam)
        #expect(records.isEmpty)
    }
    let gate = SecretGate(id: "gh", keyPatterns: ["SECRET"], routes: [],
                          defaultProtection: .readOnly, appPolicies: [])
    var records: [SecretGatePolicyRecord] = []
    #expect(updateSecretGateDenialThreshold(nil, requirement: nil, in: gate,
        runtimeRequirement: .hardened, approvedDenialThreshold: nil,
        signingAccess: .allow, records: &records) == errSecParam)
    #expect(records.isEmpty)
}
