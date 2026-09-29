import Foundation
import Security
import Testing
@testable import MenubarHelperCore

private func denialGate(_ id: String = "gh") -> SecretGate {
    SecretGate(id: id, keyPatterns: ["TOKEN"], routes: [], defaultProtection: .fullIncludingSecretDumps, appPolicies: [])
}

@Test func denialThresholdsRespectEachGatesPresets() {
    let gh = denialGate()
    #expect(!gh.denies(.readOnly, at: .readOnlyAndLocalWrites))
    #expect(gh.denies(.localWrite, at: .readOnlyAndLocalWrites))
    #expect(gh.denies(.mutating, at: .readOnlyAndLocalWrites))
    #expect(gh.denies(.secretDump, at: .readOnlyAndLocalWrites))
    #expect(!gh.denies(.localWrite, at: .fullExceptSecretDumps))
    #expect(gh.denies(.mutating, at: .fullExceptSecretDumps))
    #expect(!gh.denies(.mutating, at: .fullIncludingSecretDumps))
    #expect(gh.denies(.secretDump, at: .fullIncludingSecretDumps))
    for threshold in gh.availableProtections {
        #expect(gh.denies(.unknown, at: threshold))
    }
    for classification in SecretGateRequestClassification.allCases {
        #expect(gh.denies(classification, at: .noAccess))
        #expect(gh.denies(classification, at: .readOnly))
    }
    let brew = denialGate("brew")
    #expect(!brew.denies(.update, at: .fullExceptSecretDumps))
    #expect(!brew.denies(.readOnly, at: .fullExceptSecretDumps))
    #expect(brew.denies(.mutating, at: .fullExceptSecretDumps))
    #expect(brew.denies(.update, at: .readOnlyAndUpdates))
    #expect(denialGate("gpg-signing").denies(.localWrite, at: .readOnlyAndLocalWrites))
    #expect(denialGate("ssh-agent").denies(.mutating, at: .fullExceptSecretDumps))
    // Unsupported values must not create a hole after a catalog change.
    #expect(brew.denies(.readOnly, at: .fullIncludingSecretDumps))
}

@Test func weakeningDenialRequiresAuthorityButStrengtheningDoesNot() {
    let gate = denialGate()
    #expect(!gate.weakeningDenial(from: nil, to: .noAccess))
    #expect(!gate.weakeningDenial(from: .fullIncludingSecretDumps, to: .fullExceptSecretDumps))
    #expect(!gate.weakeningDenial(from: .fullExceptSecretDumps, to: .fullExceptSecretDumps))
    #expect(gate.weakeningDenial(from: .fullExceptSecretDumps, to: .fullIncludingSecretDumps))
    #expect(gate.weakeningDenial(from: .fullIncludingSecretDumps, to: nil))
    #expect(!gate.weakeningDenial(from: .noAccess, to: .readOnly)) // Both deny everything.
}

@Test func secondPromptOffersScopedDenialButNeverActivatesIt() {
    let state = TemporaryLauncherDenials()
    for _ in 0..<100 {
        #expect(!state.shouldOfferDenialOnNextPrompt("claude", gateID: "gh", now: 100))
    }
    #expect(!state.recordPrompt("claude", gateID: "gh", now: 100))
    #expect(state.shouldOfferDenialOnNextPrompt("claude", gateID: "gh", now: 110))
    #expect(state.recordPrompt("claude", gateID: "gh", now: 110))
    #expect(state.shouldOfferDenialOnNextPrompt("claude", gateID: "gh", now: 120))
    #expect(!state.shouldOfferDenialOnNextPrompt("claude", gateID: "ssh-agent", now: 120))
    #expect(!state.shouldOfferDenialOnNextPrompt("other", gateID: "gh", now: 120))
    #expect(!state.shouldOfferDenialOnNextPrompt("claude", gateID: "gh", now: 141))
    #expect(state.recordPrompt("claude", gateID: "gh", now: 120))
    #expect(state.active(now: 120).isEmpty)
    #expect(!state.recordPrompt("claude", gateID: "gh", now: 151))
    #expect(!state.recordPrompt("", gateID: "gh", now: 151))
}

@Test func temporaryDenialScopeExpiryAndCancellation() throws {
    let state = TemporaryLauncherDenials()
    let gate = denialGate()
    let writes = try #require(TemporaryLauncherDenialScope(gate: gate, classification: .mutating))
    let reads = try #require(TemporaryLauncherDenialScope(gate: gate, classification: .readOnly))
    #expect(writes.threshold == .fullExceptSecretDumps)
    #expect(TemporaryLauncherDenialScope(gate: gate, classification: .unknown) == nil)
    #expect(TemporaryLauncherDenialScope(gate: denialGate("ssh-agent"), classification: .mutating)?.actionTitle
        == "Deny SSH authentication for 2 minutes")
    state.deny("claude", launcherName: "Claude", scope: writes, now: 100)
    #expect(state.deadline(for: "claude", scope: writes, now: 100) == 220)
    #expect(state.isDenied("claude", gate: gate, classification: .mutating, now: 219.999))
    #expect(state.isDenied("claude", gate: gate, classification: .unknown, now: 101))
    #expect(!state.isDenied("claude", gate: gate, classification: .localWrite, now: 101))
    #expect(!state.isDenied("claude", gate: denialGate("ssh-agent"), classification: .mutating, now: 101))
    #expect(!state.isDenied("other", gate: gate, classification: .mutating, now: 101))
    #expect(!state.isDenied("claude", gate: gate, classification: .mutating, now: 220))
    #expect(state.active(now: 220).isEmpty)
    state.deny("claude", launcherName: "Claude", scope: reads, now: 300)
    let old = try #require(state.active(now: 300).first)
    state.deny("claude", launcherName: "Claude", scope: reads, now: 310)
    state.cancel(old.id) // A stale menu action cannot remove a replacement rule.
    #expect(state.active(now: 310).count == 1)
    let broad = try #require(state.active(now: 310).first)
    state.deny("claude", launcherName: "Claude", scope: writes, now: 320)
    #expect(state.active(now: 320).count == 2)
    #expect(state.isDenied("claude", gate: gate, classification: .readOnly, now: 320))
    state.cancel(broad.id)
    #expect(!state.isDenied("claude", gate: gate, classification: .readOnly, now: 320))
    #expect(state.isDenied("claude", gate: gate, classification: .secretDump, now: 320))
    state.cancel(try #require(state.active(now: 320).first).id)
    #expect(state.active(now: 320).isEmpty)
    state.deny("", launcherName: "Invalid", scope: writes, now: 320)
    #expect(state.active(now: 320).isEmpty)
}

@Test func denialHistoryScopeIsExplicitAndBackwardCompatible() throws {
    let scope = try #require(TemporaryLauncherDenialScope(gate: denialGate(), classification: .mutating))
    let record = AccessRequestRecord(date: Date(), tool: "gh", command: "gh", decision: "Denied", reason: "Test",
        launcher: "Claude", launcherRequirement: "claude", temporaryDenialScope: scope,
        callerPath: "/gh", target: "/gh", cwd: "/", keys: [], detail: nil)
    #expect(record.redactedForDisclosure.temporaryDenialScope == scope)
    let data = try JSONEncoder().encode(record)
    #expect(try JSONDecoder().decode(AccessRequestRecord.self, from: data).temporaryDenialScope == scope)
    var legacy = try #require(JSONSerialization.jsonObject(with: data) as? [String: Any])
    legacy.removeValue(forKey: "temporaryDenialScope")
    let decoded = try JSONDecoder().decode(AccessRequestRecord.self, from: JSONSerialization.data(withJSONObject: legacy))
    #expect(decoded.temporaryDenialScope == nil)
}

@Test func denialPolicyDecodesOldRecordsAndRejectsUnknownThresholds() throws {
    let legacy = Data(#"{"gateID":"gh","requirement":"claude","protection":"readOnly"}"#.utf8)
    let decoded = try JSONDecoder().decode(SecretGatePolicyRecord.self, from: legacy)
    #expect(decoded.denialThreshold == nil)
    var denied = decoded
    denied.denialThreshold = .fullIncludingSecretDumps
    #expect(try JSONDecoder().decode(SecretGatePolicyRecord.self, from: JSONEncoder().encode(denied)) == denied)
    let invalid = Data(#"{"gateID":"gh","requirement":"claude","protection":"readOnly","denialThreshold":"future"}"#.utf8)
    #expect(throws: DecodingError.self) { try JSONDecoder().decode(SecretGatePolicyRecord.self, from: invalid) }
}

@Test(.enabled(if: dataProtectionKeychainAvailable(), "requires an entitled Keychain test host"))
func persistentDenialOverridesFullAccessAndSurvivesAllowEdits() throws {
    let service = "com.automicvault.tests.denial.\(UUID().uuidString)"
    let account = "policy"
    defer { _ = deleteStoredSecret(account: account, service: service) }
    let gate = denialGate()
    let requirement = "identifier com.example.launcher"
    #expect(setSecretGateDefaultProtection(gate.defaultProtection, for: gate,
        service: service, account: account) == errSecSuccess)
    #expect(setSecretGateDenialThreshold(.fullIncludingSecretDumps, requirement: requirement,
        in: gate, runtimeRequirement: .hardened, service: service, account: account) == errSecSuccess)
    #expect(removeSecretGatePolicies(forLauncherRequirement: requirement, service: service, account: account) == errSecSuccess)
    let inherited = reloadSecretGatePolicy(for: gate, service: service, account: account)
    #expect(inherited.appPolicies.first?.usesGateDefault == true)
    #expect(inherited.appPolicies.first?.protection == gate.defaultProtection)
    #expect(inherited.defaultPolicyLabel == "Default Policy")
    #expect(setSecretGateDenialThreshold(nil, requirement: requirement, in: gate, runtimeRequirement: .hardened,
        approvedDenialThreshold: .fullIncludingSecretDumps, service: service, account: account) == errSecSuccess)
    #expect(reloadSecretGatePolicy(for: gate, service: service, account: account).appPolicies.isEmpty)
    #expect(setSecretGateDenialThreshold(.fullIncludingSecretDumps, requirement: requirement,
        in: gate, runtimeRequirement: .hardened, service: service, account: account) == errSecSuccess)
    #expect(setSecretGateAppProtection(requirement: requirement, protection: .fullIncludingSecretDumps,
        for: gate, service: service, account: account) == errSecSuccess)
    func reason(_ classification: SecretGateRequestClassification, _ launcher: String = requirement) -> String? {
        secretGateDenialReason(gate: gate, classification: classification, launcherRequirements: [launcher],
                              service: service, account: account)
    }
    #expect(reason(.secretDump)?.hasPrefix("Denied by Launcher rule:") == true)
    #expect(secretGateDenial(gate: gate, classification: .secretDump,
        launcherRequirements: ["unmatched child", requirement], service: service, account: account)?.launcherRequirement == requirement)
    #expect(reason(.readOnly) == nil)
    #expect(reason(.secretDump, "identifier com.example.other") == nil)
    let loaded = reloadSecretGatePolicy(for: gate, service: service, account: account)
    let policy = try #require(loaded.appPolicies.first)
    #expect(policy.denialThreshold == .fullIncludingSecretDumps)
    #expect(setSecretGateDenialThreshold(.fullExceptSecretDumps, requirement: requirement,
        in: gate, runtimeRequirement: .hardened, service: service, account: account) == errSecSuccess)
    #expect(setSecretGateDenialThreshold(nil, requirement: requirement, in: gate, runtimeRequirement: .hardened,
        approvedDenialThreshold: policy.denialThreshold, service: service, account: account) == errSecAuthFailed)
    #expect(removeSecretGateAppPolicy(policy, from: gate, approvedDenialThreshold: policy.denialThreshold,
        service: service, account: account) == errSecAuthFailed)
    #expect(reason(.mutating) != nil)
    #expect(setSecretGateDenialThreshold(.fullIncludingSecretDumps, requirement: requirement, in: gate,
        runtimeRequirement: .hardened, approvedDenialThreshold: .fullExceptSecretDumps,
        service: service, account: account) == errSecSuccess)
    #expect(setSecretGateDenialThreshold(nil, requirement: requirement, in: gate, runtimeRequirement: .hardened,
        service: service, account: account) == errSecAuthFailed)
    #expect(removeSecretGateAppPolicy(policy, from: gate, service: service, account: account) == errSecAuthFailed)
    #expect(removeSecretGatePolicies(forLauncherRequirement: requirement, service: service, account: account) == errSecSuccess)
    #expect(reason(.secretDump) != nil)
    let removedAllow = reloadSecretGatePolicy(for: gate, service: service, account: account)
    #expect(removedAllow.appPolicies.first?.usesGateDefault == false)
    #expect(removedAllow.appPolicies.first?.protection == .noAccess)
    #expect(setSecretGateDenialThreshold(nil, requirement: requirement, in: gate, runtimeRequirement: .hardened,
        approvedDenialThreshold: .fullIncludingSecretDumps, service: service, account: account) == errSecSuccess)
    #expect(reason(.secretDump) == nil)
    #expect(saveKeychainData(Data("malformed".utf8), service: service, account: account, accessibility: .afterFirstUnlock) == errSecSuccess)
    #expect(reason(.readOnly) == "Denied because Authorization Policy is unavailable")
}

@Test func defaultDenialAppliesOnlyWithoutMatchingLauncherRules() {
    let gate = denialGate()
    var fallback = SecretGatePolicyRecord(gateID: gate.id, requirement: nil, protection: .readOnly)
    fallback.denialThreshold = .fullExceptSecretDumps
    let exception = SecretGatePolicyRecord(gateID: gate.id, requirement: "named", protection: .noAccess)
    var denied = SecretGatePolicyRecord(gateID: gate.id, requirement: "denied", protection: .fullIncludingSecretDumps)
    denied.denialThreshold = .readOnly
    var records = [fallback, exception, denied]
    func denial(_ classification: SecretGateRequestClassification, _ launchers: [String]) -> String? {
        secretGateDenial(gate: gate, classification: classification,
                        launcherRequirements: launchers, records: records)?.reason
    }
    #expect(denial(.readOnly, ["other"]) == nil)
    #expect(denial(.mutating, ["other"])?.hasPrefix("Denied by default rule:") == true)
    #expect(denial(.unknown, []) != nil)
    #expect(denial(.secretDump, ["named"]) == nil)
    #expect(denial(.unknown, ["unmatched child", "named"]) == nil)
    #expect(denial(.readOnly, ["named", "denied"])?.hasPrefix("Denied by Launcher rule:") == true)
    records.removeAll { $0.requirement == "named" }
    #expect(denial(.mutating, ["named"]) != nil) // Removing the exception restores fallback denial.
    var inherited = exception
    inherited.usesGateDefault = true // Inherits allow only; its own denial is None.
    records.append(inherited)
    #expect(denial(.mutating, ["named"]) == nil)
    #expect(secretGateDenial(gate: denialGate("aws"), classification: .unknown,
                            launcherRequirements: [], records: records) == nil)
}

@Test func defaultDenialMutationsPreserveAllowsAndRequireCurrentApproval() throws {
    let gate = denialGate()
    var records: [SecretGatePolicyRecord] = []
    func set(_ threshold: SecretGateProtection?, approved: SecretGateProtection? = nil,
             requirement: String? = nil) -> OSStatus {
        updateSecretGateDenialThreshold(threshold, requirement: requirement, in: gate,
            runtimeRequirement: .hardened, approvedDenialThreshold: approved, records: &records)
    }
    #expect(set(.fullIncludingSecretDumps) == errSecSuccess)
    #expect(records.first?.protection == gate.defaultProtection)
    #expect(records.first?.usesGateDefault == nil)
    let defaultAllow = SecretGatePolicyRecord(gateID: gate.id, requirement: nil, protection: .readOnly)
    #expect(replaceSecretGatePolicyRecord(defaultAllow, gate: gate,
        approvedDefaultDenialThreshold: nil, records: &records) == errSecSuccess)
    #expect(records.first?.denialThreshold == .fullIncludingSecretDumps)
    #expect(set(.fullExceptSecretDumps) == errSecSuccess)
    let before = records
    #expect(set(nil) == errSecAuthFailed)
    #expect(set(nil, approved: .fullIncludingSecretDumps) == errSecAuthFailed)
    #expect(records == before)
    #expect(set(nil, approved: .fullExceptSecretDumps) == errSecSuccess)
    #expect(records.first?.protection == .readOnly)
    #expect(records.count == 1) // Clearing default denial must not delete its allow preset.
    #expect(set(.noAccess) == errSecSuccess)
    // Creating a named row, even Approval Required, removes fallback denial.
    let exception = SecretGatePolicyRecord(gateID: gate.id, requirement: "named", protection: .noAccess)
    #expect(replaceSecretGatePolicyRecord(exception, gate: gate,
        approvedDefaultDenialThreshold: nil, records: &records) == errSecAuthFailed)
    #expect(replaceSecretGatePolicyRecord(exception, gate: gate,
        approvedDefaultDenialThreshold: .fullExceptSecretDumps, records: &records) == errSecAuthFailed)
    #expect(replaceSecretGatePolicyRecord(exception, gate: gate,
        approvedDefaultDenialThreshold: .noAccess, records: &records) == errSecSuccess)
    // Denial-only creation also cannot quietly replace a stricter default.
    #expect(set(.fullIncludingSecretDumps, requirement: "denial-only") == errSecAuthFailed)
    #expect(set(.fullIncludingSecretDumps, approved: .noAccess, requirement: "denial-only") == errSecSuccess)
    #expect(set(.noAccess, requirement: "equally-restricted") == errSecSuccess)
    #expect(records.first { $0.requirement == "denial-only" }?.usesGateDefault == true)
    #expect(set(.readOnlyAndUpdates) == errSecParam)
    let data = try JSONEncoder().encode(records)
    #expect(try JSONDecoder().decode([SecretGatePolicyRecord].self, from: data) == records)
}

@Test(.enabled(if: dataProtectionKeychainAvailable(), "requires an entitled Keychain test host"))
func defaultDenialPersistsAndReloads() throws {
    let service = "com.automicvault.tests.default-denial.\(UUID().uuidString)"
    defer { _ = deleteStoredSecret(account: "policy", service: service) }
    let gate = denialGate()
    #expect(setSecretGateDenialThreshold(.fullExceptSecretDumps, requirement: nil, in: gate,
        runtimeRequirement: .hardened, service: service, account: "policy") == errSecSuccess)
    #expect(reloadSecretGatePolicy(for: gate, service: service, account: "policy").defaultDenialThreshold == .fullExceptSecretDumps)
    #expect(secretGateDenialReason(gate: gate, classification: .unknown, launcherRequirements: [],
        service: service, account: "policy") != nil)
    #expect(setSecretGateDefaultProtection(.readOnly, for: gate, service: service, account: "policy") == errSecSuccess)
    #expect(reloadSecretGatePolicy(for: gate, service: service, account: "policy").defaultDenialThreshold == .fullExceptSecretDumps)
    #expect(setSecretGateAppProtection(requirement: "named", protection: .noAccess, for: gate,
        service: service, account: "policy") == errSecAuthFailed)
    #expect(setSecretGateAppProtection(requirement: "named", protection: .noAccess, for: gate,
        approvedDefaultDenialThreshold: .fullExceptSecretDumps, service: service, account: "policy") == errSecSuccess)
    #expect(secretGateDenialReason(gate: gate, classification: .unknown, launcherRequirements: ["named"],
        service: service, account: "policy") == nil)
}

@Test func unknownOnlyDenialPersistsAndRequiresApprovalToWeaken() throws {
    let gate = denialGate()
    var records: [SecretGatePolicyRecord] = []
    func set(_ threshold: SecretGateProtection?, approved: SecretGateProtection? = nil) -> OSStatus {
        updateSecretGateDenialThreshold(threshold, requirement: nil, in: gate,
            runtimeRequirement: .hardened, approvedDenialThreshold: approved, records: &records)
    }
    #expect(!gate.availableProtections.contains(.unknownOnly))
    for classification in SecretGateRequestClassification.allCases {
        #expect(!SecretGateProtection.unknownOnly.allows(classification))
        #expect(gate.denies(classification, at: .unknownOnly) == (classification == .unknown))
    }
    #expect(set(.fullIncludingSecretDumps) == errSecSuccess)
    #expect(set(.unknownOnly) == errSecAuthFailed)
    #expect(set(.unknownOnly, approved: .fullIncludingSecretDumps) == errSecSuccess)
    let decoded = try JSONDecoder().decode([SecretGatePolicyRecord].self, from: JSONEncoder().encode(records))
    #expect(decoded == records)
    #expect(secretGateDenial(gate: gate, classification: .unknown, launcherRequirements: [], records: decoded) != nil)
    #expect(secretGateDenial(gate: gate, classification: .secretDump, launcherRequirements: [], records: decoded) == nil)
    #expect(set(nil) == errSecAuthFailed)
    #expect(set(nil, approved: .unknownOnly) == errSecSuccess)
}
