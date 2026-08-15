import CryptoKit
import Darwin
import Foundation
import Security

public let launcherBundlesKeychainService = "com.automicvault.launcher-bundles"
public let launcherBundlesKeychainAccount = "LauncherBundlesV1"
public let launcherBundleIdentifierPrefix = "com.automicvault.launcher-bundle."
public let launcherBundleGenerationInfoKey = "AVLauncherBundleGeneration"
public let launcherBundlePayloadSHA256InfoKey = "AVLauncherBundlePayloadSHA256"
public let launcherBundlePayloadName = "payload"
public let launcherBundleMaximumPayloadBytes: Int64 = 512 * 1024 * 1024

public enum LauncherBundleSigningKind: String, Codable, CaseIterable, Equatable, Sendable {
    case adHoc
    case developerID

    public var title: String {
        switch self {
        case .adHoc: "Ad Hoc"
        case .developerID: "Developer ID"
        }
    }
}

public struct LauncherBundleEnrollment: Codable, Equatable, Identifiable, Sendable {
    public let generation: UUID
    public let displayName: String
    public let bundleIdentifier: String
    public let bundlePath: String
    public let launcherIdentifier: String
    public let launcherRequirement: String
    public let bundleCodeIdentifiers: [Data]
    public let launcherCodeIdentifiers: [Data]
    public let payloadCodeIdentifiers: [Data]
    public let sourceSHA256: String
    public let payloadSHA256: String
    public let runtimeRequirement: LauncherRuntimeRequirement
    public let signingKind: LauncherBundleSigningKind
    public let signingIdentity: String?
    public let createdAt: Date

    public var id: UUID { generation }

    public init(
        generation: UUID,
        displayName: String,
        bundleIdentifier: String,
        bundlePath: String,
        launcherIdentifier: String,
        launcherRequirement: String,
        bundleCodeIdentifiers: [Data],
        launcherCodeIdentifiers: [Data],
        payloadCodeIdentifiers: [Data],
        sourceSHA256: String,
        payloadSHA256: String,
        runtimeRequirement: LauncherRuntimeRequirement,
        signingKind: LauncherBundleSigningKind,
        signingIdentity: String?,
        createdAt: Date = Date()
    ) {
        self.generation = generation
        self.displayName = displayName
        self.bundleIdentifier = bundleIdentifier
        self.bundlePath = bundlePath
        self.launcherIdentifier = launcherIdentifier
        self.launcherRequirement = launcherRequirement
        self.bundleCodeIdentifiers = normalizedCodeIdentifiers(bundleCodeIdentifiers)
        self.launcherCodeIdentifiers = normalizedCodeIdentifiers(launcherCodeIdentifiers)
        self.payloadCodeIdentifiers = normalizedCodeIdentifiers(payloadCodeIdentifiers)
        self.sourceSHA256 = sourceSHA256
        self.payloadSHA256 = payloadSHA256
        self.runtimeRequirement = runtimeRequirement
        self.signingKind = signingKind
        self.signingIdentity = signingIdentity
        self.createdAt = createdAt
    }
}

public enum LauncherBundleEnrollmentsLoad: Equatable, Sendable {
    case success([LauncherBundleEnrollment])
    case notFound
    case failure(OSStatus)
}

public func loadLauncherBundleEnrollments(
    service: String = launcherBundlesKeychainService,
    account: String = launcherBundlesKeychainAccount
) -> [LauncherBundleEnrollment] {
    guard case .success(let enrollments) = loadLauncherBundleEnrollmentsResult(
        service: service,
        account: account
    ) else { return [] }
    return enrollments.sorted(by: launcherBundleEnrollmentPrecedes)
}

public func loadLauncherBundleEnrollmentsResult(
    service: String = launcherBundlesKeychainService,
    account: String = launcherBundlesKeychainAccount
) -> LauncherBundleEnrollmentsLoad {
    switch loadKeychainDataResult(service: service, account: account) {
    case .notFound:
        return .notFound
    case .failure(let status):
        return .failure(status)
    case .success(let data):
        do {
            let records = try JSONDecoder().decode([LauncherBundleEnrollment].self, from: data)
            guard Set(records.map(\.generation)).count == records.count,
                  Set(records.map(\.bundleIdentifier)).count == records.count
            else { return .failure(errSecDecode) }
            return .success(records.sorted(by: launcherBundleEnrollmentPrecedes))
        } catch {
            return .failure(errSecDecode)
        }
    }
}

@discardableResult
public func saveLauncherBundleEnrollment(
    _ enrollment: LauncherBundleEnrollment,
    replacing generation: UUID? = nil,
    service: String = launcherBundlesKeychainService,
    account: String = launcherBundlesKeychainAccount
) -> OSStatus {
    var enrollments: [LauncherBundleEnrollment]
    switch loadLauncherBundleEnrollmentsResult(service: service, account: account) {
    case .success(let loaded): enrollments = loaded
    case .notFound: enrollments = []
    case .failure(let status): return status
    }
    if let generation { enrollments.removeAll { $0.generation == generation } }
    enrollments.removeAll {
        $0.generation == enrollment.generation || $0.bundleIdentifier == enrollment.bundleIdentifier
    }
    enrollments.append(enrollment)
    let sorted = enrollments.sorted(by: launcherBundleEnrollmentPrecedes)
    guard let data = try? JSONEncoder().encode(sorted) else { return errSecParam }
    let status = saveKeychainData(
        data,
        service: service,
        account: account,
        accessibility: .afterFirstUnlock
    )
    guard status == errSecSuccess else { return status }
    guard case .success(let stored) = loadLauncherBundleEnrollmentsResult(
        service: service,
        account: account
    ) else { return errSecDecode }
    return stored == sorted ? errSecSuccess : errSecDecode
}

@discardableResult
public func removeLauncherBundleEnrollment(
    generation: UUID,
    service: String = launcherBundlesKeychainService,
    account: String = launcherBundlesKeychainAccount
) -> OSStatus {
    let enrollments: [LauncherBundleEnrollment]
    switch loadLauncherBundleEnrollmentsResult(service: service, account: account) {
    case .success(let loaded): enrollments = loaded.filter { $0.generation != generation }
    case .notFound: return errSecSuccess
    case .failure(let status): return status
    }
    if enrollments.isEmpty {
        let status = deleteStoredSecret(account: account, service: service)
        return status == errSecItemNotFound ? errSecSuccess : status
    }
    guard let data = try? JSONEncoder().encode(enrollments) else { return errSecParam }
    return saveKeychainData(
        data,
        service: service,
        account: account,
        accessibility: .afterFirstUnlock
    )
}

@discardableResult
public func removeLauncherBundleAuthorization(
    requirement: String
) -> OSStatus {
    for status in [
        removeSecretGatePolicies(forLauncherRequirement: requirement),
        removeSecretNameAccess(forLauncherRequirement: requirement),
        removeDirectAccess(forLauncherRequirement: requirement),
        removeLauncherFromBlessedScripts(requirement: requirement),
    ] where status != errSecSuccess {
        return status
    }
    return errSecSuccess
}

public struct LauncherBundleCodeEvidence: Equatable, Sendable {
    public let identifier: String
    public let teamIdentifier: String?
    public let designatedRequirement: String
    public let codeIdentifiers: [Data]
    public let isAdHoc: Bool
    public let runtimeProtection: LauncherRuntimeProtection

    public init(
        identifier: String,
        teamIdentifier: String?,
        designatedRequirement: String,
        codeIdentifiers: [Data],
        isAdHoc: Bool,
        runtimeProtection: LauncherRuntimeProtection
    ) {
        self.identifier = identifier
        self.teamIdentifier = teamIdentifier
        self.designatedRequirement = designatedRequirement
        self.codeIdentifiers = normalizedCodeIdentifiers(codeIdentifiers)
        self.isAdHoc = isAdHoc
        self.runtimeProtection = runtimeProtection
    }
}

public enum LauncherBundleVerificationError: Error, Equatable, LocalizedError {
    case invalidBundle
    case enrollmentUnavailable
    case notEnrolled
    case identityMismatch
    case payloadMismatch
    case runtimeMismatch

    public var errorDescription: String? {
        switch self {
        case .invalidBundle: "Launcher Bundle integrity could not be verified"
        case .enrollmentUnavailable: "Launcher Bundle enrollment is unavailable"
        case .notEnrolled: "Launcher Bundle is not enrolled"
        case .identityMismatch: "Launcher Bundle signed identity changed"
        case .payloadMismatch: "Launcher Bundle payload changed"
        case .runtimeMismatch: "Launcher Bundle runtime protections changed"
        }
    }
}

public func verifyLauncherBundle(
    at appURL: URL,
    liveLauncherIdentifier: String,
    liveLauncherCodeIdentifier: Data,
    liveRuntimeProtection: LauncherRuntimeProtection,
    enrollments: LauncherBundleEnrollmentsLoad? = nil
) throws -> LauncherBundleEnrollment {
    let info = try launcherBundleInfo(at: appURL)
    guard info.bundleIdentifier.hasPrefix(launcherBundleIdentifierPrefix) else {
        throw LauncherBundleVerificationError.invalidBundle
    }
    let records = switch enrollments ?? loadLauncherBundleEnrollmentsResult() {
    case .success(let loaded): loaded
    case .notFound: throw LauncherBundleVerificationError.notEnrolled
    case .failure: throw LauncherBundleVerificationError.enrollmentUnavailable
    }
    guard let enrollment = records.first(where: {
        $0.generation == info.generation && $0.bundleIdentifier == info.bundleIdentifier
    }) else { throw LauncherBundleVerificationError.notEnrolled }

    let macOSURL = appURL.appendingPathComponent("Contents/MacOS", isDirectory: true)
    let launcherURL = macOSURL.appendingPathComponent(info.executable)
    let payloadURL = appURL.appendingPathComponent(
        "Contents/Resources/\(launcherBundlePayloadName)"
    )
    let bundle = try launcherBundleCodeEvidence(at: appURL, bundle: true)
    let launcher = try launcherBundleCodeEvidence(at: launcherURL)
    let payload = try launcherBundleCodeEvidence(at: payloadURL)

    guard bundle.identifier == enrollment.bundleIdentifier,
          appURL.standardizedFileURL.path == URL(fileURLWithPath: enrollment.bundlePath).standardizedFileURL.path,
          bundle.designatedRequirement == enrollment.launcherRequirement,
          bundle.codeIdentifiers == enrollment.bundleCodeIdentifiers,
          launcher.identifier == enrollment.launcherIdentifier,
          launcher.codeIdentifiers == enrollment.launcherCodeIdentifiers,
          launcher.identifier == liveLauncherIdentifier,
          enrollment.launcherCodeIdentifiers.contains(liveLauncherCodeIdentifier),
          payload.codeIdentifiers == enrollment.payloadCodeIdentifiers,
          bundle.isAdHoc == (enrollment.signingKind == .adHoc),
          launcher.isAdHoc == bundle.isAdHoc,
          payload.isAdHoc == bundle.isAdHoc
    else { throw LauncherBundleVerificationError.identityMismatch }

    guard try sha256OfRegularFile(at: payloadURL) == enrollment.payloadSHA256,
          info.payloadSHA256 == enrollment.payloadSHA256
    else { throw LauncherBundleVerificationError.payloadMismatch }
    guard enrollment.runtimeRequirement.allows(liveRuntimeProtection),
          enrollment.runtimeRequirement.allows(payload.runtimeProtection)
    else { throw LauncherBundleVerificationError.runtimeMismatch }
    return enrollment
}

public func launcherBundleCodeEvidence(
    at url: URL,
    bundle: Bool = false
) throws -> LauncherBundleCodeEvidence {
    var code: SecStaticCode?
    guard SecStaticCodeCreateWithPath(url as CFURL, [], &code) == errSecSuccess,
          let code
    else { throw LauncherBundleVerificationError.invalidBundle }
    var rawFlags = kSecCSStrictValidate | kSecCSCheckAllArchitectures
    if bundle { rawFlags |= kSecCSCheckNestedCode }
    guard SecStaticCodeCheckValidity(code, SecCSFlags(rawValue: rawFlags), nil) == errSecSuccess
    else { throw LauncherBundleVerificationError.invalidBundle }

    var rawInfo: CFDictionary?
    let informationFlags = SecCSFlags(
        rawValue: kSecCSSigningInformation | kSecCSRequirementInformation
    )
    guard SecCodeCopySigningInformation(code, informationFlags, &rawInfo) == errSecSuccess,
          let dictionary = rawInfo as? [CFString: Any],
          let identifier = dictionary[kSecCodeInfoIdentifier] as? String,
          let requirement = dictionary[kSecCodeInfoDesignatedRequirement] as! SecRequirement?,
          let requirementText = launcherBundleRequirementString(requirement)
    else { throw LauncherBundleVerificationError.invalidBundle }
    let identifiers = (dictionary[kSecCodeInfoCdHashes] as? [Data])
        ?? [dictionary[kSecCodeInfoUnique] as? Data].compactMap(\.self)
    guard !identifiers.isEmpty else { throw LauncherBundleVerificationError.invalidBundle }
    let signatureFlags = (dictionary[kSecCodeInfoFlags] as? NSNumber)?.uint32Value ?? 0
    return LauncherBundleCodeEvidence(
        identifier: identifier,
        teamIdentifier: dictionary[kSecCodeInfoTeamIdentifier] as? String,
        designatedRequirement: requirementText,
        codeIdentifiers: identifiers,
        isAdHoc: signatureFlags & SecCodeSignatureFlags.adhoc.rawValue != 0,
        runtimeProtection: launcherRuntimeProtection(signingInformation: dictionary)
    )
}

public struct LauncherBundlePayloadSnapshot: Equatable, Sendable {
    public let sourcePath: String
    public let sourceSHA256: String
    public let byteCount: Int64
}

public enum LauncherBundlePayloadError: Error, Equatable, LocalizedError {
    case cannotOpen
    case cannotReadMetadata
    case notRegularMachO
    case notExecutable
    case tooLarge
    case changedDuringCopy
    case cannotWrite

    public var errorDescription: String? {
        switch self {
        case .cannotOpen: "CLI executable could not be opened securely"
        case .cannotReadMetadata: "CLI executable metadata could not be read"
        case .notRegularMachO: "Choose one regular Mach-O executable"
        case .notExecutable: "The selected Mach-O is not executable"
        case .tooLarge: "The selected Mach-O exceeds 512 MiB"
        case .changedDuringCopy: "The selected executable changed while it was copied"
        case .cannotWrite: "The Launcher Bundle payload could not be written"
        }
    }
}

public func copyLauncherBundlePayload(
    from sourceURL: URL,
    to destinationURL: URL
) throws -> LauncherBundlePayloadSnapshot {
    let source = sourceURL.resolvingSymlinksInPath().standardizedFileURL
    let sourceDescriptor = open(source.path, O_RDONLY | O_NOFOLLOW | O_CLOEXEC)
    guard sourceDescriptor >= 0 else { throw LauncherBundlePayloadError.cannotOpen }
    defer { close(sourceDescriptor) }
    var before = stat()
    guard fstat(sourceDescriptor, &before) == 0 else {
        throw LauncherBundlePayloadError.cannotReadMetadata
    }
    guard before.st_mode & S_IFMT == S_IFREG else {
        throw LauncherBundlePayloadError.notRegularMachO
    }
    guard before.st_mode & 0o111 != 0 else { throw LauncherBundlePayloadError.notExecutable }
    guard before.st_size <= launcherBundleMaximumPayloadBytes else {
        throw LauncherBundlePayloadError.tooLarge
    }
    let destinationDescriptor = open(
        destinationURL.path,
        O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC,
        mode_t(0o700)
    )
    guard destinationDescriptor >= 0 else { throw LauncherBundlePayloadError.cannotWrite }
    var succeeded = false
    defer {
        close(destinationDescriptor)
        if !succeeded { unlink(destinationURL.path) }
    }

    var hasher = SHA256()
    var header = [UInt8]()
    var buffer = [UInt8](repeating: 0, count: 1024 * 1024)
    var total: Int64 = 0
    while true {
        let count = buffer.withUnsafeMutableBytes {
            Darwin.read(sourceDescriptor, $0.baseAddress, $0.count)
        }
        if count == 0 { break }
        if count < 0 {
            if errno == EINTR { continue }
            throw LauncherBundlePayloadError.cannotOpen
        }
        total += Int64(count)
        guard total <= launcherBundleMaximumPayloadBytes else {
            throw LauncherBundlePayloadError.tooLarge
        }
        let chunk = Data(buffer.prefix(count))
        if header.count < 4 { header.append(contentsOf: chunk.prefix(4 - header.count)) }
        hasher.update(data: chunk)
        try writeLauncherBundleBytes(chunk, to: destinationDescriptor)
    }
    guard launcherBundleMachOMagic(header) else {
        throw LauncherBundlePayloadError.notRegularMachO
    }
    var after = stat()
    guard fstat(sourceDescriptor, &after) == 0 else {
        throw LauncherBundlePayloadError.cannotReadMetadata
    }
    guard before.st_dev == after.st_dev,
          before.st_ino == after.st_ino,
          before.st_size == after.st_size,
          before.st_mtimespec.tv_sec == after.st_mtimespec.tv_sec,
          before.st_mtimespec.tv_nsec == after.st_mtimespec.tv_nsec,
          total == before.st_size
    else { throw LauncherBundlePayloadError.changedDuringCopy }
    guard fsync(destinationDescriptor) == 0 else { throw LauncherBundlePayloadError.cannotWrite }
    succeeded = true
    return LauncherBundlePayloadSnapshot(
        sourcePath: source.path,
        sourceSHA256: hasher.finalize().hexString,
        byteCount: total
    )
}

public func sha256OfRegularFile(at url: URL) throws -> String {
    let descriptor = open(url.path, O_RDONLY | O_NOFOLLOW | O_CLOEXEC)
    guard descriptor >= 0 else { throw LauncherBundlePayloadError.cannotOpen }
    defer { close(descriptor) }
    var info = stat()
    guard fstat(descriptor, &info) == 0 else {
        throw LauncherBundlePayloadError.cannotReadMetadata
    }
    guard info.st_mode & S_IFMT == S_IFREG else {
        throw LauncherBundlePayloadError.notRegularMachO
    }
    var hasher = SHA256()
    var buffer = [UInt8](repeating: 0, count: 1024 * 1024)
    while true {
        let count = buffer.withUnsafeMutableBytes {
            Darwin.read(descriptor, $0.baseAddress, $0.count)
        }
        if count == 0 { return hasher.finalize().hexString }
        if count < 0 {
            if errno == EINTR { continue }
            throw LauncherBundlePayloadError.cannotOpen
        }
        hasher.update(data: Data(buffer.prefix(count)))
    }
}

public func launcherBundleDisplayName(from value: String) -> String? {
    let name = value.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !name.isEmpty, name.utf8.count <= 80,
          !name.contains("/"), name != ".", name != "..",
          name.unicodeScalars.allSatisfy({ !CharacterSet.controlCharacters.contains($0) })
    else { return nil }
    return name
}

public func launcherBundleManagedDirectory() -> URL {
    FileManager.default.homeDirectoryForCurrentUser
        .appendingPathComponent("Applications/Automic Vault", isDirectory: true)
}

public func launcherBundleAppURL(
    containing executablePath: String,
    managedDirectory: URL = launcherBundleManagedDirectory()
) -> URL? {
    let managedPath = managedDirectory.standardizedFileURL.path + "/"
    var candidate = URL(fileURLWithPath: executablePath).standardizedFileURL
    while candidate.path != "/" {
        if candidate.pathExtension.caseInsensitiveCompare("app") == .orderedSame,
           candidate.path.hasPrefix(managedPath) {
            return candidate
        }
        candidate.deleteLastPathComponent()
    }
    return nil
}

private struct LauncherBundleInfo {
    let generation: UUID
    let bundleIdentifier: String
    let executable: String
    let payloadSHA256: String
}

private func launcherBundleInfo(at appURL: URL) throws -> LauncherBundleInfo {
    let infoURL = appURL.appendingPathComponent("Contents/Info.plist")
    guard let data = try? Data(contentsOf: infoURL),
          let dictionary = try? PropertyListSerialization.propertyList(
              from: data,
              format: nil
          ) as? [String: Any],
          let generationText = dictionary[launcherBundleGenerationInfoKey] as? String,
          let generation = UUID(uuidString: generationText),
          let bundleIdentifier = dictionary[kCFBundleIdentifierKey as String] as? String,
          let executable = dictionary[kCFBundleExecutableKey as String] as? String,
          let payloadSHA256 = dictionary[launcherBundlePayloadSHA256InfoKey] as? String
    else { throw LauncherBundleVerificationError.invalidBundle }
    return LauncherBundleInfo(
        generation: generation,
        bundleIdentifier: bundleIdentifier,
        executable: executable,
        payloadSHA256: payloadSHA256
    )
}

private func launcherBundleRequirementString(_ requirement: SecRequirement) -> String? {
    var text: CFString?
    guard SecRequirementCopyString(requirement, [], &text) == errSecSuccess,
          let text
    else { return nil }
    return text as String
}

private func launcherBundleEnrollmentPrecedes(
    _ lhs: LauncherBundleEnrollment,
    _ rhs: LauncherBundleEnrollment
) -> Bool {
    let comparison = lhs.displayName.localizedStandardCompare(rhs.displayName)
    return comparison == .orderedAscending
        || (comparison == .orderedSame && lhs.createdAt < rhs.createdAt)
}

private func normalizedCodeIdentifiers(_ identifiers: [Data]) -> [Data] {
    Array(Set(identifiers)).sorted { $0.hexString < $1.hexString }
}

private func launcherBundleMachOMagic(_ bytes: [UInt8]) -> Bool {
    guard bytes.count == 4 else { return false }
    let big = bytes.reduce(UInt32(0)) { ($0 << 8) | UInt32($1) }
    return [
        UInt32(0xfeedface), 0xcefaedfe, 0xfeedfacf, 0xcffaedfe,
        0xcafebabe, 0xbebafeca, 0xcafebabf, 0xbfbafeca,
    ].contains(big)
}

private func writeLauncherBundleBytes(_ data: Data, to descriptor: Int32) throws {
    try data.withUnsafeBytes { bytes in
        var written = 0
        while written < bytes.count {
            let count = Darwin.write(
                descriptor,
                bytes.baseAddress?.advanced(by: written),
                bytes.count - written
            )
            if count < 0 {
                if errno == EINTR { continue }
                throw LauncherBundlePayloadError.cannotWrite
            }
            written += count
        }
    }
}

private extension Sequence where Element == UInt8 {
    var hexString: String { map { String(format: "%02x", $0) }.joined() }
}
