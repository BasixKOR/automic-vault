import AppKit
import Foundation

enum SkillsCatalogError: Error, LocalizedError {
    case unavailable
    case commandFailed(String)
    case invalidResponse(String)

    var errorDescription: String? {
        switch self {
        case .unavailable:
            return "The SKILLS tab needs npm:skills. Install skills with Homebrew or Automic Vault, or install Node/npm so `npx skills` is available."
        case .commandFailed(let message),
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
    struct Command: Equatable {
        let executablePath: String
        let baseArguments: [String]
        let displayName: String
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

    static let defaultInstallRoot = "\(NSHomeDirectory())/.codex/skills"

    private let fileManager: FileManager
    private let environment: [String: String]
    private let commandResolver: CommandResolver?
    private let commandRunner: CommandRunner?

    init(
        fileManager: FileManager = .default,
        environment: [String: String] = ProcessInfo.processInfo.environment,
        commandResolver: CommandResolver? = nil,
        commandRunner: CommandRunner? = nil
    ) {
        self.fileManager = fileManager
        self.environment = environment
        self.commandResolver = commandResolver
        self.commandRunner = commandRunner
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
