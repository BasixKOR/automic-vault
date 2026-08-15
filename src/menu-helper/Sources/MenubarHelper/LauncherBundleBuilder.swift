import AppKit
import Foundation
import MenubarHelperCore
import Security

struct LauncherBundleOptions: Sendable {
    let sourceURL: URL
    let displayName: String
    let commandName: String
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

struct LauncherBundleCandidate: Sendable {
    let workDirectory: URL
    let stagedURL: URL
    let finalURL: URL
    let enrollment: LauncherBundleEnrollment
    let replacedEnrollment: LauncherBundleEnrollment?
}

enum LauncherBundleCreationError: Error, LocalizedError {
    case runnerUnavailable
    case iconUnavailable
    case unsafeManagedDirectory
    case cliUnavailable
    case invalidCommandName
    case commandOccupied
    case destinationOccupied
    case invalidSigningIdentity
    case commandFailed(String)
    case invalidGeneratedCode
    case enrollmentFailed(OSStatus)

    var errorDescription: String? {
        switch self {
        case .runnerUnavailable: "The bundled Launcher Bundle runner is unavailable"
        case .iconUnavailable: "The bundled Launcher Bundle icon is unavailable"
        case .unsafeManagedDirectory: "The Launcher Bundle staging directory is unsafe"
        case .cliUnavailable: "Install or update the av CLI before installing a Launcher Bundle"
        case .invalidCommandName: "Choose a command name without spaces or path separators"
        case .destinationOccupied: "A file already occupies the Launcher Bundle destination"
        case .commandOccupied: "A different file already occupies the Launcher Bundle command path"
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

func prepareLauncherBundleCandidate(_ options: LauncherBundleOptions) throws -> LauncherBundleCandidate {
    guard let displayName = launcherBundleDisplayName(from: options.displayName)
    else { throw LauncherBundlePayloadError.notRegularMachO }
    guard let commandName = launcherBundleCommandName(from: options.commandName)
    else { throw LauncherBundleCreationError.invalidCommandName }
    guard options.signingKind == .adHoc
        || options.signingIdentity?.hasPrefix("Developer ID Application:") == true
    else { throw LauncherBundleCreationError.invalidSigningIdentity }
    guard let runnerURL = Bundle.main.url(forResource: "AutomicVaultLauncher", withExtension: nil)
    else { throw LauncherBundleCreationError.runnerUnavailable }
    guard let iconURL = Bundle.main.url(forResource: "LauncherBundleIcon", withExtension: "icns")
    else { throw LauncherBundleCreationError.iconUnavailable }

    let manager = FileManager.default
    let managed = launcherBundleManagedDirectory()
    let staging = manager.temporaryDirectory.appendingPathComponent(
        "Automic Vault Launcher Bundles",
        isDirectory: true
    )
    try prepareLauncherBundleStagingDirectory(staging)
    let generation = UUID()
    let identifier = launcherBundleIdentifierPrefix + generation.uuidString.lowercased()
    let finalURL = managed.appendingPathComponent("\(displayName).app", isDirectory: true)
    let commandURL = launcherBundleCommandURL(named: commandName)
    try guardLauncherBundleCommandAvailable(
        commandURL,
        runner: finalURL.appendingPathComponent("Contents/MacOS/launcher")
    )
    let enrollments = loadLauncherBundleEnrollments()
    let destinationEnrollment = enrollments.first {
        URL(fileURLWithPath: $0.bundlePath).standardizedFileURL == finalURL.standardizedFileURL
    }
    if manager.fileExists(atPath: finalURL.path), destinationEnrollment == nil {
        throw LauncherBundleCreationError.destinationOccupied
    }
    let oldEnrollment = destinationEnrollment
        ?? enrollments.first { $0.displayName == displayName }

    let work = staging.appendingPathComponent(
        ".creating-\(generation.uuidString.lowercased())",
        isDirectory: true
    )
    try manager.createDirectory(
        at: work,
        withIntermediateDirectories: false,
        attributes: [.posixPermissions: 0o700]
    )
    var keepWork = false
    defer { if !keepWork { try? manager.removeItem(at: work) } }
    let appURL = work.appendingPathComponent("bundle.app", isDirectory: true)
    let contents = appURL.appendingPathComponent("Contents", isDirectory: true)
    let macOS = contents.appendingPathComponent("MacOS", isDirectory: true)
    let resources = contents.appendingPathComponent("Resources", isDirectory: true)
    try manager.createDirectory(at: macOS, withIntermediateDirectories: true)
    try manager.createDirectory(at: resources, withIntermediateDirectories: true)
    let launcherURL = macOS.appendingPathComponent("launcher")
    let payloadURL = resources.appendingPathComponent(launcherBundlePayloadName)
    try manager.copyItem(
        at: iconURL,
        to: resources.appendingPathComponent("LauncherBundleIcon.icns")
    )
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
    let launcherIdentifier = identifier
    try patchLauncherBundleRunner(at: launcherURL, payloadCodeIdentifiers: payload.codeIdentifiers)

    let info: [String: Any] = [
        kCFBundleIdentifierKey as String: identifier,
        kCFBundleExecutableKey as String: "launcher",
        kCFBundleNameKey as String: displayName,
        "CFBundleDisplayName": displayName,
        "CFBundleIconFile": "LauncherBundleIcon",
        "CFBundlePackageType": "APPL",
        "CFBundleVersion": "1",
        "CFBundleShortVersionString": "1.0",
        launcherBundleGenerationInfoKey: generation.uuidString.lowercased(),
        launcherBundlePayloadSHA256InfoKey: payloadSHA256,
        launcherBundleCommandNameInfoKey: commandName,
    ]
    try PropertyListSerialization.data(fromPropertyList: info, format: .xml, options: 0)
        .write(to: contents.appendingPathComponent("Info.plist"), options: .atomic)
    try signLauncherBundleCode(
        appURL,
        identity: identity,
        identifier: identifier,
        entitlements: nil
    )

    let bundle = try launcherBundleCodeEvidence(at: appURL, bundle: true)
    let launcher = try launcherBundleCodeEvidence(at: launcherURL)
    guard bundle.identifier == identifier,
          launcher.identifier == launcherIdentifier,
          launcher.runtimeProtection == .hardened,
          bundle.isAdHoc == (options.signingKind == .adHoc),
          launcher.isAdHoc == bundle.isAdHoc,
          payload.isAdHoc == bundle.isAdHoc
    else { throw LauncherBundleCreationError.invalidGeneratedCode }

    let enrollment = LauncherBundleEnrollment(
        generation: generation,
        displayName: displayName,
        commandName: commandName,
        bundleIdentifier: identifier,
        bundlePath: finalURL.path,
        launcherIdentifier: launcherIdentifier,
        launcherRequirement: bundle.designatedRequirement,
        bundleCodeIdentifiers: bundle.codeIdentifiers,
        launcherCodeIdentifiers: launcher.codeIdentifiers,
        payloadCodeIdentifiers: payload.codeIdentifiers,
        sourceSHA256: source.sourceSHA256,
        payloadSHA256: payloadSHA256,
        payloadEntitlements: entitlements.keys.sorted(),
        runtimeRequirement: runtimeRequirement,
        signingKind: options.signingKind,
        signingIdentity: options.signingIdentity,
        createdAt: Date()
    )
    keepWork = true
    return LauncherBundleCandidate(
        workDirectory: work,
        stagedURL: appURL,
        finalURL: finalURL,
        enrollment: enrollment,
        replacedEnrollment: oldEnrollment
    )
}

func installLauncherBundleCandidate(
    _ candidate: LauncherBundleCandidate
) throws -> LauncherBundleCreation {
    let manager = FileManager.default
    defer { try? manager.removeItem(at: candidate.workDirectory) }
    let enrollment = candidate.enrollment
    guard let commandName = enrollment.commandName else {
        throw LauncherBundleCreationError.invalidCommandName
    }
    let generation = enrollment.generation.uuidString.lowercased()
    try runPrivilegedLauncherBundleOperation([
        "__install-launcher-bundle",
        candidate.stagedURL.path,
        enrollment.displayName,
        commandName,
        generation,
    ])
    do {
        let oldEnrollment = candidate.replacedEnrollment
        let status = saveLauncherBundleEnrollment(
            enrollment,
            replacing: oldEnrollment?.generation
        )
        guard status == errSecSuccess else {
            throw LauncherBundleCreationError.enrollmentFailed(status)
        }
        do {
            _ = try verifyLauncherBundle(
                at: candidate.finalURL,
                liveLauncherIdentifier: enrollment.launcherIdentifier,
                liveLauncherCodeIdentifier: enrollment.launcherCodeIdentifiers[0],
                liveRuntimeProtection: .hardened
            )
        } catch {
            _ = removeLauncherBundleEnrollment(generation: enrollment.generation)
            if let oldEnrollment { _ = saveLauncherBundleEnrollment(oldEnrollment) }
            throw error
        }
    } catch {
        _ = removeLauncherBundleEnrollment(generation: enrollment.generation)
        if let oldEnrollment = candidate.replacedEnrollment {
            _ = saveLauncherBundleEnrollment(oldEnrollment)
        }
        do {
            try runPrivilegedLauncherBundleOperation([
                "__rollback-launcher-bundle",
                enrollment.displayName,
                commandName,
                generation,
            ])
        } catch let rollbackError {
            throw LauncherBundleCreationError.commandFailed(
                "\(error.localizedDescription). Rollback also failed: \(rollbackError.localizedDescription)"
            )
        }
        throw error
    }

    var warnings: [String] = []
    if let oldEnrollment = candidate.replacedEnrollment {
        let cleanup = removeLauncherBundleAuthorization(requirement: oldEnrollment.launcherRequirement)
        if cleanup != errSecSuccess {
            warnings.append("Old authorization rules could not be removed: \(cleanup)")
        }
    }
    do {
        try runPrivilegedLauncherBundleOperation([
            "__finish-launcher-bundle",
            enrollment.displayName,
            generation,
            manager.homeDirectoryForCurrentUser.appendingPathComponent(".Trash").path,
        ])
    } catch {
        warnings.append("The old bundle could not be moved to Trash: \(error.localizedDescription)")
    }
    if let oldEnrollment = candidate.replacedEnrollment,
       oldEnrollment.bundlePath != enrollment.bundlePath {
        do {
            let oldURL = URL(fileURLWithPath: oldEnrollment.bundlePath)
            if manager.fileExists(atPath: oldURL.path) {
                try manager.trashItem(at: oldURL, resultingItemURL: nil)
            }
        } catch {
            warnings.append("The old bundle could not be moved to Trash: \(error.localizedDescription)")
        }
    }
    return LauncherBundleCreation(
        enrollment: enrollment,
        cleanupWarning: warnings.isEmpty ? nil : warnings.joined(separator: " ")
    )
}

func removeInstalledLauncherBundle(_ enrollment: LauncherBundleEnrollment) throws {
    guard let commandName = enrollment.commandName else {
        try FileManager.default.trashItem(
            at: URL(fileURLWithPath: enrollment.bundlePath),
            resultingItemURL: nil
        )
        return
    }
    try runPrivilegedLauncherBundleOperation([
        "__remove-launcher-bundle",
        enrollment.displayName,
        commandName,
        enrollment.generation.uuidString.lowercased(),
        FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent(".Trash").path,
    ])
}

func discardLauncherBundleCandidate(_ candidate: LauncherBundleCandidate) {
    try? FileManager.default.removeItem(at: candidate.workDirectory)
}

private func prepareLauncherBundleStagingDirectory(_ url: URL) throws {
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

private func guardLauncherBundleCommandAvailable(_ command: URL, runner: URL) throws {
    var metadata = stat()
    guard lstat(command.path, &metadata) == 0 else {
        if errno == ENOENT { return }
        throw LauncherBundleCreationError.commandOccupied
    }
    guard metadata.st_mode & S_IFMT == S_IFLNK,
          metadata.st_uid == 0,
          let target = try? FileManager.default.destinationOfSymbolicLink(atPath: command.path),
          target == runner.path
    else { throw LauncherBundleCreationError.commandOccupied }
}

private func runPrivilegedLauncherBundleOperation(_ arguments: [String]) throws {
    guard currentCLIInstallState() == .current else {
        throw LauncherBundleCreationError.cliUnavailable
    }
    let script = """
    on run argv
        set commandText to quoted form of item 1 of argv
        repeat with argumentIndex from 2 to count argv
            set commandText to commandText & " " & quoted form of item argumentIndex of argv
        end repeat
        do shell script commandText with administrator privileges
    end run
    """
    _ = try runLauncherBundleCommand(
        executable: "/usr/bin/osascript",
        arguments: ["-e", script, installedAVCLIPath] + arguments
    )
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

private func patchLauncherBundleRunner(
    at url: URL,
    payloadCodeIdentifiers: [Data]
) throws {
    let marker = Data("AVLB_PAYLOAD_CDHASHES:".utf8)
    let value = Data(payloadCodeIdentifiers.map(\.hexString).joined(separator: ",").utf8)
    guard value.count < 513 - marker.count,
          var runner = try? Data(contentsOf: url)
    else { throw LauncherBundleCreationError.invalidGeneratedCode }
    var searchStart = runner.startIndex
    var offsets: [Int] = []
    while searchStart < runner.endIndex,
          let range = runner.range(of: marker, in: searchStart..<runner.endIndex) {
        offsets.append(range.upperBound)
        searchStart = range.upperBound
    }
    guard !offsets.isEmpty else { throw LauncherBundleCreationError.invalidGeneratedCode }
    for offset in offsets {
        runner.replaceSubrange(offset..<(offset + value.count), with: value)
    }
    try runner.write(to: url)
    try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: url.path)
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
