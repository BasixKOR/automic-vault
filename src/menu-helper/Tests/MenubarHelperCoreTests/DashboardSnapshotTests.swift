import Foundation
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

@Test func secretGateAppsCanBeAddedAndRemoved() throws {
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
    let service = "com.automicvault.tests.\(UUID().uuidString)"
    #expect(saveStoredSecret(account: "API_TOKEN", value: "secret", service: service) == errSecSuccess)
    defer { _ = deleteStoredSecret(account: "API_TOKEN", service: service) }

    #expect(loadStoredSecrets(service: service) == [StoredSecret(account: "API_TOKEN")])
    #expect(deleteStoredSecret(account: "API_TOKEN", service: service) == errSecSuccess)
    #expect(loadStoredSecrets(service: service).isEmpty)
}

@Test func storedSecretsCanBeRenamed() throws {
    let service = "com.automicvault.tests.\(UUID().uuidString)"
    #expect(saveStoredSecret(account: "OLD_TOKEN", value: "secret", service: service) == errSecSuccess)
    defer { _ = deleteStoredSecret(account: "OLD_TOKEN", service: service) }
    defer { _ = deleteStoredSecret(account: "NEW_TOKEN", service: service) }

    #expect(renameStoredSecret(account: "OLD_TOKEN", to: "NEW_TOKEN", service: service) == errSecSuccess)
    #expect(loadStoredSecrets(service: service) == [StoredSecret(account: "NEW_TOKEN")])
}

private func temporaryDirectory() -> URL {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("av-menubar-tests-\(UUID().uuidString)", isDirectory: true)
    try! FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
    return url
}
