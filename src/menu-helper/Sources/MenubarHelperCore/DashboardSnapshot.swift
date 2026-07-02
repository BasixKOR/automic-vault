import Foundation
import Security

public let automicVaultKeychainService = "com.automicvault.isotope"
public let trustedScriptApprovalsKeychainService = "com.automicvault.approvals"
public let trustedScriptApprovalsKeychainAccount = "TrustedLauncherScriptApprovals"

public struct DashboardSnapshot: Equatable, Sendable {
    public var detectors: [DetectorMetadata]
    public var detectorFindings: [DetectorFinding]
    public var hardenedTools: [HardenedTool]
    public var secretGates: [SecretGate]
    public var secrets: [StoredSecret]

    public static let empty = DashboardSnapshot(
        detectors: [],
        detectorFindings: [],
        hardenedTools: [],
        secretGates: [],
        secrets: []
    )

    public var flaggedDetectorCount: Int {
        Set(detectorFindings.map(\.source)).count
    }

    public static func load(
        avExecutableURL: URL = defaultAVExecutableURL(),
        stubDirectory: URL = URL(fileURLWithPath: "/usr/local/bin", isDirectory: true),
        ghCLIURL: URL? = URL(fileURLWithPath: "/opt/homebrew/opt/gh-cli/bin/gh"),
        approvalService: String = trustedScriptApprovalsKeychainService
    ) -> DashboardSnapshot {
        DashboardSnapshot(
            detectors: loadDetectorMetadata(avExecutableURL: avExecutableURL),
            detectorFindings: scanDetectorFindings(avExecutableURL: avExecutableURL),
            hardenedTools: loadHardenedTools(in: stubDirectory, ghCLIURL: ghCLIURL),
            secretGates: loadSecretGates(service: approvalService),
            secrets: loadStoredSecrets()
        )
    }
}

public struct DetectorMetadata: Codable, Equatable, Sendable {
    public let name: String
    public let homepage: String
    public let docsURL: String

    public init(name: String, homepage: String, docsURL: String) {
        self.name = name
        self.homepage = homepage
        self.docsURL = docsURL
    }

    enum CodingKeys: String, CodingKey {
        case name
        case homepage
        case docsURL = "docs_url"
    }
}

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
    public let stubPath: String
    public let targetPath: String?
}

public struct SecretGate: Equatable, Sendable {
    public let scriptPath: String
    public let keys: [String]
    public let target: String
    public let approvedApps: [String]
}

public struct StoredSecret: Equatable, Sendable {
    public let account: String
}

struct ScanReport: Codable {
    let findings: [DetectorFinding]
}

struct DetectorReport: Codable {
    let detectors: [DetectorMetadata]
}

public struct TrustedScriptApproval: Codable, Equatable, Sendable {
    public let scriptPath: String
    public let scriptChecksum: String
    public let keys: [String]
    public let target: String
    public let replaceExistingEnv: Bool
    public let allowMissingKeys: Bool
    public let launcherRequirement: String

    public init(
        scriptPath: String,
        scriptChecksum: String,
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

public func loadHardenedTools(
    in directory: URL,
    ghCLIURL: URL? = URL(fileURLWithPath: "/opt/homebrew/opt/gh-cli/bin/gh")
) -> [HardenedTool] {
    let fileManager = FileManager.default
    let urls = (try? fileManager.contentsOfDirectory(at: directory, includingPropertiesForKeys: [.isRegularFileKey])) ?? []
    var tools: [HardenedTool] = urls.compactMap { url in
        guard (try? url.resourceValues(forKeys: [.isRegularFileKey]).isRegularFile) == true,
              let contents = try? String(contentsOf: url, encoding: .utf8)
        else {
            return nil
        }
        if url.lastPathComponent == "aws", contents.hasPrefix("#!/usr/local/bin/av inject "), contents.contains("aws-vault") {
            return HardenedTool(name: "aws", stubPath: url.path, targetPath: "/opt/homebrew/bin/aws")
        }
        guard contents.split(whereSeparator: \.isNewline).dropFirst().first == "# Automic Vault hardened stub" else {
            return nil
        }
        return HardenedTool(
            name: url.lastPathComponent,
            stubPath: url.path,
            targetPath: hardenedTargetPath(from: contents)
        )
    }
    if let ghCLIURL, fileManager.isExecutableFile(atPath: ghCLIURL.path) {
        tools.append(HardenedTool(name: "gh-cli", stubPath: ghCLIURL.path, targetPath: "gh auth av-migrate"))
    }
    return tools
        .uniquedByName()
        .sorted { $0.name.localizedStandardCompare($1.name) == .orderedAscending }
}

public func loadSecretGates(service: String = trustedScriptApprovalsKeychainService) -> [SecretGate] {
    let approvals = loadTrustedScriptApprovals(service: service)
    let grouped = Dictionary(grouping: approvals) {
        "\($0.scriptPath)\u{1f}\($0.target)\u{1f}\($0.keys.sorted().joined(separator: "\u{1e}"))"
    }
    return grouped.values.compactMap { approvals in
        guard let first = approvals.first else { return nil }
        return SecretGate(
            scriptPath: first.scriptPath,
            keys: first.keys.sorted(),
            target: first.target,
            approvedApps: approvals.map(\.launcherRequirement).compactMap(appIdentifier).uniqueSorted()
        )
    }
    .sorted { $0.scriptPath.localizedStandardCompare($1.scriptPath) == .orderedAscending }
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

public func loadStoredSecrets(service: String = automicVaultKeychainService) -> [StoredSecret] {
    let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: service,
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
        (item[kSecAttrAccount as String] as? String).map(StoredSecret.init(account:))
    }
    .sorted { $0.account.localizedStandardCompare($1.account) == .orderedAscending }
}

public func saveStoredSecret(account: String, value: String, service: String = automicVaultKeychainService) -> OSStatus {
    saveKeychainData(Data(value.utf8), service: service, account: account)
}

public func deleteStoredSecret(account: String, service: String = automicVaultKeychainService) -> OSStatus {
    let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: service,
        kSecAttrAccount as String: account,
    ]
    return SecItemDelete(query as CFDictionary)
}

private func loadKeychainData(service: String, account: String) -> Data? {
    let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: service,
        kSecAttrAccount as String: account,
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
    ]
    let attributes: [String: Any] = [
        kSecValueData as String: data,
    ]
    let status = SecItemUpdate(query as CFDictionary, attributes as CFDictionary)
    if status != errSecItemNotFound {
        return status
    }
    var addQuery = query
    addQuery[kSecValueData as String] = data
    return SecItemAdd(addQuery as CFDictionary, nil)
}

func hardenedTargetPath(from script: String) -> String? {
    guard let line = script.split(whereSeparator: \.isNewline).first(where: { $0.contains(" stub-exec ") }) else {
        return nil
    }
    let quoted = line.split(separator: "'", omittingEmptySubsequences: false)
    guard quoted.count >= 4 else { return nil }
    return String(quoted[3]).replacingOccurrences(of: "'\\''", with: "'")
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

func scanDetectorFindings(avExecutableURL: URL) -> [DetectorFinding] {
    loadJSON(avExecutableURL: avExecutableURL, arguments: ["scan", "--json"])
        .flatMap { try? detectorFindings(from: $0) } ?? []
}

func loadDetectorMetadata(avExecutableURL: URL) -> [DetectorMetadata] {
    loadJSON(avExecutableURL: avExecutableURL, arguments: ["detectors", "--json"])
        .flatMap { try? detectorMetadata(from: $0) } ?? []
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
