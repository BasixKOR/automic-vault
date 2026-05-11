import AppKit
import Foundation

enum HomebrewCaskCatalogError: Error, LocalizedError {
    case homebrewUnavailable
    case commandFailed(String)
    case invalidResponse(String)

    var errorDescription: String? {
        switch self {
        case .homebrewUnavailable:
            return "Homebrew is not installed."
        case .commandFailed(let message),
             .invalidResponse(let message):
            return message
        }
    }
}

struct HomebrewCaskOperationResult {
    let message: String
    let packageName: String
}

final class HomebrewCaskCatalog {
    private struct InfoReport: Decodable {
        let casks: [Cask]
    }

    struct PulseEvent: Equatable {
        let token: String
        let lastUpdatedAt: String
        let pulseKind: String
    }

    struct Cask: Decodable, Equatable {
        let token: String
        let fullToken: String?
        let names: [String]
        let description: String?
        let homepage: String?
        let version: String?
        let installedVersion: String?
        let artifacts: [Artifact]
        let deprecated: Bool
        let disabled: Bool
        let rubySourcePath: String?
        var pulseEvent: PulseEvent?

        enum CodingKeys: String, CodingKey {
            case token
            case fullToken = "full_token"
            case names = "name"
            case description = "desc"
            case homepage
            case version
            case installedVersion = "installed"
            case artifacts
            case deprecated
            case disabled
            case rubySourcePath = "ruby_source_path"
        }

        init(
            token: String,
            fullToken: String? = nil,
            names: [String] = [],
            description: String? = nil,
            homepage: String? = nil,
            version: String? = nil,
            installedVersion: String? = nil,
            artifacts: [Artifact],
            deprecated: Bool = false,
            disabled: Bool = false,
            rubySourcePath: String? = nil,
            pulseEvent: PulseEvent? = nil
        ) {
            self.token = token
            self.fullToken = fullToken
            self.names = names
            self.description = description
            self.homepage = homepage
            self.version = version
            self.installedVersion = installedVersion
            self.artifacts = artifacts
            self.deprecated = deprecated
            self.disabled = disabled
            self.rubySourcePath = rubySourcePath
            self.pulseEvent = pulseEvent
        }

        init(from decoder: Decoder) throws {
            let container = try decoder.container(keyedBy: CodingKeys.self)
            token = try container.decode(String.self, forKey: .token)
            fullToken = try container.decodeIfPresent(String.self, forKey: .fullToken)
            names = try container.decodeIfPresent([String].self, forKey: .names) ?? []
            description = try container.decodeIfPresent(String.self, forKey: .description)
            homepage = try container.decodeIfPresent(String.self, forKey: .homepage)
            version = try container.decodeIfPresent(String.self, forKey: .version)
            installedVersion = try container.decodeLossyStringIfPresent(forKey: .installedVersion)
            artifacts = try container.decodeIfPresent([Artifact].self, forKey: .artifacts) ?? []
            deprecated = try container.decodeIfPresent(Bool.self, forKey: .deprecated) ?? false
            disabled = try container.decodeIfPresent(Bool.self, forKey: .disabled) ?? false
            rubySourcePath = try container.decodeIfPresent(String.self, forKey: .rubySourcePath)
            pulseEvent = nil
        }

        var isGuiAppCask: Bool {
            !deprecated && !disabled && hasAppArtifact
        }

        var displayTitle: String {
            names.first(where: { !$0.isEmpty }) ?? token
        }

        var brewToken: String {
            fullToken?.isEmpty == false ? fullToken! : token
        }

        var packageName: String {
            "cask:\(brewToken)"
        }

        var installedDisplayVersion: String {
            installedVersion?.isEmpty == false
                ? installedVersion!
                : version ?? "installed"
        }

        var installRoot: String {
            guard let appName = artifacts.lazy.compactMap(\.appName).first else {
                return "/Applications"
            }
            return "/Applications/\(appName)"
        }

        var record: PackageRecord {
            PackageRecord(
                name: packageName,
                source: .cask(caskName: brewToken),
                version: installedDisplayVersion,
                description: description,
                latestVersion: version,
                securityState: nil,
                installRoot: installRoot,
                installPackageNames: [packageName],
                installedVersions: [],
                managementBackend: .homebrewCask
            )
        }

        var searchResult: PackageSearchResult {
            PackageSearchResult(
                name: packageName,
                source: .cask(caskName: brewToken),
                version: version,
                description: description,
                homepage: homepage,
                dependencies: [],
                securityState: nil,
                pulseKind: pulseEvent?.pulseKind,
                managementBackend: .homebrewCask
            )
        }

        var detail: PackageDetail {
            PackageDetail(
                packageName: packageName,
                qualifiedName: packageName,
                installRoot: installRoot,
                installed: installedVersion != nil,
                source: .cask(caskName: brewToken),
                sourceError: nil,
                aliases: aliases,
                aliasesError: nil,
                installedVersion: installedVersion,
                latestVersion: version,
                latestVersionError: nil,
                executablePaths: [],
                executablePathsError: nil,
                popularity: nil,
                lastUpdatedAt: pulseEvent?.lastUpdatedAt,
                homebrewInfo: HomebrewPackageInfo(
                    formula: brewToken,
                    description: description,
                    homepage: homepage,
                    license: nil,
                    dependencies: []
                ),
                homebrewInfoError: nil,
                npmHomepage: nil,
                npmPackageInfoError: nil,
                securityState: nil,
                installPackageNames: [packageName],
                homebrewMigration: nil,
                managementBackend: .homebrewCask
            )
        }

        private var aliases: [String] {
            names.filter { !$0.isEmpty && $0 != token && $0 != displayTitle }
        }

        private var hasAppArtifact: Bool {
            artifacts.contains { $0.kind == "app" }
        }

    }

    struct Artifact: Decodable, Equatable {
        let kind: String
        let values: [String]

        init(kind: String, values: [String] = []) {
            self.kind = kind
            self.values = values
        }

        init(from decoder: Decoder) throws {
            let container = try decoder.container(keyedBy: DynamicCodingKey.self)
            guard let key = container.allKeys.first else {
                kind = ""
                values = []
                return
            }
            kind = key.stringValue
            values = (try? container.decode([String].self, forKey: key))
                ?? (try? container.decode(String.self, forKey: key)).map { [$0] }
                ?? []
        }

        var appName: String? {
            guard kind == "app" else { return nil }
            return values.first?.split(separator: "/").last.map(String.init)
        }
    }

    private struct DynamicCodingKey: CodingKey {
        let stringValue: String
        let intValue: Int?

        init?(stringValue: String) {
            self.stringValue = stringValue
            intValue = nil
        }

        init?(intValue: Int) {
            stringValue = String(intValue)
            self.intValue = intValue
        }
    }

    private let fileManager: FileManager
    private let brewPathOverride: String?
    private let allowPathLookup: Bool

    init(
        fileManager: FileManager = .default,
        brewPath: String? = nil,
        allowPathLookup: Bool = true
    ) {
        self.fileManager = fileManager
        brewPathOverride = brewPath
        self.allowPathLookup = allowPathLookup
    }

    func isHomebrewAvailable() -> Bool {
        resolveBrewPath() != nil
    }

    func fetchInstalledPackages() async throws -> [PackagePresentation] {
        try await Task.detached(priority: .userInitiated) {
            try self.fetchInstalledPackagesSync()
        }.value
    }

    func fetchPulsePackages(offset: Int, limit: Int) async throws -> PackageSearchPage {
        try await Task.detached(priority: .utility) {
            try self.fetchPulsePackagesSync(offset: offset, limit: limit)
        }.value
    }

    func searchPackages(query: String, offset: Int, limit: Int) async throws -> PackageSearchPage {
        try await Task.detached(priority: .userInitiated) {
            try self.searchPackagesSync(query: query, offset: offset, limit: limit)
        }.value
    }

    func installCask(
        token: String,
        progress: @escaping (NukeHelperProgressEvent) -> Void
    ) async -> Result<HomebrewCaskOperationResult, Error> {
        await runCaskOperation(
            token: token,
            arguments: ["install", "--cask", token],
            action: "installing",
            progress: progress
        )
    }

    func uninstallCask(
        token: String,
        progress: @escaping (NukeHelperProgressEvent) -> Void
    ) async -> Result<HomebrewCaskOperationResult, Error> {
        await runCaskOperation(
            token: token,
            arguments: ["uninstall", "--cask", token],
            action: "uninstalling",
            progress: progress
        )
    }

    private func fetchInstalledPackagesSync() throws -> [PackagePresentation] {
        let tokenData = try runBrew(arguments: ["list", "--cask"])
        let casks = try fetchCasks(tokens: Self.parseListTokens(from: tokenData))
            .filter(\.isGuiAppCask)
            .sorted { $0.token.packageSearchOrderName < $1.token.packageSearchOrderName }
        return casks.map { cask in
            PackagePresentation(
                item: .installed(cask.record),
                detail: cask.detail,
                freshness: freshness(for: cask.packageName)
            )
        }
    }

    private func fetchPulsePackagesSync(offset: Int, limit: Int) throws -> PackageSearchPage {
        let eventLimit = Swift.max(Swift.max(offset + limit * 4, limit * 6), 120)
        let tokenLimit = Swift.max(offset + limit * 4, limit)
        let events = try pulseEvents(limit: eventLimit)
            .nonEmpty
            ?? analyticsPulseEvents(limit: eventLimit)
        let tokenWindow = Array(events.map(\.token).prefix(tokenLimit))
        let eventByToken = Dictionary(uniqueKeysWithValues: events.map { ($0.token, $0) })
        let casks = try fetchCasks(tokens: tokenWindow)
            .compactMap { cask -> Cask? in
                guard let eventful = withPulseEvent(cask, event: eventByToken[cask.token]),
                      eventful.isGuiAppCask else {
                    return nil
                }
                return eventful
            }
        let pageCasks = Array(casks.dropFirst(offset).prefix(limit))
        return PackageSearchPage(
            packages: pageCasks.map(\.searchResult),
            totalCount: casks.count,
            nextOffset: offset + limit < casks.count ? offset + limit : nil
        )
    }

    private func searchPackagesSync(query: String, offset: Int, limit: Int) throws -> PackageSearchPage {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            return PackageSearchPage(packages: [], totalCount: 0, nextOffset: nil)
        }
        let output = try runBrew(arguments: ["search", "--cask", "--desc", trimmed])
        let tokens = Self.parseSearchTokens(from: output)
        let casks = try fetchCasks(tokens: tokens)
            .filter(\.isGuiAppCask)
        let pageCasks = Array(casks.dropFirst(offset).prefix(limit))
        return PackageSearchPage(
            packages: pageCasks.map(\.searchResult),
            totalCount: casks.count,
            nextOffset: offset + limit < casks.count ? offset + limit : nil
        )
    }

    private func fetchCasks(tokens: [String]) throws -> [Cask] {
        let uniqueTokens = Array(OrderedSet(tokens.filter { !$0.isEmpty }))
        guard !uniqueTokens.isEmpty else { return [] }

        var casks: [Cask] = []
        for chunk in uniqueTokens.chunked(into: 40) {
            do {
                let data = try runBrew(arguments: ["info", "--json=v2", "--cask"] + chunk)
                casks.append(contentsOf: try Self.decodeCasks(from: data))
            } catch {
                for token in chunk {
                    guard let data = try? runBrew(arguments: ["info", "--json=v2", "--cask", token]),
                          let cask = try? Self.decodeCasks(from: data).first else {
                        continue
                    }
                    casks.append(cask)
                }
            }
        }
        return casks
    }

    private func pulseEvents(limit: Int) throws -> [PulseEvent] {
        guard let tapPath = try homebrewCaskTapPath(),
              fileManager.fileExists(atPath: tapPath.appendingPathComponent(".git").path) else {
            return []
        }

        let data = try runExecutable(
            URL(fileURLWithPath: "/usr/bin/git"),
            arguments: [
                "-C",
                tapPath.path,
                "log",
                "--max-count=1000",
                "--format=__DATE__%cI",
                "--name-status",
                "--",
                "Casks"
            ],
            environment: ProcessInfo.processInfo.environment
        )
        return Self.parsePulseEvents(fromGitLog: data, limit: limit)
    }

    private func analyticsPulseEvents(limit: Int) -> [PulseEvent] {
        guard limit > 0,
              let data = try? runBrew(arguments: [
                "info",
                "--analytics",
                "--category=cask-install",
                "--days=30"
              ]) else {
            return []
        }
        return Self.parseAnalyticsTokens(from: data)
            .prefix(limit)
            .map {
                PulseEvent(
                    token: $0,
                    lastUpdatedAt: "",
                    pulseKind: "updated"
                )
            }
    }

    private func homebrewCaskTapPath() throws -> URL? {
        do {
            let data = try runBrew(arguments: ["--repository", "homebrew/cask"])
            guard let path = String(data: data, encoding: .utf8)?
                .trimmingCharacters(in: .whitespacesAndNewlines),
                  !path.isEmpty else {
                return nil
            }
            return URL(fileURLWithPath: path, isDirectory: true)
        } catch {
            return nil
        }
    }

    private func runCaskOperation(
        token: String,
        arguments: [String],
        action: String,
        progress: @escaping (NukeHelperProgressEvent) -> Void
    ) async -> Result<HomebrewCaskOperationResult, Error> {
        await Task.detached(priority: .userInitiated) {
            do {
                progress(.resolving)
                progress(.log(package: "cask:\(token)", message: "\(action) with Homebrew"))
                _ = try self.runBrew(arguments: arguments)
                progress(.completed(package: "cask:\(token)"))
                return .success(HomebrewCaskOperationResult(
                    message: "Homebrew cask \(action.replacingOccurrences(of: "ing", with: "ed"))",
                    packageName: "cask:\(token)"
                ))
            } catch {
                progress(.error(message: error.localizedDescription))
                return .failure(error)
            }
        }.value
    }

    private func runBrew(arguments: [String]) throws -> Data {
        guard let brewPath = resolveBrewPath() else {
            throw HomebrewCaskCatalogError.homebrewUnavailable
        }
        return try runExecutable(
            URL(fileURLWithPath: brewPath),
            arguments: arguments,
            environment: homebrewEnvironment()
        )
    }

    private func runExecutable(
        _ executableURL: URL,
        arguments: [String],
        environment: [String: String]
    ) throws -> Data {
        let process = Process()
        process.executableURL = executableURL
        process.arguments = arguments
        process.environment = environment

        let stdout = Pipe()
        let stderr = Pipe()
        process.standardOutput = stdout
        process.standardError = stderr

        do {
            try process.run()
        } catch {
            throw HomebrewCaskCatalogError.commandFailed(
                "failed to run \(executableURL.path): \(error.localizedDescription)"
            )
        }

        var output = Data()
        var errorData = Data()
        let outputGroup = DispatchGroup()
        outputGroup.enter()
        DispatchQueue.global(qos: .utility).async {
            output = stdout.fileHandleForReading.readDataToEndOfFile()
            outputGroup.leave()
        }
        outputGroup.enter()
        DispatchQueue.global(qos: .utility).async {
            errorData = stderr.fileHandleForReading.readDataToEndOfFile()
            outputGroup.leave()
        }

        process.waitUntilExit()
        outputGroup.wait()
        guard process.terminationStatus == 0 else {
            let errorText = String(data: errorData, encoding: .utf8)?
                .trimmingCharacters(in: .whitespacesAndNewlines)
            throw HomebrewCaskCatalogError.commandFailed(
                "\(executableURL.lastPathComponent) \(arguments.joined(separator: " ")) failed: " +
                    (errorText?.isEmpty == false ? errorText! : "exit \(process.terminationStatus)")
            )
        }
        return output
    }

    private func resolveBrewPath() -> String? {
        if let brewPathOverride {
            return fileManager.isExecutableFile(atPath: brewPathOverride) ? brewPathOverride : nil
        }

        guard allowPathLookup else {
            return nil
        }

        for path in ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"] {
            if fileManager.isExecutableFile(atPath: path) {
                return path
            }
        }

        let pathCandidates = (ProcessInfo.processInfo.environment["PATH"] ?? "")
            .split(separator: ":")
            .map(String.init)
        for directory in pathCandidates {
            let candidate = URL(fileURLWithPath: directory)
                .appendingPathComponent("brew")
                .path
            if fileManager.isExecutableFile(atPath: candidate) {
                return candidate
            }
        }
        return nil
    }

    private func homebrewEnvironment() -> [String: String] {
        var environment = ProcessInfo.processInfo.environment
        environment["PATH"] = "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"
        environment["HOMEBREW_NO_AUTO_UPDATE"] = "1"
        environment["HOMEBREW_NO_INSTALL_CLEANUP"] = "1"
        environment.removeValue(forKey: "VAULT_SOCKET_PATH")
        environment.removeValue(forKey: "VAULT_TOOLCHAIN_ROOT")
        return environment
    }

    private func freshness(for packageName: String) -> CGFloat {
        let hash = CGFloat(abs(packageName.hashValue % 1000)) / 1000
        return 0.28 + hash * 0.72
    }

    private func withPulseEvent(_ cask: Cask, event: PulseEvent?) -> Cask? {
        guard let event else { return cask }
        var copy = cask
        copy.pulseEvent = event
        return copy
    }

    static func decodeCasks(from data: Data) throws -> [Cask] {
        let decoder = JSONDecoder()
        do {
            return try decoder.decode(InfoReport.self, from: data).casks
        } catch {
            throw HomebrewCaskCatalogError.invalidResponse(
                "failed to parse Homebrew cask report: \(error.localizedDescription)"
            )
        }
    }

    static func parseSearchTokens(from data: Data) -> [String] {
        guard let text = String(data: data, encoding: .utf8) else {
            return []
        }
        return Array(OrderedSet(text
            .split(whereSeparator: \.isNewline)
            .map { line -> String? in
                let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
                guard !trimmed.isEmpty,
                      !trimmed.hasPrefix("==>") else {
                    return nil
                }
                return trimmed.split(separator: ":", maxSplits: 1).first.map(String.init)
            }
            .compactMap { $0 }))
    }

    static func parseListTokens(from data: Data) -> [String] {
        guard let text = String(data: data, encoding: .utf8) else {
            return []
        }
        return Array(OrderedSet(text
            .split(whereSeparator: \.isNewline)
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }))
    }

    static func parseAnalyticsTokens(from data: Data) -> [String] {
        guard let text = String(data: data, encoding: .utf8) else {
            return []
        }
        let tokens = text
            .split(whereSeparator: \.isNewline)
            .compactMap { line -> String? in
                let columns = line.split(separator: "|").map {
                    $0.trimmingCharacters(in: .whitespacesAndNewlines)
                }
                guard columns.count >= 2,
                      Int(columns[0]) != nil else {
                    return nil
                }
                return columns[1].nonEmpty
            }
        return Array(OrderedSet(tokens))
    }

    static func parsePulseEvents(fromGitLog data: Data, limit: Int) -> [PulseEvent] {
        guard limit > 0,
              let text = String(data: data, encoding: .utf8) else {
            return []
        }

        var currentDate: String?
        var seen = Set<String>()
        var events: [PulseEvent] = []

        for rawLine in text.split(whereSeparator: \.isNewline) {
            let line = String(rawLine)
            if line.hasPrefix("__DATE__") {
                currentDate = String(line.dropFirst("__DATE__".count))
                continue
            }
            guard let currentDate else { continue }
            let parts = line.split(separator: "\t").map(String.init)
            guard parts.count >= 2,
                  let path = parts.last,
                  let token = caskToken(fromRubySourcePath: path),
                  seen.insert(token).inserted else {
                continue
            }
            let status = parts[0]
            events.append(PulseEvent(
                token: token,
                lastUpdatedAt: currentDate,
                pulseKind: status.hasPrefix("A") ? "new" : "updated"
            ))
            if events.count >= limit {
                break
            }
        }

        return events
    }

    static func caskToken(fromRubySourcePath path: String) -> String? {
        guard path.hasPrefix("Casks/"),
              path.hasSuffix(".rb") else {
            return nil
        }
        return URL(fileURLWithPath: path)
            .deletingPathExtension()
            .lastPathComponent
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .nonEmpty
    }
}

private struct OrderedSet<Element: Hashable>: Sequence {
    private var values: [Element] = []
    private var seen = Set<Element>()

    init<S: Sequence>(_ sequence: S) where S.Element == Element {
        for value in sequence where seen.insert(value).inserted {
            values.append(value)
        }
    }

    func makeIterator() -> IndexingIterator<[Element]> {
        values.makeIterator()
    }
}

private extension Array {
    func chunked(into size: Int) -> [[Element]] {
        guard size > 0 else { return [self] }
        return stride(from: 0, to: count, by: size).map {
            Array(self[$0 ..< Swift.min($0 + size, count)])
        }
    }
}

private extension String {
    var nonEmpty: String? {
        isEmpty ? nil : self
    }
}

private extension Array {
    var nonEmpty: [Element]? {
        isEmpty ? nil : self
    }
}

private extension KeyedDecodingContainer {
    func decodeLossyStringIfPresent(forKey key: Key) throws -> String? {
        if try decodeNil(forKey: key) {
            return nil
        }
        if let value = try? decode(String.self, forKey: key) {
            return value
        }
        if let values = try? decode([String].self, forKey: key) {
            return values.filter { !$0.isEmpty }.joined(separator: ", ")
        }
        return nil
    }
}
