import Foundation
import Security
import Testing
@testable import MenubarHelperCore

@Test func launcherBundleNamesRejectPathsAndControls() {
    #expect(launcherBundleDisplayName(from: "  Acme CLI  ") == "Acme CLI")
    #expect(launcherBundleDisplayName(from: "../Acme") == nil)
    #expect(launcherBundleDisplayName(from: "Acme/CLI") == nil)
    #expect(launcherBundleDisplayName(from: "Acme\nCLI") == nil)
}

@Test func launcherBundlePayloadSnapshotAcceptsOneMachOAndResolvesItsSymlink() throws {
    let directory = FileManager.default.temporaryDirectory
        .appendingPathComponent(UUID().uuidString, isDirectory: true)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: false)
    defer { try? FileManager.default.removeItem(at: directory) }
    let source = directory.appendingPathComponent("source")
    let link = directory.appendingPathComponent("link")
    let destination = directory.appendingPathComponent("payload")
    try Data([0xcf, 0xfa, 0xed, 0xfe, 1, 2, 3, 4]).write(to: source)
    try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: source.path)
    try FileManager.default.createSymbolicLink(at: link, withDestinationURL: source)

    let snapshot = try copyLauncherBundlePayload(from: link, to: destination)
    let copiedSHA256 = try sha256OfRegularFile(at: destination)

    #expect(snapshot.sourcePath == source.path)
    #expect(snapshot.byteCount == 8)
    #expect(snapshot.sourceSHA256 == copiedSHA256)
    #expect(FileManager.default.isExecutableFile(atPath: destination.path))
}

@Test func launcherBundlePayloadSnapshotRejectsScripts() throws {
    let directory = FileManager.default.temporaryDirectory
        .appendingPathComponent(UUID().uuidString, isDirectory: true)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: false)
    defer { try? FileManager.default.removeItem(at: directory) }
    let source = directory.appendingPathComponent("script")
    let destination = directory.appendingPathComponent("payload")
    try Data("#!/bin/sh\nexit 0\n".utf8).write(to: source)
    try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: source.path)

    #expect(throws: LauncherBundlePayloadError.notRegularMachO) {
        try copyLauncherBundlePayload(from: source, to: destination)
    }
    #expect(!FileManager.default.fileExists(atPath: destination.path))
}

@Test func launcherBundleEnrollmentReplacementIsOneKeychainRecordChange() throws {
    guard launcherBundleKeychainTestsAvailable() else { return }
    let service = "com.automicvault.tests.launcher-bundles.\(UUID().uuidString)"
    let account = "LauncherBundles"
    defer { _ = deleteStoredSecret(account: account, service: service) }
    let old = launcherBundleEnrollment(name: "Acme", generation: UUID())
    let replacement = launcherBundleEnrollment(name: "Acme", generation: UUID())
    #expect(saveLauncherBundleEnrollment(old, service: service, account: account) == errSecSuccess)

    #expect(saveLauncherBundleEnrollment(
        replacement,
        replacing: old.generation,
        service: service,
        account: account
    ) == errSecSuccess)
    #expect(loadLauncherBundleEnrollments(service: service, account: account) == [replacement])
}

@Test func corruptLauncherBundleEnrollmentFailsClosed() {
    guard launcherBundleKeychainTestsAvailable() else { return }
    let service = "com.automicvault.tests.launcher-bundles.\(UUID().uuidString)"
    let account = "LauncherBundles"
    defer { _ = deleteStoredSecret(account: account, service: service) }
    #expect(saveKeychainData(
        Data("not json".utf8),
        service: service,
        account: account,
        accessibility: .afterFirstUnlock
    ) == errSecSuccess)

    #expect(loadLauncherBundleEnrollmentsResult(service: service, account: account) == .failure(errSecDecode))
    #expect(saveLauncherBundleEnrollment(
        launcherBundleEnrollment(name: "Acme", generation: UUID()),
        service: service,
        account: account
    ) == errSecDecode)
}

private func launcherBundleKeychainTestsAvailable() -> Bool {
    let service = "com.automicvault.tests.launcher-bundle-probe.\(UUID().uuidString)"
    let status = saveStoredSecret(account: "PROBE", value: "secret", service: service)
    defer { _ = deleteStoredSecret(account: "PROBE", service: service) }
    return status != errSecMissingEntitlement
}

private func launcherBundleEnrollment(
    name: String,
    generation: UUID
) -> LauncherBundleEnrollment {
    let identifier = "\(launcherBundleIdentifierPrefix)\(generation.uuidString.lowercased())"
    return LauncherBundleEnrollment(
        generation: generation,
        displayName: name,
        bundleIdentifier: identifier,
        bundlePath: "/tmp/\(name).app",
        launcherIdentifier: "\(identifier).runner.aabb",
        launcherRequirement: "identifier \"\(identifier)\"",
        bundleCodeIdentifiers: [Data([1])],
        launcherCodeIdentifiers: [Data([2])],
        payloadCodeIdentifiers: [Data([3])],
        sourceSHA256: String(repeating: "f", count: 64),
        payloadSHA256: String(repeating: "0", count: 64),
        runtimeRequirement: .hardened,
        signingKind: .adHoc,
        signingIdentity: nil,
        createdAt: Date(timeIntervalSince1970: 1)
    )
}
