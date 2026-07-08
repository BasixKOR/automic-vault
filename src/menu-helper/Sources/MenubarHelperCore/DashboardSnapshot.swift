import CryptoKit
import Foundation
import Security

public let automicVaultKeychainService = "com.automicvault.isotope"
public let trustedScriptApprovalsKeychainService = "com.automicvault.approvals"
public let trustedScriptApprovalsKeychainAccount = "TrustedLauncherScriptApprovals"
public let ghReadOnlyAutoApprovalDefaultsKey = "GhReadOnlyAutoApproval"
public let awsReadOnlyAutoApprovalDefaultsKey = "AwsReadOnlyAutoApproval"
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
        approvalService: String = trustedScriptApprovalsKeychainService
    ) -> DashboardSnapshot {
        let hardenerMetadata = loadHardenerMetadata(avExecutableURL: avExecutableURL)
        let hardenedTools = loadHardenedTools(
            in: stubDirectory,
            ghCLIURL: ghCLIURL,
            metadata: hardenerMetadata
        )
        return DashboardSnapshot(
            detectors: loadDetectorMetadata(avExecutableURL: avExecutableURL),
            detectorFindings: scanDetectorFindings(avExecutableURL: avExecutableURL),
            hardenedTools: hardenedTools,
            hardeners: hardenerMetadata,
            secretGates: loadSecretGates(configuredTools: hardenedTools, service: approvalService),
            secrets: loadStoredSecrets(),
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

    public init(
        name: String,
        documentation: String = "",
        hardened: Bool = false,
        stubPath: String? = nil,
        targetPath: String? = nil
    ) {
        self.name = name
        self.documentation = documentation
        self.hardened = hardened
        self.stubPath = stubPath
        self.targetPath = targetPath
    }

    enum CodingKeys: String, CodingKey {
        case name
        case documentation
        case hardened
        case stubPath = "stub_path"
        case targetPath = "target_path"
    }
}

public struct SecretGate: Equatable, Sendable {
    public let scriptPath: String
    public let scriptChecksum: String
    public let keys: [String]
    public let target: String
    public let replaceExistingEnv: Bool
    public let allowMissingKeys: Bool
    public let approvedApps: [SecretGateApprovedApp]

    public var id: String {
        [
            scriptPath,
            scriptChecksum,
            target,
            keys.sorted().joined(separator: "\u{1e}"),
            replaceExistingEnv.description,
            allowMissingKeys.description,
        ].joined(separator: "\u{1f}")
    }
}

public struct SecretGateApprovedApp: Equatable, Sendable {
    public let bundleIdentifier: String
    public let requirement: String

    public init(bundleIdentifier: String, requirement: String) {
        self.bundleIdentifier = bundleIdentifier
        self.requirement = requirement
    }
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

public struct TrustedScriptApproval: Codable, Equatable, Sendable {
    public let scriptPath: String?
    public let scriptChecksum: String?
    public let keys: [String]
    public let target: String
    public let replaceExistingEnv: Bool
    public let allowMissingKeys: Bool
    public let launcherRequirement: String

    public init(
        scriptPath: String?,
        scriptChecksum: String?,
        keys: [String],
        target: String,
        replaceExistingEnv: Bool,
        allowMissingKeys: Bool,
        launcherRequirement: String
    ) {
        self.scriptPath = scriptPath
        self.scriptChecksum = scriptChecksum
        self.keys = keys
        self.target = target
        self.replaceExistingEnv = replaceExistingEnv
        self.allowMissingKeys = allowMissingKeys
        self.launcherRequirement = launcherRequirement
    }
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
    configuredTools: [HardenedTool] = [],
    service: String = trustedScriptApprovalsKeychainService
) -> [SecretGate] {
    var gates = Dictionary(uniqueKeysWithValues: configuredSecretGates(from: configuredTools).map { ($0.id, $0) })
    let approvals = loadTrustedScriptApprovals(service: service)
    for gate in secretGates(from: approvals) {
        if let configured = gates[gate.id] {
            gates[gate.id] = SecretGate(
                scriptPath: configured.scriptPath,
                scriptChecksum: configured.scriptChecksum,
                keys: configured.keys,
                target: configured.target,
                replaceExistingEnv: configured.replaceExistingEnv,
                allowMissingKeys: configured.allowMissingKeys,
                approvedApps: gate.approvedApps
            )
        } else {
            gates[gate.id] = gate
        }
    }
    return gates.values.sorted { $0.id.localizedStandardCompare($1.id) == .orderedAscending }
}

private func secretGates(from approvals: [TrustedScriptApproval]) -> [SecretGate] {
    let grouped = Dictionary(grouping: approvals.filter { $0.scriptPath != nil && $0.scriptChecksum != nil }) {
        "\($0.scriptPath ?? "")\u{1f}\($0.scriptChecksum ?? "")\u{1f}\($0.target)\u{1f}\($0.keys.sorted().joined(separator: "\u{1e}"))\u{1f}\($0.replaceExistingEnv)\u{1f}\($0.allowMissingKeys)"
    }
    return grouped.values.compactMap { approvals in
        guard let first = approvals.first,
              let scriptPath = first.scriptPath,
              let scriptChecksum = first.scriptChecksum
        else { return nil }
        return SecretGate(
            scriptPath: scriptPath,
            scriptChecksum: scriptChecksum,
            keys: first.keys.sorted(),
            target: first.target,
            replaceExistingEnv: first.replaceExistingEnv,
            allowMissingKeys: first.allowMissingKeys,
            approvedApps: approvals.map {
                SecretGateApprovedApp(
                    bundleIdentifier: appIdentifier(from: $0.launcherRequirement) ?? "unknown",
                    requirement: $0.launcherRequirement
                )
            }.uniqueSorted()
        )
    }
    .sorted { $0.id.localizedStandardCompare($1.id) == .orderedAscending }
}

private func configuredSecretGates(from tools: [HardenedTool]) -> [SecretGate] {
    tools.compactMap { tool in
        guard let stubPath = tool.stubPath,
              let data = try? Data(contentsOf: URL(fileURLWithPath: stubPath)),
              let contents = String(data: data, encoding: .utf8),
              let firstLine = contents.split(separator: "\n", maxSplits: 1, omittingEmptySubsequences: false).first.map(String.init),
              let injection = parseInjectShebang(firstLine)
        else {
            return nil
        }
        return SecretGate(
            scriptPath: URL(fileURLWithPath: stubPath).standardizedFileURL.path,
            scriptChecksum: SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined(),
            keys: injection.keys,
            target: injection.target,
            replaceExistingEnv: injection.replaceExistingEnv,
            allowMissingKeys: injection.allowMissingKeys,
            approvedApps: []
        )
    }
}

private func parseInjectShebang(_ line: String) -> (keys: [String], target: String, replaceExistingEnv: Bool, allowMissingKeys: Bool)? {
    guard line.hasPrefix("#!") else { return nil }
    let parts = line.dropFirst(2).split(whereSeparator: \.isWhitespace).map(String.init)
    guard let injectIndex = parts.firstIndex(of: "inject") else { return nil }
    var replaceExistingEnv = false
    var allowMissingKeys = false
    var keys: [String] = []
    var index = parts.index(after: injectIndex)
    while index < parts.endIndex {
        let part = parts[index]
        if part == "--replace-existing-env" {
            replaceExistingEnv = true
        } else if part == "--allow-missing-keys" {
            allowMissingKeys = true
        } else if part.hasPrefix("+") {
            keys.append(String(part.dropFirst()))
        } else if part == "--" {
            index = parts.index(after: index)
            break
        } else {
            break
        }
        index = parts.index(after: index)
    }
    guard !keys.isEmpty, index < parts.endIndex else { return nil }
    return (
        keys: keys.uniqueSorted(),
        target: resolvedExecutable(parts[index]),
        replaceExistingEnv: replaceExistingEnv,
        allowMissingKeys: allowMissingKeys
    )
}

private func resolvedExecutable(_ executable: String) -> String {
    if executable.contains("/") {
        return URL(fileURLWithPath: executable).standardizedFileURL.path
    }
    for directory in (ProcessInfo.processInfo.environment["PATH"] ?? "").split(separator: ":") {
        let path = URL(fileURLWithPath: String(directory)).appendingPathComponent(executable).path
        if FileManager.default.isExecutableFile(atPath: path) {
            return path
        }
    }
    return executable
}

public func rememberTrustedApp(
    requirement: String,
    for gate: SecretGate,
    service: String = trustedScriptApprovalsKeychainService,
    account: String = trustedScriptApprovalsKeychainAccount
) -> OSStatus {
    var approvals = loadTrustedScriptApprovals(service: service, account: account)
    let approval = TrustedScriptApproval(
        scriptPath: gate.scriptPath,
        scriptChecksum: gate.scriptChecksum,
        keys: gate.keys.sorted(),
        target: gate.target,
        replaceExistingEnv: gate.replaceExistingEnv,
        allowMissingKeys: gate.allowMissingKeys,
        launcherRequirement: requirement
    )
    if !approvals.contains(approval) {
        approvals.append(approval)
    }
    return saveTrustedScriptApprovals(approvals, service: service, account: account)
}

public func forgetTrustedApp(
    _ app: SecretGateApprovedApp,
    from gate: SecretGate,
    service: String = trustedScriptApprovalsKeychainService,
    account: String = trustedScriptApprovalsKeychainAccount
) -> OSStatus {
    let approvals = loadTrustedScriptApprovals(service: service, account: account).filter {
        !($0.matches(gate) && $0.launcherRequirement == app.requirement)
    }
    return saveTrustedScriptApprovals(approvals, service: service, account: account)
}

public func loadTrustedScriptApprovals(
    service: String = trustedScriptApprovalsKeychainService,
    account: String = trustedScriptApprovalsKeychainAccount
) -> [TrustedScriptApproval] {
    guard let data = loadKeychainData(service: service, account: account),
          let approvals = try? JSONDecoder().decode([TrustedScriptApproval].self, from: data)
    else {
        return []
    }
    return approvals
}

public func saveTrustedScriptApprovals(
    _ approvals: [TrustedScriptApproval],
    service: String = trustedScriptApprovalsKeychainService,
    account: String = trustedScriptApprovalsKeychainAccount
) -> OSStatus {
    do {
        return saveKeychainData(try JSONEncoder().encode(approvals), service: service, account: account)
    } catch {
        return errSecParam
    }
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

private extension Array where Element == SecretGateApprovedApp {
    func uniqueSorted() -> [SecretGateApprovedApp] {
        var seen = Set<String>()
        return filter { seen.insert($0.requirement).inserted }
            .sorted {
                [$0.bundleIdentifier, $0.requirement].joined(separator: "\u{1f}")
                    .localizedStandardCompare([$1.bundleIdentifier, $1.requirement].joined(separator: "\u{1f}")) == .orderedAscending
            }
    }
}

private extension TrustedScriptApproval {
    func matches(_ gate: SecretGate) -> Bool {
        scriptPath == Optional(gate.scriptPath)
            && scriptChecksum == Optional(gate.scriptChecksum)
            && keys.sorted() == gate.keys.sorted()
            && target == gate.target
            && replaceExistingEnv == gate.replaceExistingEnv
            && allowMissingKeys == gate.allowMissingKeys
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

func loadHardenerMetadata(avExecutableURL: URL) -> [HardenerMetadata] {
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
