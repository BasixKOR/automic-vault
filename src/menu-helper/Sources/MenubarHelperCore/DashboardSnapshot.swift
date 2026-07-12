import CryptoKit
import Foundation
import Security

public let automicVaultKeychainService = "com.automicvault.isotope"
public let secretGatePoliciesKeychainService = "com.automicvault.gate-policies"
public let secretGatePoliciesKeychainAccount = "SecretGatePoliciesV2"
public let accessRequestLogDefaultsKey = "AccessRequestLog"

public struct DashboardSnapshot: Equatable, Sendable {
    public var detectors: [DetectorMetadata]
    public var detectorFindings: [DetectorFinding]
    public var hardenedTools: [HardenedTool]
    public var hardeners: [HardenerMetadata]
    public var secretGates: [SecretGate]
    public var secrets: [StoredSecret]
    public var accessRequests: [AccessRequestRecord]

    public init(
        detectors: [DetectorMetadata],
        detectorFindings: [DetectorFinding],
        hardenedTools: [HardenedTool],
        hardeners: [HardenerMetadata] = [],
        secretGates: [SecretGate],
        secrets: [StoredSecret],
        accessRequests: [AccessRequestRecord] = []
    ) {
        self.detectors = detectors
        self.detectorFindings = detectorFindings
        self.hardenedTools = hardenedTools
        self.hardeners = hardeners
        self.secretGates = secretGates
        self.secrets = secrets
        self.accessRequests = accessRequests
    }

    public static let empty = DashboardSnapshot(
        detectors: [],
        detectorFindings: [],
        hardenedTools: [],
        hardeners: [],
        secretGates: [],
        secrets: [],
        accessRequests: []
    )

    public var flaggedDetectorCount: Int {
        Set(detectorFindings.map(\.source)).count
    }

    public var detectorDisplayCount: Int {
        flaggedDetectorCount == 0 ? detectors.count : flaggedDetectorCount
    }

    public static func load(
        avExecutableURL: URL = defaultAVExecutableURL(),
        stubDirectory: URL = URL(fileURLWithPath: "/usr/local/bin", isDirectory: true),
        ghCLIURL: URL? = URL(fileURLWithPath: "/opt/homebrew/opt/gh-cli/bin/gh"),
        policyService: String = secretGatePoliciesKeychainService
    ) -> DashboardSnapshot {
        let hardenerMetadata = loadHardenerMetadata(avExecutableURL: avExecutableURL)
        let hardenedTools = loadHardenedTools(
            in: stubDirectory,
            ghCLIURL: ghCLIURL,
            metadata: hardenerMetadata
        )
        let secrets = loadStoredSecrets()
        return DashboardSnapshot(
            detectors: loadDetectorMetadata(avExecutableURL: avExecutableURL),
            detectorFindings: scanDetectorFindings(avExecutableURL: avExecutableURL),
            hardenedTools: hardenedTools,
            hardeners: hardenerMetadata,
            secretGates: loadSecretGates(hardeners: hardenerMetadata, service: policyService),
            secrets: secrets,
            accessRequests: loadAccessRequestRecords()
        )
    }
}

public struct DetectorMetadata: Codable, Equatable, Sendable {
    public let name: String
    public let homepage: String
    public let docsURL: String
    public let documentation: String

    public var displayName: DetectorDisplayName {
        detectorDisplayName(name)
    }

    public init(name: String, homepage: String, docsURL: String, documentation: String = "") {
        self.name = name
        self.homepage = homepage
        self.docsURL = docsURL
        self.documentation = documentation
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.name = try container.decode(String.self, forKey: .name)
        self.homepage = try container.decode(String.self, forKey: .homepage)
        self.docsURL = try container.decode(String.self, forKey: .docsURL)
        self.documentation = try container.decodeIfPresent(String.self, forKey: .documentation) ?? ""
    }

    enum CodingKeys: String, CodingKey {
        case name
        case homepage
        case docsURL = "docs_url"
        case documentation
    }
}

public struct DetectorDisplayName: Equatable, Sendable {
    public let packageName: String
    public let kind: String?

    public init(packageName: String, kind: String? = nil) {
        self.packageName = packageName
        self.kind = kind
    }
}

public func detectorDisplayName(_ name: String) -> DetectorDisplayName {
    splitDetectorDisplayNames[name] ?? DetectorDisplayName(packageName: name, kind: "plaintext secret")
}

private let splitDetectorDisplayNames: [String: DetectorDisplayName] = [
    "aws-cli-credentials-file": DetectorDisplayName(packageName: "aws-cli", kind: "credentials file"),
    "aws-cli-legacy-plugins": DetectorDisplayName(packageName: "aws-cli", kind: "legacy plugins"),
    "aws-cli-login-cache": DetectorDisplayName(packageName: "aws-cli", kind: "login cache"),
    "cariddi-persisted-output": DetectorDisplayName(packageName: "cariddi", kind: "persisted output"),
    "cariddi-shell-history": DetectorDisplayName(packageName: "cariddi", kind: "shell history"),
    "docker-credential-helpers": DetectorDisplayName(packageName: "docker", kind: "credential helpers"),
    "docker-registry-credentials": DetectorDisplayName(packageName: "docker", kind: "registry credentials"),
    "docker-root-access": DetectorDisplayName(packageName: "docker", kind: "root access"),
    "gh-cli-hosts-token": DetectorDisplayName(packageName: "gh-cli", kind: "hosts token"),
    "gh-cli-keychain-access": DetectorDisplayName(packageName: "gh-cli", kind: "keychain access"),
    "git-credential-fill": DetectorDisplayName(packageName: "git", kind: "credential fill"),
    "git-credential-oauth": DetectorDisplayName(packageName: "git", kind: "credential oauth"),
    "git-credentials-file": DetectorDisplayName(packageName: "git", kind: "credentials file"),
    "homebrew": DetectorDisplayName(packageName: "homebrew", kind: "mutable"),
    "pnpm-auth-token": DetectorDisplayName(packageName: "pnpm", kind: "auth token"),
    "pnpm-minimum-release-age": DetectorDisplayName(packageName: "pnpm", kind: "minimum release age"),
    "secretlint-persisted-report": DetectorDisplayName(packageName: "secretlint", kind: "persisted report"),
    "secretlint-shell-history": DetectorDisplayName(packageName: "secretlint", kind: "shell history"),
    "sip": DetectorDisplayName(packageName: "SIP", kind: "system integrity"),
    "sudo": DetectorDisplayName(packageName: "sudo", kind: "root hardening"),
]

public struct DetectorFinding: Codable, Equatable, Sendable {
    public let source: String
    public let severity: String
    public let homepage: String?
    public let explanation: String?
    public let solution: String?
    public let affected: [AffectedFile]
    public let docsURL: String?

    enum CodingKeys: String, CodingKey {
        case source
        case severity
        case homepage
        case explanation
        case solution
        case affected
        case docsURL = "docs_url"
    }
}

public struct AffectedFile: Codable, Equatable, Sendable {
    public let path: String
    public let line: Int
}

public struct HardenedTool: Equatable, Sendable {
    public let name: String
    public let stubPath: String?
    public let targetPath: String?
    public let documentation: String

    public init(name: String, stubPath: String? = nil, targetPath: String?, documentation: String = "") {
        self.name = name
        self.stubPath = stubPath
        self.targetPath = targetPath
        self.documentation = documentation
    }
}

public struct HardenerMetadata: Codable, Equatable, Sendable {
    public let name: String
    public let documentation: String
    public let hardened: Bool
    public let stubPath: String?
    public let targetPath: String?
    public let secretGate: SecretGateDescriptor?

    public init(
        name: String,
        documentation: String = "",
        hardened: Bool = false,
        stubPath: String? = nil,
        targetPath: String? = nil,
        secretGate: SecretGateDescriptor? = nil
    ) {
        self.name = name
        self.documentation = documentation
        self.hardened = hardened
        self.stubPath = stubPath
        self.targetPath = targetPath
        self.secretGate = secretGate
    }

    enum CodingKeys: String, CodingKey {
        case name
        case documentation
        case hardened
        case stubPath = "stub_path"
        case targetPath = "target_path"
        case secretGate = "secret_gate"
    }
}

public struct SecretGateDescriptor: Codable, Equatable, Sendable {
    public let id: String
    public let keyPatterns: [String]
    public let routes: [SecretGateRoute]

    public init(id: String, keyPatterns: [String], routes: [SecretGateRoute]) {
        self.id = id
        self.keyPatterns = keyPatterns
        self.routes = routes
    }

    enum CodingKeys: String, CodingKey {
        case id
        case keyPatterns = "key_patterns"
        case routes
    }
}

public struct SecretGateRoute: Codable, Equatable, Sendable {
    public let operation: String
    public let scriptPath: String?
    public let targetPath: String
    public let callerIdentifiers: [String]
    public let keyPatterns: [String]
    public let replaceExistingEnv: Bool
    public let allowMissingKeys: Bool

    public init(
        operation: String,
        scriptPath: String?,
        targetPath: String,
        callerIdentifiers: [String],
        keyPatterns: [String],
        replaceExistingEnv: Bool,
        allowMissingKeys: Bool
    ) {
        self.operation = operation
        self.scriptPath = scriptPath
        self.targetPath = targetPath
        self.callerIdentifiers = callerIdentifiers
        self.keyPatterns = keyPatterns
        self.replaceExistingEnv = replaceExistingEnv
        self.allowMissingKeys = allowMissingKeys
    }

    enum CodingKeys: String, CodingKey {
        case operation
        case scriptPath = "script_path"
        case targetPath = "target_path"
        case callerIdentifiers = "caller_identifiers"
        case keyPatterns = "key_patterns"
        case replaceExistingEnv = "replace_existing_env"
        case allowMissingKeys = "allow_missing_keys"
    }
}

public enum SecretGateProtection: String, Codable, CaseIterable, Identifiable, Sendable {
    case noAccess
    case readOnly
    case fullExceptSecretDumps
    case fullIncludingSecretDumps

    public var id: String { rawValue }

    public var title: String {
        switch self {
        case .noAccess: "No Access"
        case .readOnly: "Read Only Access"
        case .fullExceptSecretDumps: "Trusted Access"
        case .fullIncludingSecretDumps: "Full Access"
        }
    }

    public var subtitle: String {
        switch self {
        case .noAccess: "All authenticated commands have approval gates"
        case .readOnly: "Commands without side-effects are approved automatically"
        case .fullExceptSecretDumps: "All commands are approved automatically except those that might exfiltrate secrets"
        case .fullIncludingSecretDumps: "All commands are approved automatically"
        }
    }

    public func allows(_ classification: SecretGateRequestClassification) -> Bool {
        switch self {
        case .noAccess:
            false
        case .readOnly:
            classification == .readOnly
        case .fullExceptSecretDumps:
            classification != .secretDump
        case .fullIncludingSecretDumps:
            true
        }
    }
}

public enum SecretGateRequestClassification: CaseIterable, Sendable {
    case readOnly
    case mutating
    case secretDump
    case unknown
}

public struct SecretGatePolicy: Equatable, Sendable {
    public let bundleIdentifier: String
    public let requirement: String
    public let protection: SecretGateProtection

    public init(bundleIdentifier: String, requirement: String, protection: SecretGateProtection) {
        self.bundleIdentifier = bundleIdentifier
        self.requirement = requirement
        self.protection = protection
    }
}

public struct SecretGate: Equatable, Identifiable, Sendable {
    public let id: String
    public let keyPatterns: [String]
    public let routes: [SecretGateRoute]
    public let defaultProtection: SecretGateProtection
    public let appPolicies: [SecretGatePolicy]

    public init(
        id: String,
        keyPatterns: [String],
        routes: [SecretGateRoute],
        defaultProtection: SecretGateProtection,
        appPolicies: [SecretGatePolicy]
    ) {
        self.id = id
        self.keyPatterns = keyPatterns
        self.routes = routes
        self.defaultProtection = defaultProtection
        self.appPolicies = appPolicies
    }

    public var scriptPaths: [String] { routes.compactMap(\.scriptPath).uniqueSorted() }
    public var targetPaths: [String] { routes.map(\.targetPath).uniqueSorted() }
}

public struct StoredSecret: Equatable, Sendable {
    public let account: String
    public let keychainProperties: [String]

    public init(account: String, keychainProperties: [String] = []) {
        self.account = account
        self.keychainProperties = keychainProperties
    }

    public var subtitle: String {
        keychainProperties.isEmpty ? "Keychain secret" : keychainProperties.joined(separator: " • ")
    }
}

public struct AccessRequestRecord: Codable, Equatable, Identifiable, Sendable {
    public let id: UUID
    public let date: Date
    public let tool: String
    public let command: String
    public let decision: String
    public let approvalSource: String?
    public let reason: String
    public let launcher: String?
    public let callerPath: String
    public let target: String
    public let cwd: String
    public let keys: [String]
    public let detail: String?

    public init(
        id: UUID = UUID(),
        date: Date,
        tool: String,
        command: String,
        decision: String,
        approvalSource: String? = nil,
        reason: String,
        launcher: String?,
        callerPath: String,
        target: String,
        cwd: String,
        keys: [String],
        detail: String?
    ) {
        self.id = id
        self.date = date
        self.tool = tool
        self.command = command
        self.decision = decision
        self.approvalSource = approvalSource
        self.reason = reason
        self.launcher = launcher
        self.callerPath = callerPath
        self.target = target
        self.cwd = cwd
        self.keys = keys
        self.detail = detail
    }

    public var approvalSourceLabel: String {
        if let approvalSource, !approvalSource.isEmpty {
            return approvalSource
        }
        if reason.localizedCaseInsensitiveContains("auto") || reason.localizedCaseInsensitiveContains("reused") {
            return "Auto"
        }
        if reason.localizedCaseInsensitiveContains("prompt") {
            return "Human"
        }
        return "Unknown"
    }
}

struct ScanReport: Codable {
    let findings: [DetectorFinding]
}

struct DetectorReport: Codable {
    let detectors: [DetectorMetadata]
}

struct HardenerReport: Codable {
    let hardeners: [HardenerMetadata]
}

public func detectorFindings(from scanJSON: Data) throws -> [DetectorFinding] {
    try JSONDecoder().decode(ScanReport.self, from: scanJSON).findings
}

public func detectorMetadata(from detectorsJSON: Data) throws -> [DetectorMetadata] {
    try JSONDecoder().decode(DetectorReport.self, from: detectorsJSON).detectors
}

public func hardenerMetadata(from hardenersJSON: Data) throws -> [HardenerMetadata] {
    try JSONDecoder().decode(HardenerReport.self, from: hardenersJSON).hardeners
}

public func hardenerNameReferencedByDocumentation(_ documentation: String) -> String? {
    guard let range = documentation.range(
        of: #"av[ \t\r\n]+harden[ \t\r\n]+[A-Za-z0-9_-]+"#,
        options: .regularExpression
    ) else {
        return nil
    }
    return documentation[range].split(whereSeparator: \.isWhitespace).last.map(String.init)
}

public func loadHardenedTools(
    in directory: URL,
    ghCLIURL: URL? = URL(fileURLWithPath: "/opt/homebrew/opt/gh-cli/bin/gh"),
    metadata: [HardenerMetadata] = []
) -> [HardenedTool] {
    _ = directory
    _ = ghCLIURL
    return metadata.filter(\.hardened).map {
        HardenedTool(
            name: $0.name,
            stubPath: $0.stubPath,
            targetPath: $0.targetPath,
            documentation: $0.documentation
        )
    }
        .uniquedByName()
        .sorted { $0.name.localizedStandardCompare($1.name) == .orderedAscending }
}

public func loadSecretGates(
    hardeners: [HardenerMetadata] = [],
    service: String = secretGatePoliciesKeychainService,
    account: String = secretGatePoliciesKeychainAccount
) -> [SecretGate] {
    let records = loadSecretGatePolicyRecords(service: service, account: account)
    return hardeners.compactMap { hardener in
        guard hardener.hardened, let descriptor = hardener.secretGate else { return nil }
        let gateRecords = records.filter { $0.gateID == descriptor.id }
        return SecretGate(
            id: descriptor.id,
            keyPatterns: descriptor.keyPatterns.uniqueSorted(),
            routes: descriptor.routes,
            defaultProtection: gateRecords.last(where: { $0.requirement == nil })?.protection ?? .noAccess,
            appPolicies: gateRecords.compactMap { record in
                record.requirement.map {
                    SecretGatePolicy(
                        bundleIdentifier: appIdentifier(from: $0) ?? "unknown",
                        requirement: $0,
                        protection: record.protection
                    )
                }
            }.uniqueSorted()
        )
    }
    .sorted { $0.id.localizedStandardCompare($1.id) == .orderedAscending }
}

public func normalizedExecutablePath(_ path: String) -> String {
    normalizedExecutablePath(path) {
        try? FileManager.default.destinationOfSymbolicLink(atPath: $0)
    }
}

func normalizedExecutablePath(_ path: String, symlinkDestination: (String) -> String?) -> String {
    let standardized = URL(fileURLWithPath: path).standardizedFileURL.path
    if let path = normalizedHomebrewCellarExecutablePath(standardized) {
        return path
    }

    let url = URL(fileURLWithPath: standardized)
    guard url.deletingLastPathComponent().path == "/opt/homebrew/bin",
          let destination = symlinkDestination(standardized)
    else {
        return standardized
    }

    let resolved = destination.hasPrefix("/")
        ? URL(fileURLWithPath: destination).standardizedFileURL.path
        : url.deletingLastPathComponent().appendingPathComponent(destination).standardizedFileURL.path
    guard resolved != standardized else { return standardized }
    return normalizedExecutablePath(resolved, symlinkDestination: symlinkDestination)
}

private func normalizedHomebrewCellarExecutablePath(_ path: String) -> String? {
    let components = URL(fileURLWithPath: path).standardizedFileURL.pathComponents
    guard components.count == 8,
          components[0] == "/",
          components[1] == "opt",
          components[2] == "homebrew",
          components[3] == "Cellar",
          components[6] == "bin"
    else {
        return nil
    }
    return "/opt/homebrew/opt/\(components[4])/bin/\(components[7])"
}

public func setSecretGateDefaultProtection(
    _ protection: SecretGateProtection,
    for gate: SecretGate,
    service: String = secretGatePoliciesKeychainService,
    account: String = secretGatePoliciesKeychainAccount
) -> OSStatus {
    setSecretGatePolicyRecord(
        SecretGatePolicyRecord(gateID: gate.id, requirement: nil, protection: protection),
        service: service,
        account: account
    )
}

public func setSecretGateAppProtection(
    requirement: String,
    protection: SecretGateProtection,
    for gate: SecretGate,
    service: String = secretGatePoliciesKeychainService,
    account: String = secretGatePoliciesKeychainAccount
) -> OSStatus {
    setSecretGatePolicyRecord(
        SecretGatePolicyRecord(gateID: gate.id, requirement: requirement, protection: protection),
        service: service,
        account: account
    )
}

public func removeSecretGateAppPolicy(
    _ policy: SecretGatePolicy,
    from gate: SecretGate,
    service: String = secretGatePoliciesKeychainService,
    account: String = secretGatePoliciesKeychainAccount
) -> OSStatus {
    let records = loadSecretGatePolicyRecords(service: service, account: account).filter {
        !($0.gateID == gate.id && $0.requirement == policy.requirement)
    }
    return saveSecretGatePolicyRecords(records, service: service, account: account)
}

public func secretGateProtection(
    for requirement: String?,
    in gate: SecretGate
) -> (protection: SecretGateProtection, source: String) {
    if let requirement, let policy = gate.appPolicies.first(where: { $0.requirement == requirement }) {
        return (policy.protection, policy.bundleIdentifier)
    }
    return (gate.defaultProtection, "All Other Apps")
}

private struct SecretGatePolicyRecord: Codable, Equatable {
    let gateID: String
    let requirement: String?
    let protection: SecretGateProtection
}

private func loadSecretGatePolicyRecords(
    service: String,
    account: String
) -> [SecretGatePolicyRecord] {
    guard let data = loadKeychainData(service: service, account: account) else { return [] }
    do {
        return try JSONDecoder().decode([SecretGatePolicyRecord].self, from: data)
    } catch {
        return []
    }
}

private func saveSecretGatePolicyRecords(
    _ records: [SecretGatePolicyRecord],
    service: String,
    account: String
) -> OSStatus {
    if records.isEmpty {
        let status = deleteStoredSecret(account: account, service: service)
        return status == errSecItemNotFound ? errSecSuccess : status
    }
    do {
        let sorted = records.sorted {
            [$0.gateID, $0.requirement ?? ""].joined(separator: "\u{1f}")
                .localizedStandardCompare([$1.gateID, $1.requirement ?? ""].joined(separator: "\u{1f}")) == .orderedAscending
        }
        return saveKeychainData(try JSONEncoder().encode(sorted), service: service, account: account)
    } catch {
        return errSecParam
    }
}

private func setSecretGatePolicyRecord(
    _ record: SecretGatePolicyRecord,
    service: String,
    account: String
) -> OSStatus {
    var records = loadSecretGatePolicyRecords(service: service, account: account)
    records.removeAll { $0.gateID == record.gateID && $0.requirement == record.requirement }
    if record.requirement != nil || record.protection != .noAccess {
        records.append(record)
    }
    return saveSecretGatePolicyRecords(records, service: service, account: account)
}

public func loadAccessRequestRecords(
    defaults: UserDefaults = .standard,
    key: String = accessRequestLogDefaultsKey
) -> [AccessRequestRecord] {
    guard let data = defaults.data(forKey: key),
          let records = try? JSONDecoder().decode([AccessRequestRecord].self, from: data)
    else {
        return []
    }
    return Array(records.prefix(50))
}

public func appendAccessRequestRecord(
    _ record: AccessRequestRecord,
    defaults: UserDefaults = .standard,
    key: String = accessRequestLogDefaultsKey
) {
    let records = Array(([record] + loadAccessRequestRecords(defaults: defaults, key: key)).prefix(50))
    if let data = try? JSONEncoder().encode(records) {
        defaults.set(data, forKey: key)
    }
}

public func loadStoredSecrets(service: String = automicVaultKeychainService) -> [StoredSecret] {
    let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: service,
        kSecUseDataProtectionKeychain as String: true,
        kSecReturnAttributes as String: true,
        kSecMatchLimit as String: kSecMatchLimitAll,
    ]
    var result: CFTypeRef?
    guard SecItemCopyMatching(query as CFDictionary, &result) == errSecSuccess,
          let items = result as? [[String: Any]]
    else {
        return []
    }
    return items.compactMap { item in
        guard let account = item[kSecAttrAccount as String] as? String else { return nil }
        return StoredSecret(account: account, keychainProperties: keychainProperties(for: item, dataProtection: true))
    }
    .sorted { $0.account.localizedStandardCompare($1.account) == .orderedAscending }
}

private func keychainProperties(for item: [String: Any], dataProtection: Bool) -> [String] {
    [
        dataProtection ? "Data Protection Enabled" : nil,
        isSynchronizable(item[kSecAttrSynchronizable as String]) ? "iCloud On" : "iCloud Off",
        accessibleLabel(item[kSecAttrAccessible as String]),
    ].compactMap(\.self)
}

private func isSynchronizable(_ value: Any?) -> Bool {
    if let value = value as? Bool {
        return value
    }
    if let value = value as? NSNumber {
        return value.boolValue
    }
    return false
}

private func accessibleLabel(_ value: Any?) -> String? {
    guard let value = value as? String else { return nil }
    let whenUnlocked = kSecAttrAccessibleWhenUnlocked as String
    let afterFirstUnlock = kSecAttrAccessibleAfterFirstUnlock as String
    let passcodeSetThisDeviceOnly = kSecAttrAccessibleWhenPasscodeSetThisDeviceOnly as String
    let whenUnlockedThisDeviceOnly = kSecAttrAccessibleWhenUnlockedThisDeviceOnly as String
    let afterFirstUnlockThisDeviceOnly = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly as String
    return switch value {
    case whenUnlocked:
        "When Unlocked"
    case afterFirstUnlock:
        "After First Unlock"
    case passcodeSetThisDeviceOnly:
        "Passcode Set, This Device Only"
    case whenUnlockedThisDeviceOnly:
        "When Unlocked, This Device Only"
    case afterFirstUnlockThisDeviceOnly:
        "After First Unlock, This Device Only"
    default:
        nil
    }
}

public func saveStoredSecret(account: String, value: String, service: String = automicVaultKeychainService) -> OSStatus {
    saveKeychainData(Data(value.utf8), service: service, account: account)
}

public func renameStoredSecret(account: String, to newAccount: String, service: String = automicVaultKeychainService) -> OSStatus {
    let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: service,
        kSecAttrAccount as String: account,
        kSecUseDataProtectionKeychain as String: true,
    ]
    return SecItemUpdate(query as CFDictionary, [kSecAttrAccount as String: newAccount] as CFDictionary)
}

public func deleteStoredSecret(account: String, service: String = automicVaultKeychainService) -> OSStatus {
    let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: service,
        kSecAttrAccount as String: account,
        kSecUseDataProtectionKeychain as String: true,
    ]
    return SecItemDelete(query as CFDictionary)
}

public func loadStoredSecret(account: String, service: String = automicVaultKeychainService) -> String? {
    guard let data = loadKeychainData(service: service, account: account) else { return nil }
    return String(data: data, encoding: .utf8)
}

private func loadKeychainData(service: String, account: String) -> Data? {
    let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: service,
        kSecAttrAccount as String: account,
        kSecUseDataProtectionKeychain as String: true,
        kSecReturnData as String: true,
    ]
    var result: CFTypeRef?
    guard SecItemCopyMatching(query as CFDictionary, &result) == errSecSuccess else { return nil }
    return result as? Data
}

private func saveKeychainData(_ data: Data, service: String, account: String) -> OSStatus {
    let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: service,
        kSecAttrAccount as String: account,
        kSecUseDataProtectionKeychain as String: true,
    ]
    let attributes: [String: Any] = [
        kSecValueData as String: data,
        kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlocked,
    ]
    let status = SecItemUpdate(query as CFDictionary, attributes as CFDictionary)
    if status != errSecItemNotFound {
        return status
    }
    var addQuery = query
    addQuery[kSecValueData as String] = data
    addQuery[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlocked
    return SecItemAdd(addQuery as CFDictionary, nil)
}

func appIdentifier(from requirement: String) -> String? {
    guard let range = requirement.range(of: #"identifier ""#) else { return nil }
    let rest = requirement[range.upperBound...]
    guard let end = rest.firstIndex(of: "\"") else { return nil }
    return String(rest[..<end])
}

private extension Array where Element == HardenedTool {
    func uniquedByName() -> [HardenedTool] {
        var seen = Set<String>()
        return filter { seen.insert($0.name).inserted }
    }
}

private extension Array where Element == String {
    func uniqueSorted() -> [String] {
        Array(Set(self)).sorted { $0.localizedStandardCompare($1) == .orderedAscending }
    }
}

private extension Array where Element == SecretGatePolicy {
    func uniqueSorted() -> [SecretGatePolicy] {
        var seen = Set<String>()
        return filter { seen.insert($0.requirement).inserted }
            .sorted {
                [$0.bundleIdentifier, $0.requirement].joined(separator: "\u{1f}")
                    .localizedStandardCompare([$1.bundleIdentifier, $1.requirement].joined(separator: "\u{1f}")) == .orderedAscending
            }
    }
}

func scanDetectorFindings(avExecutableURL: URL) -> [DetectorFinding] {
    loadJSON(avExecutableURL: avExecutableURL, arguments: ["scan", "--json"])
        .flatMap { try? detectorFindings(from: $0) } ?? []
}

public func loadDetectorMetadata(avExecutableURL: URL) -> [DetectorMetadata] {
    loadJSON(avExecutableURL: avExecutableURL, arguments: ["detectors", "--json"])
        .flatMap { try? detectorMetadata(from: $0) } ?? []
}

public func loadHardenerMetadata(avExecutableURL: URL) -> [HardenerMetadata] {
    loadJSON(avExecutableURL: avExecutableURL, arguments: ["hardeners", "--json"])
        .flatMap { try? hardenerMetadata(from: $0) } ?? []
}

func loadJSON(avExecutableURL: URL, arguments: [String]) -> Data? {
    let process = Process()
    process.executableURL = avExecutableURL
    process.arguments = arguments

    let output = Pipe()
    process.standardOutput = output
    process.standardError = Pipe()

    do {
        try process.run()
    } catch {
        return nil
    }

    let data = output.fileHandleForReading.readDataToEndOfFile()
    process.waitUntilExit()
    guard process.terminationStatus == 0 else { return nil }
    return data
}

public func defaultAVExecutableURL() -> URL {
    if let bundled = Bundle.main.executableURL?.deletingLastPathComponent().appendingPathComponent("av"),
       FileManager.default.isExecutableFile(atPath: bundled.path)
    {
        return bundled
    }
    return URL(fileURLWithPath: "/usr/local/bin/av")
}
