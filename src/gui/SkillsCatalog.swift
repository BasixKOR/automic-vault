import AppKit
import Foundation

enum SkillsCatalogError: Error, LocalizedError {
    case unavailable
    case commandFailed(String)
    case apiUnavailable(String)
    case invalidResponse(String)

    var errorDescription: String? {
        switch self {
        case .unavailable:
            return "The SKILLS tab needs npm:skills. Install skills with Homebrew or Automic Vault, or install Node/npm so `npx skills` is available."
        case .commandFailed(let message),
             .apiUnavailable(let message),
             .invalidResponse(let message):
            return message
        }
    }
}

struct SkillsOperationResult {
    let message: String
    let packageName: String
}

final class SkillsCatalog {
    private enum API {
        static let baseURL = URL(string: "https://skills.sh")!
        static let maximumSearchLimit = 200
    }

    struct Command: Equatable {
        let executablePath: String
        let baseArguments: [String]
        let displayName: String
    }

    struct RemoteSkill: Decodable, Equatable {
        let id: String
        let slug: String
        let name: String
        let source: String
        let installs: Int
        let sourceType: String?
        let installURL: String?
        let url: String?
        let isDuplicate: Bool
        let installsYesterday: Int?
        let change: Int?

        enum CodingKeys: String, CodingKey {
            case id
            case slug
            case skillID = "skillId"
            case name
            case source
            case installs
            case sourceType
            case installURL = "installUrl"
            case url
            case isDuplicate
            case installsYesterday
            case change
        }

        init(
            id: String,
            slug: String,
            name: String,
            source: String,
            installs: Int,
            sourceType: String? = nil,
            installURL: String? = nil,
            url: String? = nil,
            isDuplicate: Bool = false,
            installsYesterday: Int? = nil,
            change: Int? = nil
        ) {
            self.id = id
            self.slug = slug
            self.name = name
            self.source = source
            self.installs = installs
            self.sourceType = sourceType
            self.installURL = installURL
            self.url = url
            self.isDuplicate = isDuplicate
            self.installsYesterday = installsYesterday
            self.change = change
        }

        init(from decoder: Decoder) throws {
            let container = try decoder.container(keyedBy: CodingKeys.self)
            id = try container.decode(String.self, forKey: .id)
            slug = try container.decodeIfPresent(String.self, forKey: .slug)
                ?? container.decodeIfPresent(String.self, forKey: .skillID)
                ?? container.decodeIfPresent(String.self, forKey: .name)
                ?? id.split(separator: "/").last.map(String.init)
                ?? id
            name = try container.decodeIfPresent(String.self, forKey: .name) ?? slug
            source = try container.decode(String.self, forKey: .source)
            installs = try container.decodeIfPresent(Int.self, forKey: .installs) ?? 0
            sourceType = try container.decodeIfPresent(String.self, forKey: .sourceType)
            installURL = try container.decodeIfPresent(String.self, forKey: .installURL)
            url = try container.decodeIfPresent(String.self, forKey: .url)
            isDuplicate = try container.decodeIfPresent(Bool.self, forKey: .isDuplicate) ?? false
            installsYesterday = try container.decodeIfPresent(Int.self, forKey: .installsYesterday)
            change = try container.decodeIfPresent(Int.self, forKey: .change)
        }

        var installSpec: String {
            "\(source)@\(name)"
        }

        var qualifiedPackageName: String {
            SkillsCatalog.qualifiedPackageName(for: installSpec)
        }

        var summary: String {
            let installText = Self.installsText(installs)
            if installText.isEmpty {
                return "Published by \(source)."
            }
            return "\(installText) from \(source)."
        }

        var searchResult: PackageSearchResult {
            PackageSearchResult(
                name: qualifiedPackageName,
                source: .npm(packageName: "skills"),
                version: nil,
                description: summary,
                homepage: url,
                dependencies: [],
                securityState: nil,
                pulseKind: nil,
                managementBackend: .npmSkills
            )
        }

        func pulseSearchResult() -> PackageSearchResult {
            PackageSearchResult(
                name: qualifiedPackageName,
                source: .npm(packageName: "skills"),
                version: nil,
                description: summary,
                homepage: url,
                dependencies: [],
                securityState: nil,
                pulseKind: "updated",
                managementBackend: .npmSkills
            )
        }

        var detail: PackageDetail {
            detail(pulseKind: nil)
        }

        func detail(pulseKind: String?) -> PackageDetail {
            PackageDetail(
                packageName: qualifiedPackageName,
                qualifiedName: qualifiedPackageName,
                installRoot: SkillsCatalog.defaultInstallRoot,
                installed: false,
                source: .npm(packageName: "skills"),
                sourceError: nil,
                aliases: sourceType.map { [$0] } ?? [],
                aliasesError: nil,
                installedVersion: nil,
                latestVersion: nil,
                latestVersionError: nil,
                executablePaths: [],
                executablePathsError: nil,
                popularity: nil,
                lastUpdatedAt: nil,
                homebrewInfo: HomebrewPackageInfo(
                    formula: name,
                    description: summary,
                    homepage: url,
                    license: nil,
                    dependencies: []
                ),
                homebrewInfoError: nil,
                npmHomepage: installURL,
                npmPackageInfoError: nil,
                securityState: nil,
                installPackageNames: [installSpec],
                homebrewMigration: nil,
                managementBackend: .npmSkills
            )
        }

        private static func installsText(_ installs: Int) -> String {
            guard installs > 0 else { return "" }
            if installs >= 1_000_000 {
                let count = (Double(installs) / 1_000_000).formattedInstallCountSuffix("M")
                return "\(count) installs"
            }
            if installs >= 1_000 {
                let count = (Double(installs) / 1_000).formattedInstallCountSuffix("K")
                return "\(count) installs"
            }
            return "\(installs) install\(installs == 1 ? "" : "s")"
        }
    }

    struct SearchAPIResponse: Decodable, Equatable {
        let skills: [RemoteSkill]
        let count: Int

        enum CodingKeys: String, CodingKey {
            case data
            case skills
            case count
        }

        init(skills: [RemoteSkill], count: Int) {
            self.skills = skills
            self.count = count
        }

        init(from decoder: Decoder) throws {
            let container = try decoder.container(keyedBy: CodingKeys.self)
            skills = try container.decodeIfPresent([RemoteSkill].self, forKey: .data)
                ?? container.decodeIfPresent([RemoteSkill].self, forKey: .skills)
                ?? []
            count = try container.decodeIfPresent(Int.self, forKey: .count) ?? skills.count
        }
    }

    struct PulseAPIResponse: Decodable, Equatable {
        let data: [RemoteSkill]
        let pagination: Pagination

        struct Pagination: Decodable, Equatable {
            let page: Int
            let perPage: Int
            let total: Int
            let hasMore: Bool
        }
    }

    struct SkillRecord: Decodable, Equatable {
        let name: String
        let path: String
        let scope: String
        let agents: [String]

        enum CodingKeys: String, CodingKey {
            case name
            case path
            case scope
            case agents
        }

        init(
            name: String,
            path: String,
            scope: String,
            agents: [String] = []
        ) {
            self.name = name
            self.path = path
            self.scope = scope
            self.agents = agents
        }

        init(from decoder: Decoder) throws {
            let container = try decoder.container(keyedBy: CodingKeys.self)
            name = try container.decode(String.self, forKey: .name)
            path = try container.decodeIfPresent(String.self, forKey: .path) ?? ""
            scope = try container.decodeIfPresent(String.self, forKey: .scope) ?? ""
            agents = try container.decodeIfPresent([String].self, forKey: .agents) ?? []
        }

        var qualifiedPackageName: String {
            SkillsCatalog.qualifiedPackageName(for: name)
        }

        var summary: String {
            let agentText = agents
                .filter { $0.isEmpty == false }
                .joined(separator: ", ")
            guard agentText.isEmpty == false else {
                return "Globally installed npm:skills skill."
            }
            return "Globally installed for \(agentText)."
        }

        var record: PackageRecord {
            PackageRecord(
                name: qualifiedPackageName,
                source: .npm(packageName: "skills"),
                version: "global",
                description: summary,
                securityState: nil,
                installRoot: path.isEmpty ? SkillsCatalog.defaultInstallRoot : path,
                installPackageNames: [name],
                managementBackend: .npmSkills
            )
        }

        var detail: PackageDetail {
            PackageDetail(
                packageName: qualifiedPackageName,
                qualifiedName: qualifiedPackageName,
                installRoot: path.isEmpty ? SkillsCatalog.defaultInstallRoot : path,
                installed: true,
                source: .npm(packageName: "skills"),
                sourceError: nil,
                aliases: [],
                aliasesError: nil,
                installedVersion: "global",
                latestVersion: nil,
                latestVersionError: nil,
                executablePaths: [],
                executablePathsError: nil,
                popularity: nil,
                lastUpdatedAt: nil,
                homebrewInfo: HomebrewPackageInfo(
                    formula: name,
                    description: summary,
                    homepage: nil,
                    license: nil,
                    dependencies: []
                ),
                homebrewInfoError: nil,
                npmHomepage: nil,
                npmPackageInfoError: nil,
                securityState: nil,
                installPackageNames: [name],
                homebrewMigration: nil,
                managementBackend: .npmSkills
            )
        }
    }

    typealias CommandRunner = (Command, [String]) throws -> Data
    typealias CommandResolver = () -> Command?
    typealias APIDataFetcher = (URL) async throws -> Data

    static let defaultInstallRoot = "\(NSHomeDirectory())/.codex/skills"

    private let fileManager: FileManager
    private let environment: [String: String]
    private let commandResolver: CommandResolver?
    private let commandRunner: CommandRunner?
    private let apiDataFetcher: APIDataFetcher?

    init(
        fileManager: FileManager = .default,
        environment: [String: String] = ProcessInfo.processInfo.environment,
        commandResolver: CommandResolver? = nil,
        commandRunner: CommandRunner? = nil,
        apiDataFetcher: APIDataFetcher? = nil
    ) {
        self.fileManager = fileManager
        self.environment = environment
        self.commandResolver = commandResolver
        self.commandRunner = commandRunner
        self.apiDataFetcher = apiDataFetcher
    }

    func isAvailable() -> Bool {
        resolveCommand() != nil
    }

    func unavailableMessage() -> String {
        SkillsCatalogError.unavailable.localizedDescription
    }

    func fetchInstalledPackages() async throws -> [PackagePresentation] {
        try await Task.detached(priority: .userInitiated) {
            let data = try self.run(arguments: ["list", "-g", "--json"])
            let records = try Self.decodeInstalledSkills(from: data).sorted {
                $0.name.packageSearchOrderName < $1.name.packageSearchOrderName
            }
            return records.map { record in
                PackagePresentation(
                    item: .installed(record.record),
                    detail: record.detail,
                    freshness: self.freshness(for: record.qualifiedPackageName)
                )
            }
        }.value
    }

    func searchInstalledPackages(
        query: String,
        installedPackages: [PackagePresentation]
    ) -> [PackagePresentation] {
        let normalizedQuery = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard normalizedQuery.isEmpty == false else {
            return installedPackages
        }
        var results = installedPackages.filter { package in
            package.displayName.localizedCaseInsensitiveContains(normalizedQuery)
                || package.listSecondaryText.localizedCaseInsensitiveContains(normalizedQuery)
        }
        if results.isEmpty {
            let installCandidate = Self.installCandidate(
                skillName: normalizedQuery,
                freshness: freshness(for: Self.qualifiedPackageName(for: normalizedQuery))
            )
            results.append(installCandidate)
        }
        return results
    }

    func searchPackages(
        query: String,
        offset: Int,
        limit: Int,
        excludingInstalledSkillNames installedSkillNames: Set<String>
    ) async -> PackageSearchPage {
        let normalizedQuery = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard normalizedQuery.count >= 2 else {
            return PackageSearchPage(packages: [], totalCount: 0, nextOffset: nil)
        }
        let requestLimit = min(max(offset + limit, limit), API.maximumSearchLimit)
        do {
            let response = try await fetchSearchResponse(query: normalizedQuery, limit: requestLimit)
            let remoteSkills = Self.filteredRemoteSkills(
                response.skills,
                installedSkillNames: installedSkillNames
            )
            let pageSkills = Array(remoteSkills.dropFirst(offset).prefix(limit))
            let nextOffset = offset + limit
            let mayHaveMore = requestLimit < API.maximumSearchLimit
                && response.skills.count == requestLimit
            return PackageSearchPage(
                packages: pageSkills.map(\.searchResult),
                totalCount: max(remoteSkills.count, response.count),
                nextOffset: nextOffset < remoteSkills.count || mayHaveMore ? nextOffset : nil
            )
        } catch {
            return PackageSearchPage(packages: [], totalCount: 0, nextOffset: nil)
        }
    }

    func fetchPulsePackages(
        offset: Int,
        limit: Int,
        excludingInstalledSkillNames installedSkillNames: Set<String>
    ) async -> PackageSearchPage {
        do {
            let response = try await fetchPulseResponse(offset: offset, limit: limit)
            let remoteSkills = Self.filteredRemoteSkills(
                response.data,
                installedSkillNames: installedSkillNames
            )
            let nextOffset = offset + limit
            return PackageSearchPage(
                packages: remoteSkills.map { $0.pulseSearchResult() },
                totalCount: response.pagination.total,
                nextOffset: response.pagination.hasMore ? nextOffset : nil
            )
        } catch {
            return PackageSearchPage(packages: [], totalCount: 0, nextOffset: nil)
        }
    }

    func installSkill(
        name: String,
        progress: @escaping (NukeHelperProgressEvent) -> Void
    ) async -> Result<SkillsOperationResult, Error> {
        await runSkillOperation(
            name: name,
            arguments: ["add", "-g", "-y", name],
            action: "installing",
            progress: progress
        )
    }

    func removeSkill(
        name: String,
        progress: @escaping (NukeHelperProgressEvent) -> Void
    ) async -> Result<SkillsOperationResult, Error> {
        await runSkillOperation(
            name: name,
            arguments: ["remove", "-g", "-y", name],
            action: "removing",
            progress: progress
        )
    }

    static func qualifiedPackageName(for skillName: String) -> String {
        "npm:skills:\(skillName)"
    }

    static func decodeSearchResponse(from data: Data) throws -> SearchAPIResponse {
        do {
            return try JSONDecoder().decode(SearchAPIResponse.self, from: data)
        } catch {
            throw SkillsCatalogError.invalidResponse(
                "failed to parse skills search response: \(error.localizedDescription)"
            )
        }
    }

    static func decodePulseResponse(from data: Data) throws -> PulseAPIResponse {
        do {
            return try JSONDecoder().decode(PulseAPIResponse.self, from: data)
        } catch {
            throw SkillsCatalogError.invalidResponse(
                "failed to parse skills pulse response: \(error.localizedDescription)"
            )
        }
    }

    static func decodeTrendingHTMLPulseResponse(
        from data: Data,
        offset: Int,
        limit: Int
    ) throws -> PulseAPIResponse {
        guard let rawText = String(data: data, encoding: .utf8) else {
            throw SkillsCatalogError.invalidResponse("failed to parse skills pulse response")
        }
        let text = rawText
            .replacingOccurrences(of: "\\\"", with: "\"")
            .replacingOccurrences(of: "\\/", with: "/")
            .replacingOccurrences(of: "&quot;", with: "\"")
        let expression = try NSRegularExpression(
            pattern: #""source":"([^"]+)","skillId":"([^"]+)","name":"([^"]+)","installs":([0-9]+)"#
        )
        let range = NSRange(text.startIndex..<text.endIndex, in: text)
        let matches = expression.matches(in: text, range: range)
        var seenIDs: Set<String> = []
        let allSkills = matches.compactMap { match -> RemoteSkill? in
            guard
                let source = text.substring(for: match.range(at: 1)),
                let skillID = text.substring(for: match.range(at: 2)),
                let name = text.substring(for: match.range(at: 3)),
                let installsText = text.substring(for: match.range(at: 4)),
                let installs = Int(installsText)
            else {
                return nil
            }
            let id = "\(source)/\(skillID)"
            guard seenIDs.insert(id).inserted else {
                return nil
            }
            return RemoteSkill(
                id: id,
                slug: skillID,
                name: name,
                source: source,
                installs: installs,
                url: "https://skills.sh/\(id)"
            )
        }
        let total = Self.firstIntegerMatch(
            pattern: #""totalSkills":([0-9]+)"#,
            in: text
        ) ?? allSkills.count
        let pageSkills = Array(allSkills.dropFirst(offset).prefix(limit))
        return PulseAPIResponse(
            data: pageSkills,
            pagination: PulseAPIResponse.Pagination(
                page: limit == 0 ? 0 : offset / limit,
                perPage: limit,
                total: total,
                hasMore: offset + pageSkills.count < allSkills.count
            )
        )
    }

    private static func firstIntegerMatch(pattern: String, in text: String) -> Int? {
        guard let expression = try? NSRegularExpression(pattern: pattern) else {
            return nil
        }
        let range = NSRange(text.startIndex..<text.endIndex, in: text)
        guard
            let match = expression.firstMatch(in: text, range: range),
            let value = text.substring(for: match.range(at: 1))
        else {
            return nil
        }
        return Int(value)
    }

    static func filteredRemoteSkills(
        _ skills: [RemoteSkill],
        installedSkillNames: Set<String>
    ) -> [RemoteSkill] {
        skills.filter { skill in
            skill.isDuplicate == false
                && installedSkillNames.contains(skill.name.lowercased()) == false
        }
    }

    static func decodeInstalledSkills(from data: Data) throws -> [SkillRecord] {
        do {
            return try JSONDecoder().decode([SkillRecord].self, from: data)
                .filter { $0.scope == "global" && $0.name.isEmpty == false }
        } catch {
            throw SkillsCatalogError.invalidResponse(
                "failed to parse npm:skills global list: \(error.localizedDescription)"
            )
        }
    }

    static func resolveCommand(
        fileManager: FileManager,
        environment: [String: String],
        includeFixedPaths: Bool = true
    ) -> Command? {
        if let skillsPath = resolveExecutable(
            named: "skills",
            fixedPaths: includeFixedPaths ? [
                "/opt/homebrew/bin/skills",
                "/usr/local/bin/skills",
            ] : [],
            fileManager: fileManager,
            environment: environment
        ) {
            return Command(
                executablePath: skillsPath,
                baseArguments: [],
                displayName: "skills"
            )
        }
        if let npxPath = resolveExecutable(
            named: "npx",
            fixedPaths: includeFixedPaths ? [
                "/opt/homebrew/bin/npx",
                "/usr/local/bin/npx",
            ] : [],
            fileManager: fileManager,
            environment: environment
        ) {
            return Command(
                executablePath: npxPath,
                baseArguments: ["--yes", "skills"],
                displayName: "npx skills"
            )
        }
        return nil
    }

    private func runSkillOperation(
        name: String,
        arguments: [String],
        action: String,
        progress: @escaping (NukeHelperProgressEvent) -> Void
    ) async -> Result<SkillsOperationResult, Error> {
        await Task.detached(priority: .userInitiated) {
            let packageName = Self.qualifiedPackageName(for: name)
            do {
                progress(.resolving)
                progress(.log(package: packageName, message: "\(action.capitalized) global skill with npm:skills"))
                let output = try self.run(arguments: arguments)
                Self.progressLogLines(from: output, packageName: packageName, progress: progress)
                progress(.completed(package: packageName))
                return .success(SkillsOperationResult(
                    message: "npm:skills \(action.replacingOccurrences(of: "ing", with: "ed")) \(name)",
                    packageName: packageName
                ))
            } catch {
                progress(.error(message: error.localizedDescription))
                return .failure(error)
            }
        }.value
    }

    private func resolveCommand() -> Command? {
        if let commandResolver {
            return commandResolver()
        }
        return Self.resolveCommand(
            fileManager: fileManager,
            environment: environment
        )
    }

    private func run(arguments: [String]) throws -> Data {
        guard let command = resolveCommand() else {
            throw SkillsCatalogError.unavailable
        }
        if let commandRunner {
            return try commandRunner(command, arguments)
        }
        return try runExecutable(command, arguments: arguments)
    }

    private func fetchAPIData(from url: URL) async throws -> Data {
        if let apiDataFetcher {
            return try await apiDataFetcher(url)
        }
        let (data, response) = try await URLSession.shared.data(from: url)
        guard let httpResponse = response as? HTTPURLResponse else {
            throw SkillsCatalogError.apiUnavailable("skills.sh returned an invalid response.")
        }
        guard (200..<300).contains(httpResponse.statusCode) else {
            throw SkillsCatalogError.apiUnavailable(
                "skills.sh request failed with HTTP \(httpResponse.statusCode)."
            )
        }
        return data
    }

    private func fetchSearchResponse(query: String, limit: Int) async throws -> SearchAPIResponse {
        do {
            return try Self.decodeSearchResponse(
                from: try await fetchAPIData(from: Self.searchURL(path: "/api/v1/skills/search", query: query, limit: limit))
            )
        } catch {
            return try Self.decodeSearchResponse(
                from: try await fetchAPIData(from: Self.searchURL(path: "/api/search", query: query, limit: limit))
            )
        }
    }

    private func fetchPulseResponse(offset: Int, limit: Int) async throws -> PulseAPIResponse {
        let page = limit == 0 ? 0 : offset / limit
        do {
            var components = URLComponents(
                url: API.baseURL.appendingPathComponent("/api/v1/skills"),
                resolvingAgainstBaseURL: false
            )
            components?.queryItems = [
                URLQueryItem(name: "view", value: "trending"),
                URLQueryItem(name: "page", value: String(page)),
                URLQueryItem(name: "per_page", value: String(limit)),
            ]
            guard let url = components?.url else {
                throw SkillsCatalogError.apiUnavailable("failed to build skills pulse URL.")
            }
            return try Self.decodePulseResponse(from: try await fetchAPIData(from: url))
        } catch {
            let data = try await fetchAPIData(from: API.baseURL.appendingPathComponent("/trending"))
            return try Self.decodeTrendingHTMLPulseResponse(from: data, offset: offset, limit: limit)
        }
    }

    private static func searchURL(path: String, query: String, limit: Int) throws -> URL {
        var components = URLComponents(
            url: API.baseURL.appendingPathComponent(path),
            resolvingAgainstBaseURL: false
        )
        components?.queryItems = [
            URLQueryItem(name: "q", value: query),
            URLQueryItem(name: "limit", value: String(limit)),
        ]
        guard let url = components?.url else {
            throw SkillsCatalogError.apiUnavailable("failed to build skills search URL.")
        }
        return url
    }

    private func runExecutable(_ command: Command, arguments: [String]) throws -> Data {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: command.executablePath)
        process.arguments = command.baseArguments + arguments
        process.environment = commandEnvironment()

        let stdout = Pipe()
        let stderr = Pipe()
        process.standardOutput = stdout
        process.standardError = stderr

        do {
            try process.run()
        } catch {
            throw SkillsCatalogError.commandFailed(
                "failed to run \(command.displayName): \(error.localizedDescription)"
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
            throw SkillsCatalogError.commandFailed(
                "\(command.displayName) \(arguments.joined(separator: " ")) failed: " +
                    (errorText?.isEmpty == false ? errorText! : "exit \(process.terminationStatus)")
            )
        }
        return output
    }

    private func commandEnvironment() -> [String: String] {
        var commandEnvironment = environment
        commandEnvironment["PATH"] = [
            "/opt/homebrew/bin",
            "/usr/local/bin",
            "/usr/bin",
            "/bin",
            "/usr/sbin",
            "/sbin",
        ].joined(separator: ":")
        commandEnvironment.removeValue(forKey: "VAULT_SOCKET_PATH")
        commandEnvironment.removeValue(forKey: "VAULT_TOOLCHAIN_ROOT")
        return commandEnvironment
    }

    private func freshness(for packageName: String) -> CGFloat {
        let hash = CGFloat(abs(packageName.hashValue % 1000)) / 1000
        return 0.28 + hash * 0.72
    }

    private static func installCandidate(skillName: String, freshness: CGFloat) -> PackagePresentation {
        let packageName = qualifiedPackageName(for: skillName)
        let result = PackageSearchResult(
            name: packageName,
            source: .npm(packageName: "skills"),
            version: nil,
            description: "Install a global npm:skills skill.",
            homepage: nil,
            dependencies: [],
            securityState: nil,
            pulseKind: nil,
            managementBackend: .npmSkills
        )
        return PackagePresentation(
            item: .available(result),
            detail: result.fallbackDetail,
            freshness: freshness
        )
    }

    private static func progressLogLines(
        from data: Data,
        packageName: String,
        progress: (NukeHelperProgressEvent) -> Void
    ) {
        guard let text = String(data: data, encoding: .utf8) else { return }
        text.split(whereSeparator: \.isNewline)
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { $0.isEmpty == false }
            .forEach { progress(.log(package: packageName, message: $0)) }
    }

    private static func resolveExecutable(
        named executableName: String,
        fixedPaths: [String],
        fileManager: FileManager,
        environment: [String: String]
    ) -> String? {
        for path in fixedPaths where fileManager.isExecutableFile(atPath: path) {
            return path
        }
        let pathCandidates = (environment["PATH"] ?? "")
            .split(separator: ":")
            .map(String.init)
        for directory in pathCandidates {
            let candidate = URL(fileURLWithPath: directory)
                .appendingPathComponent(executableName)
                .path
            if fileManager.isExecutableFile(atPath: candidate) {
                return candidate
            }
        }
        return nil
    }
}

private extension Double {
    func formattedInstallCountSuffix(_ suffix: String) -> String {
        let value = String(format: "%.1f", self)
        return value.hasSuffix(".0")
            ? "\(value.dropLast(2))\(suffix)"
            : "\(value)\(suffix)"
    }
}

private extension String {
    func substring(for range: NSRange) -> String? {
        guard let stringRange = Range(range, in: self) else {
            return nil
        }
        return String(self[stringRange])
    }
}
