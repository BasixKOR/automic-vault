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
        detectorFindings: try detectorFindings(from: data),
        hardenedTools: [],
        secretGates: []
    )

    #expect(snapshot.flaggedDetectorCount == 2)
}

@Test func hardenedToolsFindsAutomicVaultStubs() throws {
    let directory = temporaryDirectory()
    try """
    #!/bin/sh
    # Automic Vault hardened stub
    exec /usr/local/bin/av stub-exec 'aws' '/opt/homebrew/bin/aws' "$@"
    """.write(to: directory.appendingPathComponent("aws"), atomically: true, encoding: .utf8)
    try "not a stub".write(to: directory.appendingPathComponent("plain"), atomically: true, encoding: .utf8)

    let tools = loadHardenedTools(in: directory)

    #expect(tools.count == 1)
    #expect(tools.first?.name == "aws")
    #expect(tools.first?.stubPath.hasSuffix("/aws") == true)
    #expect(tools.first?.targetPath == "/opt/homebrew/bin/aws")
}

@Test func secretGatesDecodeRememberedApprovals() throws {
    let suite = "com.automicvault.tests.\(UUID().uuidString)"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }

    let data = try JSONEncoder().encode([
        TrustedScriptApproval(
            scriptPath: "/tmp/deploy",
            scriptChecksum: "abc",
            keys: ["B", "A"],
            target: "/bin/echo",
            replaceExistingEnv: true,
            allowMissingKeys: false,
            launcherRequirement: #"identifier "com.example.app""#
        )
    ])
    defaults.set(data, forKey: trustedScriptApprovalsDefaultsKey)

    #expect(loadSecretGates(defaults: defaults) == [
        SecretGate(scriptPath: "/tmp/deploy", keys: ["A", "B"], target: "/bin/echo")
    ])
}

private func temporaryDirectory() -> URL {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("av-menubar-tests-\(UUID().uuidString)", isDirectory: true)
    try! FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
    return url
}
