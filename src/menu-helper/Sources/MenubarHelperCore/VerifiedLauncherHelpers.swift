import Foundation
import MachO
import Security

private let verifiedLauncherHelpersKeychainService = "com.automicvault.verified-launcher-helpers"
private let verifiedLauncherHelpersKeychainAccount = "VerifiedLauncherHelpersV1"

public struct VerifiedLauncherHelper: Identifiable, Codable, Equatable, Sendable {
    public let id: String
    public let name: String
    public let appName: String
    public let appBundleIdentifier: String
    public let appTeamIdentifier: String
    public let helperSigningIdentifier: String
    public let helperTeamIdentifier: String
    public let relativePath: String?

    public init(
        id: String,
        name: String,
        appName: String,
        appBundleIdentifier: String,
        appTeamIdentifier: String,
        helperSigningIdentifier: String,
        helperTeamIdentifier: String,
        relativePath: String? = nil
    ) {
        self.id = id
        self.name = name
        self.appName = appName
        self.appBundleIdentifier = appBundleIdentifier
        self.appTeamIdentifier = appTeamIdentifier
        self.helperSigningIdentifier = helperSigningIdentifier
        self.helperTeamIdentifier = helperTeamIdentifier
        self.relativePath = relativePath
    }

    public func hasSameSigningAssociation(as other: Self) -> Bool {
        appBundleIdentifier == other.appBundleIdentifier
            && appTeamIdentifier == other.appTeamIdentifier
            && helperSigningIdentifier == other.helperSigningIdentifier
            && helperTeamIdentifier == other.helperTeamIdentifier
            && relativePath == other.relativePath
    }
}

public struct VerifiedLauncherHelperParent: Identifiable {
    public let appName: String
    public let appBundleIdentifier: String
    public let appTeamIdentifier: String
    public var id: String { "\(appTeamIdentifier.utf8.count):\(appTeamIdentifier)\(appBundleIdentifier)" }
}

public func verifiedLauncherHelperParents(
    helpers: [VerifiedLauncherHelper], appPolicies: [SecretGatePolicy]
) -> [VerifiedLauncherHelperParent] {
    var parents = helpers.map {
        VerifiedLauncherHelperParent(appName: $0.appName,
            appBundleIdentifier: $0.appBundleIdentifier, appTeamIdentifier: $0.appTeamIdentifier)
    }
    parents += appPolicies.compactMap {
        guard let team = codeSigningTeamIdentifier(from: $0.requirement) else { return nil }
        return VerifiedLauncherHelperParent(appName: $0.bundleIdentifier,
            appBundleIdentifier: $0.bundleIdentifier, appTeamIdentifier: team)
    }
    var seen = Set<String>()
    return parents.filter { seen.insert($0.id).inserted }
}

public struct VerifiedLauncherHelperOutlineItem: Identifiable {
    public let id: String
    public let title: String
    public let depth: Int
    public let helper: VerifiedLauncherHelper?
}

/// Bundle paths organize presentation only; they do not confer Launcher Identity.
public func verifiedLauncherHelperOutline(_ helpers: [VerifiedLauncherHelper]) -> [VerifiedLauncherHelperOutlineItem] {
    func containers(_ helper: VerifiedLauncherHelper) -> [String] {
        let parts = (helper.relativePath ?? "").split(separator: "/").dropLast()
        return parts.enumerated().compactMap { index, part in
            ["app", "framework", "xpc", "bundle"].contains((String(part) as NSString).pathExtension.lowercased())
                ? parts.prefix(index + 1).joined(separator: "/") : nil
        }
    }
    let groups = Dictionary(grouping: helpers) { containers($0).last ?? "" }
    var result: [VerifiedLauncherHelperOutlineItem] = []
    var headings = Set<String>()
    for (_, group) in groups.sorted(by: { $0.key < $1.key }) {
        let members = group.sorted {
            let order = $0.name.localizedCaseInsensitiveCompare($1.name)
            return order == .orderedSame ? $0.id < $1.id : order == .orderedAscending
        }
        let parents = containers(members[0])
        for (depth, parent) in parents.enumerated() where headings.insert(parent).inserted {
            result.append(.init(id: "bundle:" + parent,
                title: (parent as NSString).lastPathComponent, depth: depth, helper: nil))
        }
        for helper in members {
            result.append(.init(id: helper.id, title: helper.name, depth: parents.count, helper: helper))
        }
    }
    return result
}

/// A review default only: this never adds a helper to the enabled catalog.
public func shouldPreselectVerifiedLauncherHelper(_ helper: VerifiedLauncherHelper) -> Bool {
    guard isValidUserApprovedHelper(helper),
          helper.appTeamIdentifier == helper.helperTeamIdentifier else { return false }
    switch (helper.appBundleIdentifier, helper.appTeamIdentifier, helper.helperSigningIdentifier) {
    case ("com.openai.codex", "2DC432GLL2", "codex"),
         ("com.anthropic.claudefordesktop", "Q6L2SF6YDW", "com.anthropic.claude-code"):
        return true
    default:
        return false
    }
}

public struct VerifiedLauncherHelperConfiguration: Codable, Equatable, Sendable {
    public var disabledHelperIDs: Set<String>
    public var userApprovedHelpers: [VerifiedLauncherHelper]
    public var allowedOutsideBundleHelperIDs: Set<String>

    public init(
        disabledHelperIDs: Set<String> = [],
        userApprovedHelpers: [VerifiedLauncherHelper] = [],
        allowedOutsideBundleHelperIDs: Set<String> = []
    ) {
        self.disabledHelperIDs = disabledHelperIDs
        self.userApprovedHelpers = userApprovedHelpers
        self.allowedOutsideBundleHelperIDs = allowedOutsideBundleHelperIDs
    }

    public func isEnabled(_ helper: VerifiedLauncherHelper) -> Bool {
        guard let helper = catalogHelper(matching: helper) else { return false }
        return !disabledHelperIDs.contains(helper.id)
    }

    public func shouldSelectInReview(_ helper: VerifiedLauncherHelper) -> Bool {
        if isEnabled(helper) { return true }
        guard shouldPreselectVerifiedLauncherHelper(helper),
              !disabledHelperIDs.contains(helper.id) else { return false }
        let legacyID = helper.appBundleIdentifier == "com.openai.codex" ? "codex" : "claude-code"
        return !disabledHelperIDs.contains(legacyID)
    }

    public var helpers: [VerifiedLauncherHelper] {
        userApprovedHelpers
    }

    public func catalogHelper(matching discovered: VerifiedLauncherHelper) -> VerifiedLauncherHelper? {
        helpers.first { $0.hasSameSigningAssociation(as: discovered) }
    }

    public mutating func enable(_ helpers: [VerifiedLauncherHelper]) {
        for discovered in helpers {
            guard isValidUserApprovedHelper(discovered) else { continue }
            if let helper = catalogHelper(matching: discovered) {
                disabledHelperIDs.remove(helper.id)
            } else {
                userApprovedHelpers.append(discovered)
                disabledHelperIDs.remove(discovered.id)
            }
        }
        userApprovedHelpers.sort { $0.id < $1.id }
    }

    public mutating func remove(_ helper: VerifiedLauncherHelper) {
        guard userApprovedHelpers.contains(where: { $0.id == helper.id }) else { return }
        userApprovedHelpers.removeAll { $0.id == helper.id }
        disabledHelperIDs.remove(helper.id)
        allowedOutsideBundleHelperIDs.remove(helper.id)
    }

    private enum CodingKeys: String, CodingKey {
        case disabledHelperIDs
        case userApprovedHelpers
        case allowedOutsideBundleHelperIDs
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        disabledHelperIDs = try container.decodeIfPresent(
            Set<String>.self,
            forKey: .disabledHelperIDs
        ) ?? []
        userApprovedHelpers = try container.decodeIfPresent(
            [VerifiedLauncherHelper].self,
            forKey: .userApprovedHelpers
        ) ?? []
        allowedOutsideBundleHelperIDs = try container.decodeIfPresent(
            Set<String>.self,
            forKey: .allowedOutsideBundleHelperIDs
        ) ?? []
        guard isValidVerifiedLauncherHelperConfiguration(self) else {
            throw DecodingError.dataCorrupted(
                .init(codingPath: decoder.codingPath, debugDescription: "Invalid user-approved Launcher helper catalog")
            )
        }
    }
}

public func userApprovedVerifiedLauncherHelperID(_ helper: VerifiedLauncherHelper) -> String {
    "user:" + [
        helper.appTeamIdentifier,
        helper.appBundleIdentifier,
        helper.helperTeamIdentifier,
        helper.helperSigningIdentifier,
        helper.relativePath ?? "",
    ].map { "\($0.utf8.count):\($0)" }.joined()
}

@concurrent
public func discoverVerifiedLauncherHelpers(in appURL: URL) async -> [VerifiedLauncherHelper] {
    let appURL = appURL.standardizedFileURL.resolvingSymlinksInPath()
    guard appURL.pathExtension.caseInsensitiveCompare("app") == .orderedSame,
          let appBundle = Bundle(url: appURL),
          let appBundleIdentifier = appBundle.bundleIdentifier,
          let appExecutableURL = appBundle.executableURL?.standardizedFileURL.resolvingSymlinksInPath(),
          let appCode = staticCode(at: appURL),
          validateAppBundleMainExecutable(appCode) == errSecSuccess,
          let appSigning = signingIdentity(appCode),
          let appTeamIdentifier = appSigning.teamIdentifier,
          let developerIDRequirement = developerIDRequirement()
    else { return [] }

    let contentsURL = appURL.appendingPathComponent("Contents", isDirectory: true)
    guard let enumerator = FileManager.default.enumerator(
        at: contentsURL,
        includingPropertiesForKeys: [.isRegularFileKey, .isSymbolicLinkKey],
        options: [],
        errorHandler: { _, _ in true }
    ) else { return [] }

    var helpers: [VerifiedLauncherHelper] = []
    while let candidate = enumerator.nextObject() as? URL {
        guard !Task.isCancelled else { return [] }
        guard let values = try? candidate.resourceValues(forKeys: [.isRegularFileKey, .isSymbolicLinkKey]),
              values.isRegularFile == true,
              values.isSymbolicLink != true,
              FileManager.default.isExecutableFile(atPath: candidate.path)
        else { continue }

        let executableURL = candidate.standardizedFileURL.resolvingSymlinksInPath()
        guard executableURL != appExecutableURL,
              isLauncherHelperExecutable(at: executableURL),
              let helperCode = staticCode(at: executableURL),
              SecStaticCodeCheckValidity(
                  helperCode,
                  SecCSFlags(rawValue: kSecCSCheckAllArchitectures | kSecCSStrictValidate),
                  developerIDRequirement
              ) == errSecSuccess,
              validateAppBundleResource(appCode, resourceURL: executableURL) == errSecSuccess,
              let helperSigning = signingIdentity(helperCode),
              let helperTeamIdentifier = helperSigning.teamIdentifier,
              launcherRuntimeProtection(signingInformation: helperSigning.information)
                  .allowsSecretGateAccess
        else { continue }

        guard let relativePath = verifiedLauncherHelperRelativePath(
            for: executableURL,
            inside: appURL
        ) else { continue }
        var helper = VerifiedLauncherHelper(
            id: "",
            name: helperDisplayName(executableURL, inside: appURL),
            appName: appDisplayName(appBundle, url: appURL),
            appBundleIdentifier: appBundleIdentifier,
            appTeamIdentifier: appTeamIdentifier,
            helperSigningIdentifier: helperSigning.identifier,
            helperTeamIdentifier: helperTeamIdentifier,
            relativePath: relativePath
        )
        helper = VerifiedLauncherHelper(
            id: userApprovedVerifiedLauncherHelperID(helper),
            name: helper.name,
            appName: helper.appName,
            appBundleIdentifier: helper.appBundleIdentifier,
            appTeamIdentifier: helper.appTeamIdentifier,
            helperSigningIdentifier: helper.helperSigningIdentifier,
            helperTeamIdentifier: helper.helperTeamIdentifier,
            relativePath: helper.relativePath
        )
        if !helpers.contains(where: { $0.id == helper.id }) { helpers.append(helper) }
    }
    return helpers.sorted {
        let order = $0.name.localizedCaseInsensitiveCompare($1.name)
        return order == .orderedSame
            ? ($0.relativePath ?? "") < ($1.relativePath ?? "")
            : order == .orderedAscending
    }
}

// Execute permission also appears on dylibs and loadable bundles. Only MH_EXECUTE
// images can independently launch and represent a Launcher.
func isLauncherHelperExecutable(at url: URL) -> Bool {
    guard let file = try? FileHandle(forReadingFrom: url) else { return false }
    defer { try? file.close() }
    func read(at offset: UInt64, count: Int) -> [UInt8] {
        guard (try? file.seek(toOffset: offset)) != nil,
              let data = try? file.read(upToCount: count), data.count == count else { return [] }
        return Array(data)
    }
    func integer(_ bytes: ArraySlice<UInt8>, littleEndian: Bool) -> UInt64 {
        (littleEndian ? Array(bytes.reversed()) : Array(bytes)).reduce(0) { ($0 << 8) | UInt64($1) }
    }
    func isExecutableHeader(_ bytes: [UInt8]) -> Bool {
        guard bytes.count == 16 else { return false }
        let magic = integer(bytes[0..<4], littleEndian: false)
        guard [0xfeedface, 0xfeedfacf, 0xcefaedfe, 0xcffaedfe].contains(magic) else { return false }
        return integer(bytes[12..<16], littleEndian: magic == 0xcefaedfe || magic == 0xcffaedfe) == UInt64(MH_EXECUTE)
    }
    let header = read(at: 0, count: 16)
    if isExecutableHeader(header) { return true }
    guard header.count == 16 else { return false }
    let magic = integer(header[0..<4], littleEndian: false)
    guard [0xcafebabe, 0xbebafeca, 0xcafebabf, 0xbfbafeca].contains(magic) else { return false }
    let littleEndian = magic == 0xbebafeca || magic == 0xbfbafeca
    let is64 = magic == 0xcafebabf || magic == 0xbfbafeca
    let count = integer(header[4..<8], littleEndian: littleEndian)
    // Bound work on untrusted universal headers; signatures are verified separately.
    guard count > 0, count <= 64 else { return false }
    let stride = is64 ? 32 : 20
    let table = read(at: 8, count: Int(count) * stride)
    guard table.count == Int(count) * stride else { return false }
    for index in 0..<Int(count) {
        let start = index * stride + 8
        let offset = integer(table[start..<(start + (is64 ? 8 : 4))], littleEndian: littleEndian)
        guard offset >= UInt64(8 + table.count),
              isExecutableHeader(read(at: offset, count: 16)) else { return false }
    }
    return true
}

func verifiedLauncherHelperRelativePath(for executableURL: URL, inside appURL: URL) -> String? {
    let appURL = appURL.standardizedFileURL.resolvingSymlinksInPath()
    let executableURL = executableURL.standardizedFileURL.resolvingSymlinksInPath()
    let prefix = appURL.path + "/"
    guard executableURL.path.hasPrefix(prefix) else { return nil }
    return String(executableURL.path.dropFirst(prefix.count))
}

private func isValidUserApprovedHelper(_ helper: VerifiedLauncherHelper) -> Bool {
    guard helper.id == userApprovedVerifiedLauncherHelperID(helper),
          !helper.name.isEmpty,
          !helper.appName.isEmpty,
          !helper.appBundleIdentifier.isEmpty,
          !helper.appTeamIdentifier.isEmpty,
          !helper.helperSigningIdentifier.isEmpty,
          !helper.helperTeamIdentifier.isEmpty,
          let relativePath = helper.relativePath,
          !relativePath.isEmpty,
          !relativePath.hasPrefix("/")
    else { return false }
    return !relativePath.split(separator: "/", omittingEmptySubsequences: false).contains {
        $0.isEmpty || $0 == "." || $0 == ".."
    }
}

private func isValidVerifiedLauncherHelperConfiguration(
    _ configuration: VerifiedLauncherHelperConfiguration
) -> Bool {
    Set(configuration.userApprovedHelpers.map(\.id)).count
        == configuration.userApprovedHelpers.count
        && configuration.userApprovedHelpers.allSatisfy(isValidUserApprovedHelper)
}

private struct HelperSigningIdentity {
    let identifier: String
    let teamIdentifier: String?
    let information: [CFString: Any]
}

private func staticCode(at url: URL) -> SecStaticCode? {
    var code: SecStaticCode?
    guard SecStaticCodeCreateWithPath(url as CFURL, [], &code) == errSecSuccess else { return nil }
    return code
}

private func signingIdentity(_ code: SecStaticCode) -> HelperSigningIdentity? {
    var rawInformation: CFDictionary?
    let flags = SecCSFlags(rawValue: kSecCSSigningInformation | kSecCSRequirementInformation)
    guard SecCodeCopySigningInformation(code, flags, &rawInformation) == errSecSuccess,
          let information = rawInformation as? [CFString: Any],
          let identifier = information[kSecCodeInfoIdentifier] as? String
    else { return nil }
    return HelperSigningIdentity(
        identifier: identifier,
        teamIdentifier: information[kSecCodeInfoTeamIdentifier] as? String,
        information: information
    )
}

private func developerIDRequirement() -> SecRequirement? {
    var requirement: SecRequirement?
    let source = """
    anchor apple generic and \
    certificate 1[field.1.2.840.113635.100.6.2.6] exists and \
    certificate leaf[field.1.2.840.113635.100.6.1.13] exists
    """
    guard SecRequirementCreateWithString(source as CFString, [], &requirement) == errSecSuccess
    else { return nil }
    return requirement
}

private func appDisplayName(_ bundle: Bundle, url: URL) -> String {
    bundle.object(forInfoDictionaryKey: "CFBundleDisplayName") as? String
        ?? bundle.object(forInfoDictionaryKey: "CFBundleName") as? String
        ?? url.deletingPathExtension().lastPathComponent
}

private func helperDisplayName(_ executableURL: URL, inside appURL: URL) -> String {
    var container = executableURL.deletingLastPathComponent()
    while container.path.count > appURL.path.count {
        if container.pathExtension.caseInsensitiveCompare("app") == .orderedSame,
           let bundle = Bundle(url: container),
           bundle.executableURL?.standardizedFileURL.resolvingSymlinksInPath() == executableURL
        {
            return appDisplayName(bundle, url: container)
        }
        container.deleteLastPathComponent()
    }
    return executableURL.lastPathComponent
}

public func loadVerifiedLauncherHelperConfiguration() -> VerifiedLauncherHelperConfiguration {
    loadVerifiedLauncherHelperConfiguration(
        service: verifiedLauncherHelpersKeychainService,
        account: verifiedLauncherHelpersKeychainAccount
    )
}

func loadVerifiedLauncherHelperConfiguration(
    service: String,
    account: String
) -> VerifiedLauncherHelperConfiguration {
    switch loadKeychainDataResult(service: service, account: account) {
    case .notFound:
        return VerifiedLauncherHelperConfiguration()
    case .failure:
        return failClosedVerifiedLauncherHelperConfiguration
    case .success(let data):
        return decodeVerifiedLauncherHelperConfiguration(data)
    }
}

func decodeVerifiedLauncherHelperConfiguration(
    _ data: Data
) -> VerifiedLauncherHelperConfiguration {
    (try? JSONDecoder().decode(
        VerifiedLauncherHelperConfiguration.self,
        from: data
    )) ?? failClosedVerifiedLauncherHelperConfiguration
}

@discardableResult
public func saveVerifiedLauncherHelperConfiguration(
    _ configuration: VerifiedLauncherHelperConfiguration
) -> OSStatus {
    saveVerifiedLauncherHelperConfiguration(
        configuration,
        service: verifiedLauncherHelpersKeychainService,
        account: verifiedLauncherHelpersKeychainAccount
    )
}

@discardableResult
func saveVerifiedLauncherHelperConfiguration(
    _ configuration: VerifiedLauncherHelperConfiguration,
    service: String,
    account: String
) -> OSStatus {
    guard isValidVerifiedLauncherHelperConfiguration(configuration),
          let data = try? JSONEncoder().encode(configuration)
    else { return errSecParam }
    return saveKeychainData(
        data,
        service: service,
        account: account,
        accessibility: .afterFirstUnlock
    )
}

private let failClosedVerifiedLauncherHelperConfiguration = VerifiedLauncherHelperConfiguration()
