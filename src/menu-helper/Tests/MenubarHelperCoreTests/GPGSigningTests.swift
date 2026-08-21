import Foundation
import Security
import Testing
@testable import MenubarHelperCore

@Test func launcherListSelectsTheAlternateSigningCredential() {
    let launcher = BlessedScriptLauncher(bundleIdentifier: "com.openai.codex", requirement: "codex")
    let configuration = GPGSigningConfiguration(alternateKeyLaunchers: [launcher])

    #expect(gpgSigningSecretNames(
        configuration: configuration,
        launcherRequirements: ["codex"]
    ) == [gpgAlternatePrivateKeySecretName, gpgAlternatePassphraseSecretName])
    #expect(gpgSigningSecretNames(
        configuration: configuration,
        launcherRequirements: ["terminal"]
    ) == [gpgDefaultPrivateKeySecretName, gpgDefaultPassphraseSecretName])
}

@Test func gitConfigurationPreservesAnAppPathContainingSpaces() throws {
    let directory = FileManager.default.temporaryDirectory
        .appendingPathComponent("Automic Vault Tests \(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: directory) }
    let bpb = directory.appendingPathComponent("bpb")
    try Data("#!/bin/sh\nexit 0\n".utf8).write(to: bpb)
    try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: bpb.path)
    let gitConfig = directory.appendingPathComponent("gitconfig")

    let environment = ProcessInfo.processInfo.environment.merging(
        ["GIT_CONFIG_GLOBAL": gitConfig.path],
        uniquingKeysWith: { _, configured in configured }
    )
    try configureGitForGPGSigning(bpbURL: bpb, environment: environment)

    let configured = try String(contentsOf: gitConfig, encoding: .utf8)
    #expect(configured.contains(bpb.path))
    #expect(configured.contains("gpgSign = true"))
    #expect(configured.contains("format = openpgp"))
}
