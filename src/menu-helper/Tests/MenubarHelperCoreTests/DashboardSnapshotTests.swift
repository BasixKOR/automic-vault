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
    #expect(detectorDisplayName("homebrew") == DetectorDisplayName(packageName: "homebrew", kind: "unhardened install"))
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

@Test func secretGatesDecodeRememberedApprovals() throws {
    guard dataProtectionKeychainAvailable() else { return }
    let service = "com.automicvault.tests.\(UUID().uuidString)"
    defer { _ = deleteStoredSecret(account: trustedScriptApprovalsKeychainAccount, service: service) }

    #expect(saveTrustedScriptApprovals([
        TrustedScriptApproval(
            scriptPath: "/tmp/deploy",
            scriptChecksum: "abc",
            keys: ["B", "A"],
            target: "/bin/echo",
            replaceExistingEnv: true,
            allowMissingKeys: false,
            launcherRequirement: #"identifier "com.example.app""#
        ),
        TrustedScriptApproval(
            scriptPath: "/tmp/deploy",
            scriptChecksum: "abc",
            keys: ["A", "B"],
            target: "/bin/echo",
            replaceExistingEnv: true,
            allowMissingKeys: false,
            launcherRequirement: #"identifier "com.other.app""#
        ),
        TrustedScriptApproval(
            scriptPath: "/tmp/deploy",
            scriptChecksum: "def",
            keys: ["A", "B"],
            target: "/bin/echo",
            replaceExistingEnv: true,
            allowMissingKeys: false,
            launcherRequirement: #"identifier "com.third.app""#
        ),
        TrustedScriptApproval(
            scriptPath: nil,
            scriptChecksum: nil,
            keys: ["A", "B"],
            target: "/bin/echo",
            replaceExistingEnv: true,
            allowMissingKeys: false,
            launcherRequirement: #"identifier "com.direct.app""#
        )
    ], service: service) == errSecSuccess)

    #expect(loadSecretGates(service: service) == [
        SecretGate(
            scriptPath: "/tmp/deploy",
            scriptChecksum: "abc",
            keys: ["A", "B"],
            target: "/bin/echo",
            replaceExistingEnv: true,
            allowMissingKeys: false,
            approvedApps: [
                SecretGateApprovedApp(bundleIdentifier: "com.example.app", requirement: #"identifier "com.example.app""#),
                SecretGateApprovedApp(bundleIdentifier: "com.other.app", requirement: #"identifier "com.other.app""#),
            ]
        ),
        SecretGate(
            scriptPath: "/tmp/deploy",
            scriptChecksum: "def",
            keys: ["A", "B"],
            target: "/bin/echo",
            replaceExistingEnv: true,
            allowMissingKeys: false,
            approvedApps: [
                SecretGateApprovedApp(bundleIdentifier: "com.third.app", requirement: #"identifier "com.third.app""#),
            ]
        ),
    ])
}

@Test func configuredSecretGatesDoNotRequireStoredKeys() throws {
    let directory = temporaryDirectory()
    let stub = directory.appendingPathComponent("aws")
    let contents = """
    #!/usr/local/bin/av inject --replace-existing-env +AWS_SECRET_ACCESS_KEY +AWS_ACCESS_KEY_ID /bin/zsh
    echo ignored
    """
    try contents.write(to: stub, atomically: true, encoding: .utf8)
    try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: stub.path)
    defer { try? FileManager.default.removeItem(at: directory) }

    let gates = loadSecretGates(
        configuredTools: [HardenedTool(name: "aws", stubPath: stub.path, targetPath: "/opt/homebrew/bin/aws")],
        service: "com.automicvault.tests.\(UUID().uuidString)"
    )

    #expect(gates == [
        SecretGate(
            scriptPath: stub.standardizedFileURL.path,
            scriptChecksum: SHA256.hash(data: Data(contents.utf8)).map { String(format: "%02x", $0) }.joined(),
            keys: ["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY"],
            target: "/bin/zsh",
            replaceExistingEnv: true,
            allowMissingKeys: false,
            approvedApps: []
        )
    ])
}

@Test func secretGateAppsCanBeAddedAndRemoved() throws {
    guard dataProtectionKeychainAvailable() else { return }
    let service = "com.automicvault.tests.\(UUID().uuidString)"
    defer { _ = deleteStoredSecret(account: trustedScriptApprovalsKeychainAccount, service: service) }

    let gate = SecretGate(
        scriptPath: "/tmp/deploy",
        scriptChecksum: "abc",
        keys: ["A", "B"],
        target: "/bin/echo",
        replaceExistingEnv: true,
        allowMissingKeys: false,
        approvedApps: []
    )
    let requirement = #"identifier "com.example.app""#

    #expect(rememberTrustedApp(requirement: requirement, for: gate, service: service) == errSecSuccess)
    #expect(loadSecretGates(service: service).first?.approvedApps == [
        SecretGateApprovedApp(bundleIdentifier: "com.example.app", requirement: requirement)
    ])
    #expect(forgetTrustedApp(SecretGateApprovedApp(bundleIdentifier: "com.example.app", requirement: requirement), from: gate, service: service) == errSecSuccess)
    #expect(loadSecretGates(service: service).isEmpty)
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
