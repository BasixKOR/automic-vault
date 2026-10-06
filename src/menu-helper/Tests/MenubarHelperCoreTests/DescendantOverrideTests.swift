import Foundation
import Security
import Testing
@testable import MenubarHelperCore

private let overrideGate = SecretGate(id: "gh", keyPatterns: ["SECRET"], routes: [],
    defaultProtection: .readOnly, appPolicies: [])
private let overridePolicy = SecretGatePolicy(bundleIdentifier: "test", requirement: "test",
    protection: .readOnly, runtimeRequirement: .hardened)

@Test func descendantOverrideDefaultsOffAndRejectsMalformedStorage() throws {
    let legacy = Data(#"{"gateID":"gh","requirement":"test","protection":"readOnly"}"#.utf8)
    let record = try JSONDecoder().decode(SecretGatePolicyRecord.self, from: legacy)
    #expect(record.overridesDescendantRules == nil)
    let descriptor = SecretGateDescriptor(id: "gh", keyPatterns: [], routes: [])
    let loaded = loadedSecretGate(from: descriptor, policyRecords: .success([record]))
    #expect(loaded.appPolicies.first?.overridesDescendantRules == false)
    let malformed = Data(#"{"gateID":"gh","requirement":"test","protection":"readOnly","overridesDescendantRules":"true"}"#.utf8)
    #expect(throws: (any Error).self) { try JSONDecoder().decode(SecretGatePolicyRecord.self, from: malformed) }
}

@Test func descendantOverrideRequiresAnUnchangedExplicitRule() throws {
    let record = SecretGatePolicyRecord(gateID: "gh", requirement: "test", protection: .readOnly, runtimeRequirement: .hardened)
    var records = [record]
    #expect(updateSecretGateDescendantOverride(true, for: overridePolicy, in: overrideGate, records: &records) == errSecSuccess)
    #expect(records.first?.overridesDescendantRules == true)
    #expect(try JSONDecoder().decode([SecretGatePolicyRecord].self, from: JSONEncoder().encode(records)) == records)
    // Stale Approval cannot disable a newly enabled override.
    #expect(updateSecretGateDescendantOverride(false, for: overridePolicy, in: overrideGate, records: &records) == errSecAuthFailed)
    let approved = SecretGatePolicy(bundleIdentifier: "test", requirement: "test", protection: .readOnly,
        runtimeRequirement: .hardened, overridesDescendantRules: true)
    #expect(updateSecretGateDescendantOverride(false, for: approved, in: overrideGate, records: &records) == errSecSuccess)
    #expect(records.first?.overridesDescendantRules == false)
    for mutation in 0..<4 {
        records = [record]
        switch mutation {
        case 0: records.removeAll()
        case 1: records[0].protection = .fullExceptSecretDumps
        case 2: records[0].denialThreshold = .noAccess
        default: records[0].usesGateDefault = true
        }
        let before = records
        #expect(updateSecretGateDescendantOverride(true, for: overridePolicy, in: overrideGate, records: &records) == errSecAuthFailed)
        #expect(records == before)
    }
}

@Test func descendantOverrideSurvivesPolicyEditsWithoutBypassingDenial() {
    var record = SecretGatePolicyRecord(gateID: "gh", requirement: "test", protection: .readOnly, runtimeRequirement: .hardened)
    record.overridesDescendantRules = true
    record.denialThreshold = .fullIncludingSecretDumps
    var records = [record]
    let replacement = SecretGatePolicyRecord(gateID: "gh", requirement: "test", protection: .fullExceptSecretDumps, runtimeRequirement: .hardened)
    #expect(replaceSecretGatePolicyRecord(replacement, gate: overrideGate, approvedDefaultDenialThreshold: nil, records: &records) == errSecSuccess)
    #expect(records.first?.overridesDescendantRules == true)
    #expect(records.first?.denialThreshold == .fullIncludingSecretDumps)
    #expect(updateSecretGateDenialThreshold(.fullExceptSecretDumps, requirement: "test", in: overrideGate,
        runtimeRequirement: .hardened, approvedDenialThreshold: nil, records: &records) == errSecSuccess)
    #expect(records.first?.overridesDescendantRules == true)
    var child = SecretGatePolicyRecord(gateID: "gh", requirement: "child", protection: .readOnly, runtimeRequirement: .hardened)
    child.denialThreshold = .fullExceptSecretDumps
    records.append(child)
    #expect(secretGateDenial(gate: overrideGate, classification: .mutating,
        launcherRequirements: ["child", "test"], records: records) != nil)
    #expect(secretGateDenial(gate: overrideGate, classification: .unknown,
        launcherRequirements: ["child", "test"], records: records) != nil)
}

@Test func sshAncestorsCannotSuppressNearestLauncherDefaultDenial() {
    let gate = SecretGate(id: "ssh-agent", keyPatterns: ["SSH"], routes: [], defaultProtection: .noAccess, appPolicies: [])
    var fallback = SecretGatePolicyRecord(gateID: gate.id, requirement: nil, protection: .noAccess)
    fallback.denialThreshold = .noAccess
    var parent = SecretGatePolicyRecord(gateID: gate.id, requirement: "parent", protection: .fullExceptSecretDumps)
    for enabled in [false, true] {
        parent.overridesDescendantRules = enabled
        let records = [fallback, parent]
        #expect(secretGateDenial(gate: gate, classification: .mutating,
            launcherRequirements: ["child", "parent"], defaultPolicyLauncherRequirements: ["child"], records: records) != nil)
        let child = SecretGatePolicyRecord(gateID: gate.id, requirement: "child", protection: .noAccess)
        #expect(secretGateDenial(gate: gate, classification: .mutating,
            launcherRequirements: ["child", "parent"], defaultPolicyLauncherRequirements: ["child"], records: records + [child]) == nil)
    }
    parent.denialThreshold = .noAccess
    #expect(secretGateDenial(gate: gate, classification: .mutating,
        launcherRequirements: ["child", "parent"], defaultPolicyLauncherRequirements: ["child"], records: [parent]) != nil)
}
