import AppKit
import Foundation
import MenubarHelperCore
import Security

struct LauncherBundleOptions: Sendable {
    let sourceURL: URL
    let displayName: String
    let signingKind: LauncherBundleSigningKind
    let signingIdentity: String?
    let allowJIT: Bool
    let allowUnsignedExecutableMemory: Bool
    let disableLibraryValidation: Bool
}

struct LauncherBundleCreation: Sendable {
    let enrollment: LauncherBundleEnrollment
    let cleanupWarning: String?
}

enum LauncherBundleCreationError: Error, LocalizedError {
    case runnerUnavailable
    case unsafeManagedDirectory
    case destinationOccupied
    case invalidSigningIdentity
    case commandFailed(String)
    case invalidGeneratedCode
    case enrollmentFailed(OSStatus)

    var errorDescription: String? {
        switch self {
        case .runnerUnavailable: "The bundled Launcher Bundle runner is unavailable"
        case .unsafeManagedDirectory: "~/Applications/Automic Vault is not a safe managed directory"
        case .destinationOccupied: "A file already occupies the Launcher Bundle destination"
        case .invalidSigningIdentity: "Choose a valid Developer ID Application identity"
        case .commandFailed(let message): message
        case .invalidGeneratedCode: "The generated Launcher Bundle did not pass verification"
        case .enrollmentFailed(let status): "Could not enroll the Launcher Bundle: \(status)"
        }
    }
}

func developerIDApplicationIdentities() -> [String] {
    guard let output = try? runLauncherBundleCommand(
        executable: "/usr/bin/security",
        arguments: ["find-identity", "-v", "-p", "codesigning"]
    ) else { return [] }
    return output.split(separator: "\n").compactMap { line in
        guard let first = line.firstIndex(of: "\""),
              let last = line.lastIndex(of: "\""),
              first < last
        else { return nil }
        let identity = String(line[line.index(after: first)..<last])
        return identity.hasPrefix("Developer ID Application:") ? identity : nil
    }
}

func buildLauncherBundle(_ options: LauncherBundleOptions) throws -> LauncherBundleCreation {
    guard let displayName = launcherBundleDisplayName(from: options.displayName)
    else { throw LauncherBundlePayloadError.notRegularMachO }
    guard options.signingKind == .adHoc
        || options.signingIdentity?.hasPrefix("Developer ID Application:") == true
    else { throw LauncherBundleCreationError.invalidSigningIdentity }
    guard let runnerURL = Bundle.main.url(forResource: "AutomicVaultLauncher", withExtension: nil)
    else { throw LauncherBundleCreationError.runnerUnavailable }

    let manager = FileManager.default
    let managed = launcherBundleManagedDirectory()
    try prepareLauncherBundleManagedDirectory(managed)
    let generation = UUID()
    let identifier = launcherBundleIdentifierPrefix + generation.uuidString.lowercased()
    let finalURL = managed.appendingPathComponent("\(displayName).app", isDirectory: true)
    let oldEnrollment = loadLauncherBundleEnrollments().first {
        URL(fileURLWithPath: $0.bundlePath).standardizedFileURL == finalURL.standardizedFileURL
            || $0.displayName == displayName
    }
    if manager.fileExists(atPath: finalURL.path), oldEnrollment == nil {
        throw LauncherBundleCreationError.destinationOccupied
    }

    let work = managed.appendingPathComponent(".creating-\(generation.uuidString)", isDirectory: true)
    try manager.createDirectory(
        at: work,
        withIntermediateDirectories: false,
        attributes: [.posixPermissions: 0o700]
    )
    defer { try? manager.removeItem(at: work) }
    let appURL = work.appendingPathComponent("bundle.app", isDirectory: true)
    let contents = appURL.appendingPathComponent("Contents", isDirectory: true)
    let macOS = contents.appendingPathComponent("MacOS", isDirectory: true)
    let resources = contents.appendingPathComponent("Resources", isDirectory: true)
    try manager.createDirectory(at: macOS, withIntermediateDirectories: true)
    try manager.createDirectory(at: resources, withIntermediateDirectories: true)
    let launcherURL = macOS.appendingPathComponent("launcher")
    let payloadURL = resources.appendingPathComponent(launcherBundlePayloadName)
    try manager.copyItem(at: runnerURL, to: launcherURL)
    let source = try copyLauncherBundlePayload(from: options.sourceURL, to: payloadURL)

    let identity = options.signingKind == .adHoc ? "-" : options.signingIdentity!
    let entitlementsURL = work.appendingPathComponent("payload-entitlements.plist")
    let entitlements: [String: Bool] = [
        "com.apple.security.cs.allow-jit": options.allowJIT,
        "com.apple.security.cs.allow-unsigned-executable-memory": options.allowUnsignedExecutableMemory,
        "com.apple.security.cs.disable-library-validation": options.disableLibraryValidation,
    ].filter(\.value)
    if !entitlements.isEmpty {
        let data = try PropertyListSerialization.data(
            fromPropertyList: entitlements,
            format: .xml,
            options: 0
        )
        try data.write(to: entitlementsURL, options: .atomic)
    }
    try signLauncherBundleCode(
        payloadURL,
        identity: identity,
        identifier: "\(identifier).payload",
        entitlements: entitlements.isEmpty ? nil : entitlementsURL
    )
    let payload = try launcherBundleCodeEvidence(at: payloadURL)
    guard let runtimeRequirement = payload.runtimeProtection.secretGateAdmissionRequirement
    else { throw LauncherBundleCreationError.invalidGeneratedCode }
    let payloadSHA256 = try sha256OfRegularFile(at: payloadURL)
    let launcherIdentifier = "\(identifier).runner.\(payload.codeIdentifiers.map(\.hexString).joined(separator: "."))"
    try signLauncherBundleCode(
        launcherURL,
        identity: identity,
        identifier: launcherIdentifier,
        entitlements: nil
    )

    let info: [String: Any] = [
        kCFBundleIdentifierKey as String: identifier,
        kCFBundleExecutableKey as String: "launcher",
        kCFBundleNameKey as String: displayName,
        "CFBundleDisplayName": displayName,
        "CFBundlePackageType": "APPL",
        "CFBundleVersion": "1",
        "CFBundleShortVersionString": "1.0",
        launcherBundleGenerationInfoKey: generation.uuidString.lowercased(),
        launcherBundlePayloadSHA256InfoKey: payloadSHA256,
    ]
    try PropertyListSerialization.data(fromPropertyList: info, format: .xml, options: 0)
        .write(to: contents.appendingPathComponent("Info.plist"), options: .atomic)
    try signLauncherBundleCode(appURL, identity: identity, identifier: identifier, entitlements: nil)

    let bundle = try launcherBundleCodeEvidence(at: appURL, bundle: true)
    let launcher = try launcherBundleCodeEvidence(at: launcherURL)
    guard bundle.identifier == identifier,
          launcher.identifier == launcherIdentifier,
          launcher.runtimeProtection == .hardened,
          bundle.isAdHoc == (options.signingKind == .adHoc),
          launcher.isAdHoc == bundle.isAdHoc,
          payload.isAdHoc == bundle.isAdHoc
    else { throw LauncherBundleCreationError.invalidGeneratedCode }

    let backupURL = work.appendingPathComponent("replaced.app", isDirectory: true)
    if manager.fileExists(atPath: finalURL.path) {
        try manager.moveItem(at: finalURL, to: backupURL)
    }
    do {
        try manager.moveItem(at: appURL, to: finalURL)
        let enrollment = LauncherBundleEnrollment(
            generation: generation,
            displayName: displayName,
            bundleIdentifier: identifier,
            bundlePath: finalURL.path,
            launcherIdentifier: launcherIdentifier,
            launcherRequirement: bundle.designatedRequirement,
            bundleCodeIdentifiers: bundle.codeIdentifiers,
            launcherCodeIdentifiers: launcher.codeIdentifiers,
            payloadCodeIdentifiers: payload.codeIdentifiers,
            sourceSHA256: source.sourceSHA256,
            payloadSHA256: payloadSHA256,
            runtimeRequirement: runtimeRequirement,
            signingKind: options.signingKind,
            signingIdentity: options.signingIdentity,
            createdAt: Date()
        )
        let status = saveLauncherBundleEnrollment(enrollment, replacing: oldEnrollment?.generation)
        guard status == errSecSuccess else {
            throw LauncherBundleCreationError.enrollmentFailed(status)
        }
        do {
            _ = try verifyLauncherBundle(
                at: finalURL,
                liveLauncherIdentifier: launcherIdentifier,
                liveLauncherCodeIdentifier: launcher.codeIdentifiers[0],
                liveRuntimeProtection: .hardened
            )
        } catch {
            _ = removeLauncherBundleEnrollment(generation: generation)
            if let oldEnrollment { _ = saveLauncherBundleEnrollment(oldEnrollment) }
            throw error
        }
        var cleanupWarning: String?
        if let oldEnrollment {
            let cleanup = removeLauncherBundleAuthorization(
                requirement: oldEnrollment.launcherRequirement
            )
            if cleanup != errSecSuccess {
                cleanupWarning = "Old authorization rules could not be removed: \(cleanup)"
            }
            do {
                try manager.trashItem(at: backupURL, resultingItemURL: nil)
            } catch {
                cleanupWarning = [cleanupWarning, "The old bundle could not be moved to Trash."]
                    .compactMap(\.self).joined(separator: " ")
            }
        }
        return LauncherBundleCreation(enrollment: enrollment, cleanupWarning: cleanupWarning)
    } catch {
        if manager.fileExists(atPath: finalURL.path) { try? manager.removeItem(at: finalURL) }
        if manager.fileExists(atPath: backupURL.path) { try? manager.moveItem(at: backupURL, to: finalURL) }
        throw error
    }
}

private func prepareLauncherBundleManagedDirectory(_ url: URL) throws {
    try FileManager.default.createDirectory(
        at: url,
        withIntermediateDirectories: true,
        attributes: [.posixPermissions: 0o700]
    )
    var metadata = stat()
    guard lstat(url.path, &metadata) == 0,
          metadata.st_mode & S_IFMT == S_IFDIR,
          metadata.st_uid == getuid(),
          metadata.st_mode & 0o022 == 0
    else { throw LauncherBundleCreationError.unsafeManagedDirectory }
}

private func signLauncherBundleCode(
    _ url: URL,
    identity: String,
    identifier: String,
    entitlements: URL?
) throws {
    var arguments = ["--force", "--sign", identity, "--options", "runtime"]
    if identity != "-" { arguments.append("--timestamp") }
    arguments += ["--identifier", identifier]
    if let entitlements { arguments += ["--entitlements", entitlements.path] }
    arguments.append(url.path)
    _ = try runLauncherBundleCommand(executable: "/usr/bin/codesign", arguments: arguments)
}

@discardableResult
private func runLauncherBundleCommand(executable: String, arguments: [String]) throws -> String {
    let process = Process()
    let output = Pipe()
    process.executableURL = URL(fileURLWithPath: executable)
    process.arguments = arguments
    process.standardOutput = output
    process.standardError = output
    try process.run()
    process.waitUntilExit()
    let message = String(
        decoding: output.fileHandleForReading.readDataToEndOfFile(),
        as: UTF8.self
    ).trimmingCharacters(in: .whitespacesAndNewlines)
    guard process.terminationStatus == 0 else {
        throw LauncherBundleCreationError.commandFailed(
            message.isEmpty ? "\(URL(fileURLWithPath: executable).lastPathComponent) failed" : message
        )
    }
    return message
}

private extension Data {
    var hexString: String { map { String(format: "%02x", $0) }.joined() }
}
