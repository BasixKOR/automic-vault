import Testing
@testable import MenubarHelperCore

@Test func gatePolicyRegionsMatchSupportedPresetsAndDenialPrecedence() {
    for id in ["aws", "gh", "docker", "brew", "gpg-signing", "ssh-agent", "execution"] {
        let gate = SecretGate(id: id, keyPatterns: id == "execution" || id == "brew" ? [] : ["SECRET"],
                              routes: [], defaultProtection: .noAccess, appPolicies: [])
        let levels = Array(gate.availableProtections.dropFirst())
        for protection in gate.availableProtections {
            for denial in [nil] + gate.availableProtections.map(Optional.some) {
                let regions = GatePolicyRegions(gate: gate, protection: protection, denial: denial)
                #expect(regions.levels == levels)
                #expect(regions.protection(at: regions.allowEnd) == protection)
                #expect(regions.deniesUnknown == (denial != nil))
                for classification in SecretGateRequestClassification.allCases where classification != .unknown {
                    guard let column = levels.firstIndex(where: { $0.allows(classification) }) else { continue }
                    let denied = denial.map { gate.denies(classification, at: $0) } ?? false
                    #expect((column >= regions.denyStart) == denied)
                    #expect((column < regions.effectiveAllowEnd) == (protection.allows(classification) && !denied))
                }
                for boundary in 0...levels.count {
                    #expect(regions.snappedBoundary(fraction: Double(boundary) / Double(levels.count)) == boundary)
                    let threshold = regions.denial(at: boundary)
                    let roundtrip = GatePolicyRegions(gate: gate, protection: protection, denial: threshold)
                    #expect(roundtrip.denyStart == boundary)
                }
                #expect(regions.snappedBoundary(fraction: -1) == 0)
                #expect(regions.snappedBoundary(fraction: 2) == levels.count)
                #expect(regions.snappedBoundary(fraction: .nan) == 0)
            }
        }
    }
}

@Test func gatePolicyRegionsDoNotInventAWSLocalWriteOrAllowUnknown() {
    let gate = SecretGate(id: "aws", keyPatterns: ["AWS_SECRET_ACCESS_KEY"], routes: [],
                          defaultProtection: .readOnly, appPolicies: [])
    let regions = GatePolicyRegions(gate: gate, protection: .fullIncludingSecretDumps, denial: nil)
    #expect(regions.levels == [.readOnly, .fullExceptSecretDumps, .fullIncludingSecretDumps])
    #expect(!regions.deniesUnknown)
    #expect(!SecretGateProtection.fullIncludingSecretDumps.allows(.unknown))
    let overlap = GatePolicyRegions(gate: gate, protection: .fullIncludingSecretDumps, denial: .fullExceptSecretDumps)
    #expect(overlap.allowEnd == 3)
    #expect(overlap.effectiveAllowEnd == 1)
    #expect(overlap.deniesUnknown)
    let invalid = GatePolicyRegions(gate: gate, protection: .readOnly, denial: .readOnlyAndUpdates)
    #expect(invalid.denyStart == 0)
    #expect(invalid.deniesUnknown)
}
