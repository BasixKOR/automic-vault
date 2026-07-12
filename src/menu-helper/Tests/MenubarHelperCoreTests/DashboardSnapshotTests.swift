import CryptoKit
import Foundation
import Security
import Testing
@testable import MenubarHelperCore

@Test func markdownRenderingDropsInitialHeadingMarker() {
    #expect(markdownDroppingInitialHeadingMarker("# gh-cli Detector\n\n## Trigger Conditions") == "\n## Trigger Conditions")
    #expect(markdownDroppingInitialHeadingMarker("# gh-cli Detector") == "")
    #expect(markdownDroppingInitialHeadingMarker("## Trigger Conditions") == "## Trigger Conditions")
}

@Test func scanJSONCountsUniqueFlaggedDetectors() throws {
    let data = Data("""
    {"findings":[
      {"source":"git","severity":"high","affected":[]},
      {"source":"git","severity":"high","affected":[]},
      {"source":"aws","severity":"high","affected":[]}
    ]}
    """.utf8)

    let snapshot = DashboardSnapshot(
        detectors: [],
        detectorFindings: try detectorFindings(from: data),
        hardenedTools: [],
        secretGates: [],
        secrets: []
    )

    #expect(snapshot.flaggedDetectorCount == 2)
    #expect(snapshot.detectorDisplayCount == 2)
}

@Test func cleanScanDisplaysTotalDetectorCount() {
    let snapshot = DashboardSnapshot(
        detectors: [
            DetectorMetadata(name: "aws", homepage: "", docsURL: ""),
            DetectorMetadata(name: "git", homepage: "", docsURL: ""),
        ],
        detectorFindings: [],
        hardenedTools: [],
        secretGates: [],
        secrets: []
    )

    #expect(snapshot.flaggedDetectorCount == 0)
    #expect(snapshot.detectorDisplayCount == 2)
}

@Test func detectorMetadataDecodesAllDetectors() throws {
    let data = Data("""
    {"detectors":[{"name":"git","homepage":"https://git-scm.com/","docs_url":"https://example.test/git","documentation":"# git Detector"}]}
    """.utf8)

    #expect(try detectorMetadata(from: data) == [
        DetectorMetadata(name: "git", homepage: "https://git-scm.com/", docsURL: "https://example.test/git", documentation: "# git Detector")
    ])
}

@Test func detectorMetadataAcceptsOlderDetectorOutput() throws {
    let data = Data("""
    {"detectors":[{"name":"git","homepage":"https://git-scm.com/","docs_url":"https://example.test/git"}]}
    """.utf8)

    #expect(try detectorMetadata(from: data) == [
        DetectorMetadata(name: "git", homepage: "https://git-scm.com/", docsURL: "https://example.test/git")
    ])
}

@Test func splitDetectorNamesDisplayPackageAndKind() {
    #expect(detectorDisplayName("git-credential-fill") == DetectorDisplayName(packageName: "git", kind: "credential fill"))
    #expect(detectorDisplayName("aws-cli-login-cache") == DetectorDisplayName(packageName: "aws-cli", kind: "login cache"))
    #expect(detectorDisplayName("docker-root-access") == DetectorDisplayName(packageName: "docker", kind: "root access"))
    #expect(detectorDisplayName("homebrew") == DetectorDisplayName(packageName: "homebrew", kind: "mutable"))
    #expect(detectorDisplayName("sip") == DetectorDisplayName(packageName: "SIP", kind: "system integrity"))
    #expect(detectorDisplayName("sudo") == DetectorDisplayName(packageName: "sudo", kind: "system integrity"))
}

@Test func homebrewExecutablePathsNormalizeToStableOptPath() {
    let symlinks = ["/opt/homebrew/bin/gh": "../Cellar/gh-cli/2.96.0/bin/gh"]
    let expected = "/opt/homebrew/opt/gh-cli/bin/gh"

    #expect(normalizedExecutablePath("/opt/homebrew/bin/gh") { symlinks[$0] } == expected)
    #expect(normalizedExecutablePath("/opt/homebrew/Cellar/gh-cli/2.96.0/bin/gh") { _ in nil } == expected)
    #expect(normalizedExecutablePath("/opt/homebrew/opt/gh-cli/bin/gh") { _ in nil } == expected)
}

@Test func singleDetectorNamesDefaultToPlaintextSecretKind() {
    #expect(detectorDisplayName("docker-machine") == DetectorDisplayName(packageName: "docker-machine", kind: "plaintext secret"))
    #expect(detectorDisplayName("docker-credential-helper") == DetectorDisplayName(packageName: "docker-credential-helper", kind: "plaintext secret"))
    #expect(detectorDisplayName("curl") == DetectorDisplayName(packageName: "curl", kind: "plaintext secret"))
}

@Test func hardenerMetadataDecodesDocumentation() throws {
    let data = Data("""
    {"hardeners":[{"name":"aws","documentation":"## What It Does","hardened":true,"stub_path":"/usr/local/bin/aws","target_path":"/opt/homebrew/bin/aws"}]}
    """.utf8)

    #expect(try hardenerMetadata(from: data) == [
        HardenerMetadata(
            name: "aws",
            documentation: "## What It Does",
            hardened: true,
            stubPath: "/usr/local/bin/aws",
            targetPath: "/opt/homebrew/bin/aws"
        )
    ])
}

@Test func hardenerMetadataDecodesSecretGateDescriptor() throws {
    let data = Data(#"""
    {"hardeners":[{"name":"gh","documentation":"","hardened":true,"stub_path":null,"target_path":"/opt/homebrew/opt/gh-cli/bin/gh","secret_gate":{"id":"gh","key_patterns":["GH_TOKEN_*"],"routes":[{"operation":"keys","script_path":null,"target_path":"/opt/homebrew/opt/gh-cli/bin/gh","caller_identifiers":["gh","com.github.cli"],"key_patterns":["GH_TOKEN_*"],"replace_existing_env":true,"allow_missing_keys":false}]}}]}
    """#.utf8)

    let hardener = try #require(try hardenerMetadata(from: data).first)
    #expect(hardener.secretGate?.id == "gh")
    #expect(hardener.secretGate?.keyPatterns == ["GH_TOKEN_*"])
    #expect(hardener.secretGate?.routes.first?.callerIdentifiers == ["gh", "com.github.cli"])
}

@Test func doctorJSONFlattensIssuesWithHardenerNames() throws {
    let data = Data(#"""
    {"results":[
      {"name":"aws","commands":["aws"],"issues":[{"kind":"stub_not_first_on_path","command":"aws","message":"aws is shadowed","remediation":"Fix PATH.","stub_path":"/usr/local/bin/aws","target_path":"/opt/homebrew/bin/aws","resolved_path":"/opt/homebrew/bin/aws"}]},
      {"name":"gh","commands":["gh"],"issues":[]}
    ]}
    """#.utf8)

    #expect(try doctorIssues(from: data) == [
        DoctorIssue(
            hardener: "aws",
            kind: "stub_not_first_on_path",
            command: "aws",
            message: "aws is shadowed",
            remediation: "Fix PATH.",
            stubPath: "/usr/local/bin/aws",
            targetPath: "/opt/homebrew/bin/aws",
            resolvedPath: "/opt/homebrew/bin/aws"
        )
    ])
}

@Test func unavailableLoginShellPATHSuppressesMisleadingPATHIssues() throws {
    let data = Data(#"""
    {"results":[{"name":"aws","commands":["aws"],"issues":[
      {"kind":"stub_not_first_on_path","command":"aws","message":"shadowed","remediation":"Fix PATH.","stub_path":"/usr/local/bin/aws","target_path":"/opt/homebrew/bin/aws","resolved_path":"/opt/homebrew/bin/aws"},
      {"kind":"hardening_not_applied","command":"aws","message":"not hardened","remediation":"Harden it.","stub_path":"/usr/local/bin/aws","target_path":"/opt/homebrew/bin/aws","resolved_path":null}
    ]}]}
    """#.utf8)

    let issues = try doctorIssues(from: data, loginShellPATHAvailable: false)

    #expect(issues.map(\.kind) == ["hardening_not_applied", "login_shell_path_unavailable"])
}

@Test func JSONLoaderCanAcceptDoctorIssueExitStatus() throws {
    let data = try #require(loadJSON(
        avExecutableURL: URL(fileURLWithPath: "/bin/sh"),
        arguments: ["-c", "printf '{\"results\":[]}'; exit 1"],
        acceptedTerminationStatuses: [0, 1]
    ))

    #expect(try doctorIssues(from: data).isEmpty)
}

@Test func detectorDocumentationReferencesHardenerCommand() {
    #expect(hardenerNameReferencedByDocumentation("```sh\nav harden gh\n```") == "gh")
    #expect(hardenerNameReferencedByDocumentation("Run `sudo av harden aws` after import.") == "aws")
    #expect(hardenerNameReferencedByDocumentation("No mitigation command here.") == nil)
}

@Test func hardenedToolsUseHardenerDetection() throws {
    let directory = temporaryDirectory()
    let tools = loadHardenedTools(
        in: directory,
        ghCLIURL: nil,
        metadata: [
            HardenerMetadata(
                name: "aws",
                documentation: "AWS docs",
                hardened: true,
                stubPath: "/usr/local/bin/aws",
                targetPath: "/opt/homebrew/bin/aws"
            ),
            HardenerMetadata(
                name: "sudo",
                documentation: "Sudo docs",
                hardened: true,
                targetPath: "/etc/pam.d/sudo_local"
            ),
            HardenerMetadata(name: "gh-cli", documentation: "GitHub docs", hardened: false),
        ]
    )

    #expect(tools.map(\.name) == ["aws", "sudo"])
    #expect(tools.map(\.documentation) == ["AWS docs", "Sudo docs"])
    #expect(tools.first?.stubPath == "/usr/local/bin/aws")
    #expect(tools.last?.stubPath == nil)
}


private func testGateMetadata(hardened: Bool = true) -> HardenerMetadata {
    HardenerMetadata(
        name: "gh",
        hardened: hardened,
        stubPath: "/opt/homebrew/opt/gh-cli/bin/gh",
        targetPath: "/opt/homebrew/opt/gh-cli/bin/gh",
        secretGate: SecretGateDescriptor(
            id: "gh",
            keyPatterns: ["GH_TOKEN_*"],
            routes: [SecretGateRoute(
                operation: "keys",
                scriptPath: nil,
                targetPath: "/opt/homebrew/opt/gh-cli/bin/gh",
                callerIdentifiers: ["gh", "com.github.cli"],
                keyPatterns: ["GH_TOKEN_*"],
                replaceExistingEnv: true,
                allowMissingKeys: false
            )]
        )
    )
}

@Test func hardenedToolGetsOneGateWithoutStoredSecrets() {
    let service = "com.automicvault.tests.\(UUID().uuidString)"
    let gates = loadSecretGates(hardeners: [testGateMetadata(), testGateMetadata(hardened: false)], service: service)

    #expect(gates.count == 1)
    #expect(gates.first?.id == "gh")
    #expect(gates.first?.keyPatterns == ["GH_TOKEN_*"])
    #expect(gates.first?.defaultProtection == .noAccess)
    #expect(gates.first?.appPolicies.isEmpty == true)
}

@Test(
    arguments: SecretGateProtection.allCases,
    SecretGateRequestClassification.allCases
)
func protectionPolicyMatrix(
    protection: SecretGateProtection,
    classification: SecretGateRequestClassification
) {
    let expected = switch protection {
    case .noAccess: false
    case .readOnly: classification == .readOnly
    case .fullExceptSecretDumps: classification != .secretDump
    case .fullIncludingSecretDumps: true
    }
    #expect(protection.allows(classification) == expected)
}

@Test func secretGatePoliciesPersistAndResolveOverrides() throws {
    guard dataProtectionKeychainAvailable() else { return }
    let service = "com.automicvault.tests.\(UUID().uuidString)"
    let account = "policies.\(UUID().uuidString)"
    defer { _ = deleteStoredSecret(account: account, service: service) }
    let metadata = testGateMetadata()
    var gate = try #require(loadSecretGates(hardeners: [metadata], service: service, account: account).first)
    let requirement = #"identifier "com.example.app""#

    #expect(setSecretGateDefaultProtection(.fullExceptSecretDumps, for: gate, service: service, account: account) == errSecSuccess)
    gate = try #require(loadSecretGates(hardeners: [metadata], service: service, account: account).first)
    #expect(secretGateProtection(for: nil, in: gate).protection == .fullExceptSecretDumps)

    #expect(setSecretGateAppProtection(
        requirement: requirement,
        protection: .noAccess,
        for: gate,
        service: service,
        account: account
    ) == errSecSuccess)
    gate = try #require(loadSecretGates(hardeners: [metadata], service: service, account: account).first)
    let appPolicy = try #require(gate.appPolicies.first)
    #expect(appPolicy.protection == .noAccess)
    #expect(secretGateProtection(for: requirement, in: gate).protection == .noAccess)
    #expect(secretGateProtection(for: #"identifier "com.other.app""#, in: gate).protection == .fullExceptSecretDumps)

    #expect(removeSecretGateAppPolicy(appPolicy, from: gate, service: service, account: account) == errSecSuccess)
    gate = try #require(loadSecretGates(hardeners: [metadata], service: service, account: account).first)
    #expect(gate.appPolicies.isEmpty)
    #expect(secretGateProtection(for: requirement, in: gate).protection == .fullExceptSecretDumps)
}

@Test func defaultNoAccessIsNotPersisted() throws {
    guard dataProtectionKeychainAvailable() else { return }
    let service = "com.automicvault.tests.\(UUID().uuidString)"
    let account = "policies.\(UUID().uuidString)"
    defer { _ = deleteStoredSecret(account: account, service: service) }
    let metadata = testGateMetadata()
    let gate = try #require(loadSecretGates(hardeners: [metadata], service: service, account: account).first)

    #expect(setSecretGateDefaultProtection(.noAccess, for: gate, service: service, account: account) == errSecSuccess)
    #expect(loadSecretGates(hardeners: [metadata], service: service, account: account).first?.defaultProtection == .noAccess)
}

@Test func storedSecretsListNamesOnlyAndDelete() throws {
    guard dataProtectionKeychainAvailable() else { return }
    let service = "com.automicvault.tests.\(UUID().uuidString)"
    #expect(saveStoredSecret(account: "API_TOKEN", value: "secret", service: service) == errSecSuccess)
    defer { _ = deleteStoredSecret(account: "API_TOKEN", service: service) }

    let secrets = loadStoredSecrets(service: service)
    #expect(secrets.map(\.account) == ["API_TOKEN"])
    #expect(secrets.first?.keychainProperties.contains("Data Protection Enabled") == true)
    #expect(secrets.first?.keychainProperties.contains("iCloud Off") == true)
    #expect(deleteStoredSecret(account: "API_TOKEN", service: service) == errSecSuccess)
    #expect(loadStoredSecrets(service: service).isEmpty)
}

@Test func storedSecretsUseDataProtectionKeychain() throws {
    guard dataProtectionKeychainAvailable() else { return }
    let service = "com.automicvault.tests.\(UUID().uuidString)"
    #expect(saveStoredSecret(account: "API_TOKEN", value: "secret", service: service) == errSecSuccess)
    defer { _ = deleteStoredSecret(account: "API_TOKEN", service: service) }

    let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: service,
        kSecAttrAccount as String: "API_TOKEN",
        kSecUseDataProtectionKeychain as String: true,
        kSecReturnAttributes as String: true,
        kSecMatchLimit as String: kSecMatchLimitOne,
    ]
    var result: CFTypeRef?
    #expect(SecItemCopyMatching(query as CFDictionary, &result) == errSecSuccess)
    let attributes = try #require(result as? [String: Any])
    #expect(attributes[kSecAttrAccessible as String] as? String == kSecAttrAccessibleWhenUnlocked as String)
}

@Test func storedSecretsCanBeRenamed() throws {
    guard dataProtectionKeychainAvailable() else { return }
    let service = "com.automicvault.tests.\(UUID().uuidString)"
    #expect(saveStoredSecret(account: "OLD_TOKEN", value: "secret", service: service) == errSecSuccess)
    defer { _ = deleteStoredSecret(account: "OLD_TOKEN", service: service) }
    defer { _ = deleteStoredSecret(account: "NEW_TOKEN", service: service) }

    #expect(renameStoredSecret(account: "OLD_TOKEN", to: "NEW_TOKEN", service: service) == errSecSuccess)
    #expect(loadStoredSecrets(service: service).map(\.account) == ["NEW_TOKEN"])
}

@Test func accessRequestLogKeepsNewestFifty() throws {
    let defaultsName = "com.automicvault.tests.defaults.\(UUID().uuidString)"
    let defaults = try #require(UserDefaults(suiteName: defaultsName))
    defer { defaults.removePersistentDomain(forName: defaultsName) }

    for index in 0..<55 {
        appendAccessRequestRecord(AccessRequestRecord(
            date: Date(timeIntervalSince1970: TimeInterval(index)),
            tool: "aws",
            command: "aws s3 ls \(index)",
            decision: "Approved",
            approvalSource: "Human",
            reason: "Approved in prompt",
            launcher: "Codex",
            callerPath: "/usr/local/bin/av",
            target: "/opt/homebrew/bin/aws",
            cwd: "/tmp",
            keys: ["AWS_ACCESS_KEY_ID"],
            detail: nil
        ), defaults: defaults)
    }

    let records = loadAccessRequestRecords(defaults: defaults)
    #expect(records.count == 50)
    #expect(records.first?.command == "aws s3 ls 54")
    #expect(records.first?.approvalSourceLabel == "Human")
    #expect(records.last?.command == "aws s3 ls 5")
}

@Test func accessRequestLogInfersSourceForOlderRecords() throws {
    let data = Data("""
    [{
      "id": "00000000-0000-0000-0000-000000000001",
      "date": 0,
      "tool": "gh",
      "command": "gh pr list",
      "decision": "Approved",
      "reason": "Auto-approved read-only gh request",
      "launcher": "Codex",
      "callerPath": "/opt/homebrew/bin/gh",
      "target": "/opt/homebrew/bin/gh",
      "cwd": "/tmp",
      "keys": [],
      "detail": null
    }]
    """.utf8)

    let records = try JSONDecoder().decode([AccessRequestRecord].self, from: data)
    #expect(records.first?.approvalSource == nil)
    #expect(records.first?.approvalSourceLabel == "Auto")
}

private func temporaryDirectory() -> URL {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("av-menubar-tests-\(UUID().uuidString)", isDirectory: true)
    try! FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
    return url
}

private func dataProtectionKeychainAvailable() -> Bool {
    let service = "com.automicvault.tests.probe.\(UUID().uuidString)"
    let status = saveStoredSecret(account: "PROBE", value: "secret", service: service)
    defer { _ = deleteStoredSecret(account: "PROBE", service: service) }
    return status != errSecMissingEntitlement
}
