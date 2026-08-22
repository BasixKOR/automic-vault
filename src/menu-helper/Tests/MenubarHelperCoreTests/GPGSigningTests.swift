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
    let program = directory.appendingPathComponent("av-gpg")
    try Data("#!/bin/sh\nexit 0\n".utf8).write(to: program)
    try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: program.path)
    let gitConfig = directory.appendingPathComponent("gitconfig")

    let environment = ProcessInfo.processInfo.environment.merging(
        ["GIT_CONFIG_GLOBAL": gitConfig.path],
        uniquingKeysWith: { _, configured in configured }
    )
    try configureGitForGPGSigning(programURL: program, environment: environment)

    let configured = try String(contentsOf: gitConfig, encoding: .utf8)
    #expect(configured.contains(program.path))
    #expect(configured.contains("gpgSign = true"))
    #expect(configured.contains("format = openpgp"))
}

@Test func bundledExecutableResolutionNeverFallsBackOutsideTheApp() throws {
    let directory = FileManager.default.temporaryDirectory
        .appendingPathComponent("Automic Vault Bundle Tests \(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: directory) }
    let mainExecutable = directory.appendingPathComponent("AutomicVaultMenubar")
    let bundledAV = directory.appendingPathComponent("av")
    try Data().write(to: mainExecutable)

    #expect(throws: GPGSigningConfigurationError.self) {
        try bundledExecutableURL(named: "av", beside: mainExecutable)
    }

    try Data().write(to: bundledAV)
    try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: bundledAV.path)
    #expect(try bundledExecutableURL(named: "av", beside: mainExecutable) == bundledAV)
}

@Test func failedPrivateKeySaveRemovesANewPassphrase() {
    var values: [String: String] = [:]
    let status = saveGPGSigningCredential(
        privateKey: "private",
        passphrase: "new-passphrase",
        alternate: false,
        load: { values[$0].map(GPGStoredValueLoad.value) ?? .missing },
        save: { account, value in
            if account == gpgDefaultPrivateKeySecretName { return errSecAuthFailed }
            values[account] = value
            return errSecSuccess
        },
        delete: { account in
            values.removeValue(forKey: account)
            return errSecSuccess
        }
    )

    #expect(status == errSecAuthFailed)
    #expect(values[gpgDefaultPassphraseSecretName] == nil)
    #expect(values[gpgDefaultPrivateKeySecretName] == nil)
}

@Test func failedPrivateKeyReplacementRestoresThePreviousPassphrase() {
    var values = [gpgDefaultPassphraseSecretName: "old-passphrase"]
    let status = saveGPGSigningCredential(
        privateKey: "replacement",
        passphrase: "new-passphrase",
        alternate: false,
        load: { values[$0].map(GPGStoredValueLoad.value) ?? .missing },
        save: { account, value in
            if account == gpgDefaultPrivateKeySecretName { return errSecAuthFailed }
            values[account] = value
            return errSecSuccess
        },
        delete: { account in
            values.removeValue(forKey: account)
            return errSecSuccess
        }
    )

    #expect(status == errSecAuthFailed)
    #expect(values[gpgDefaultPassphraseSecretName] == "old-passphrase")
}
