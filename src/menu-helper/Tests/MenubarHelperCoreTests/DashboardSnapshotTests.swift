import Foundation
import Testing
@testable import MenubarHelperCore

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
}

@Test func detectorMetadataDecodesAllDetectors() throws {
    let data = Data("""
    {"detectors":[{"name":"git","homepage":"https://git-scm.com/","docs_url":"https://example.test/git"}]}
    """.utf8)

    #expect(try detectorMetadata(from: data) == [
        DetectorMetadata(name: "git", homepage: "https://git-scm.com/", docsURL: "https://example.test/git")
    ])
}

@Test func hardenedToolsFindsAutomicVaultStubs() throws {
    let directory = temporaryDirectory()
    try """
    #!/bin/sh
    # Automic Vault hardened stub
    exec /usr/local/bin/av stub-exec 'aws' '/opt/homebrew/bin/aws' "$@"
    """.write(to: directory.appendingPathComponent("aws"), atomically: true, encoding: .utf8)
    try "not a stub".write(to: directory.appendingPathComponent("plain"), atomically: true, encoding: .utf8)

    let tools = loadHardenedTools(in: directory, ghCLIURL: nil)

    #expect(tools.count == 1)
    #expect(tools.first?.name == "aws")
    #expect(tools.first?.stubPath.hasSuffix("/aws") == true)
    #expect(tools.first?.targetPath == "/opt/homebrew/bin/aws")
}

@Test func hardenedToolsFindsLegacyAWSInjectStubAndGHTap() throws {
    let directory = temporaryDirectory()
    try """
    #!/usr/local/bin/av inject +AWS_ACCESS_KEY_ID +AWS_SECRET_ACCESS_KEY /bin/zsh
    exec /opt/homebrew/bin/aws-vault exec default -- /opt/homebrew/bin/aws "$@"
    """.write(to: directory.appendingPathComponent("aws"), atomically: true, encoding: .utf8)
    let gh = directory.appendingPathComponent("gh")
    try "".write(to: gh, atomically: true, encoding: .utf8)
    try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: gh.path)

    let tools = loadHardenedTools(in: directory, ghCLIURL: gh)

    #expect(tools.map(\.name) == ["aws", "gh-cli"])
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
        )
    ], service: service) == errSecSuccess)

    #expect(loadSecretGates(service: service) == [
        SecretGate(scriptPath: "/tmp/deploy", keys: ["A", "B"], target: "/bin/echo", approvedApps: ["com.example.app", "com.other.app"])
    ])
}

@Test func storedSecretsListNamesOnlyAndDelete() throws {
    let service = "com.automicvault.tests.\(UUID().uuidString)"
    #expect(saveStoredSecret(account: "API_TOKEN", value: "secret", service: service) == errSecSuccess)
    defer { _ = deleteStoredSecret(account: "API_TOKEN", service: service) }

    #expect(loadStoredSecrets(service: service) == [StoredSecret(account: "API_TOKEN")])
    #expect(deleteStoredSecret(account: "API_TOKEN", service: service) == errSecSuccess)
    #expect(loadStoredSecrets(service: service).isEmpty)
}

private func temporaryDirectory() -> URL {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("av-menubar-tests-\(UUID().uuidString)", isDirectory: true)
    try! FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
    return url
}
