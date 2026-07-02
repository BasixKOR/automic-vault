import Foundation

public let trustedScriptApprovalsDefaultsKey = "TrustedLauncherScriptApprovals"

public struct DashboardSnapshot: Equatable, Sendable {
    public var detectorFindings: [DetectorFinding]
    public var hardenedTools: [HardenedTool]
    public var secretGates: [SecretGate]

    public static let empty = DashboardSnapshot(
        detectorFindings: [],
        hardenedTools: [],
        secretGates: []
    )

    public var flaggedDetectorCount: Int {
        Set(detectorFindings.map(\.source)).count
    }

    public static func load(
        avExecutableURL: URL = defaultAVExecutableURL(),
        stubDirectory: URL = URL(fileURLWithPath: "/usr/local/bin", isDirectory: true),
        defaults: UserDefaults = .standard
    ) -> DashboardSnapshot {
        DashboardSnapshot(
            detectorFindings: scanDetectorFindings(avExecutableURL: avExecutableURL),
            hardenedTools: loadHardenedTools(in: stubDirectory),
            secretGates: loadSecretGates(defaults: defaults)
        )
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
}

struct ScanReport: Codable {
    let findings: [DetectorFinding]
}

struct TrustedScriptApproval: Codable {
    let scriptPath: String
    let scriptChecksum: String
    let keys: [String]
    let target: String
    let replaceExistingEnv: Bool
    let allowMissingKeys: Bool
    let launcherRequirement: String
}

public func detectorFindings(from scanJSON: Data) throws -> [DetectorFinding] {
    try JSONDecoder().decode(ScanReport.self, from: scanJSON).findings
}

public func loadHardenedTools(in directory: URL) -> [HardenedTool] {
    let fileManager = FileManager.default
    guard let urls = try? fileManager.contentsOfDirectory(at: directory, includingPropertiesForKeys: [.isRegularFileKey]) else {
        return []
    }

    return urls.compactMap { url in
        guard (try? url.resourceValues(forKeys: [.isRegularFileKey]).isRegularFile) == true,
              let contents = try? String(contentsOf: url, encoding: .utf8),
              contents.split(whereSeparator: \.isNewline).dropFirst().first == "# Automic Vault hardened stub"
        else {
            return nil
        }
        return HardenedTool(
            name: url.lastPathComponent,
            stubPath: url.path,
            targetPath: hardenedTargetPath(from: contents)
        )
    }
    .sorted { $0.name.localizedStandardCompare($1.name) == .orderedAscending }
}

public func loadSecretGates(defaults: UserDefaults = .standard) -> [SecretGate] {
    guard let data = defaults.data(forKey: trustedScriptApprovalsDefaultsKey),
          let approvals = try? JSONDecoder().decode([TrustedScriptApproval].self, from: data)
    else {
        return []
    }

    return approvals.map {
        SecretGate(scriptPath: $0.scriptPath, keys: $0.keys.sorted(), target: $0.target)
    }
    .sorted { $0.scriptPath.localizedStandardCompare($1.scriptPath) == .orderedAscending }
}

func hardenedTargetPath(from script: String) -> String? {
    guard let line = script.split(whereSeparator: \.isNewline).first(where: { $0.contains(" stub-exec ") }) else {
        return nil
    }
    let quoted = line.split(separator: "'", omittingEmptySubsequences: false)
    guard quoted.count >= 4 else { return nil }
    return String(quoted[3]).replacingOccurrences(of: "'\\''", with: "'")
}

func scanDetectorFindings(avExecutableURL: URL) -> [DetectorFinding] {
    let process = Process()
    process.executableURL = avExecutableURL
    process.arguments = ["scan", "--json"]

    let output = Pipe()
    process.standardOutput = output
    process.standardError = Pipe()

    do {
        try process.run()
    } catch {
        return []
    }

    let data = output.fileHandleForReading.readDataToEndOfFile()
    process.waitUntilExit()
    guard process.terminationStatus == 0 else { return [] }
    return (try? detectorFindings(from: data)) ?? []
}

public func defaultAVExecutableURL() -> URL {
    if let bundled = Bundle.main.executableURL?.deletingLastPathComponent().appendingPathComponent("av"),
       FileManager.default.isExecutableFile(atPath: bundled.path)
    {
        return bundled
    }
    return URL(fileURLWithPath: "/usr/local/bin/av")
}
