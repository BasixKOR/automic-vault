import Foundation

enum HomebrewUpdateCheckerError: Error, LocalizedError {
    case commandFailed(String)
    case invalidResponse(String)

    var errorDescription: String? {
        switch self {
        case .commandFailed(let message):
            return message
        case .invalidResponse(let message):
            return message
        }
    }
}

final class HomebrewUpdateChecker {
    private struct OutdatedReport: Decodable {
        let formulae: [OutdatedFormula]
        let casks: [OutdatedCask]
    }

    private struct OutdatedFormula: Decodable {
        let name: String
        let installedVersions: [String]?
        let currentVersion: String?
    }

    private struct OutdatedCask: Decodable {
        let name: String
        let installedVersions: [String]?
        let currentVersion: String?
    }

    private static let brewPath = "/opt/homebrew/bin/brew"

    private let fileManager: FileManager

    init(fileManager: FileManager = .default) {
        self.fileManager = fileManager
    }

    func refreshOutdatedPackages() async throws -> [OutdatedPackageRecord] {
        try await Task.detached(priority: .utility) {
            try self.refreshOutdatedPackagesSync()
        }.value
    }

    func refreshOutdatedPackagesSync() throws -> [OutdatedPackageRecord] {
        guard fileManager.isExecutableFile(atPath: Self.brewPath) else {
            return []
        }

        _ = try runBrew(arguments: ["update"])
        let output = try runBrew(arguments: ["outdated", "--json=v2"])
        return try parseOutdatedPackages(from: output)
    }

    private func runBrew(arguments: [String]) throws -> Data {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: Self.brewPath)
        process.arguments = arguments
        process.environment = homebrewEnvironment()

        let stdout = Pipe()
        let stderr = Pipe()
        process.standardOutput = stdout
        process.standardError = stderr

        do {
            try process.run()
        } catch {
            throw HomebrewUpdateCheckerError.commandFailed(
                "failed to run \(Self.brewPath): \(error.localizedDescription)"
            )
        }

        process.waitUntilExit()
        let output = stdout.fileHandleForReading.readDataToEndOfFile()
        let errorData = stderr.fileHandleForReading.readDataToEndOfFile()

        guard process.terminationStatus == 0 else {
            let message = String(data: errorData, encoding: .utf8)?
                .trimmingCharacters(in: .whitespacesAndNewlines)
            throw HomebrewUpdateCheckerError.commandFailed(
                "\(Self.brewPath) \(arguments.joined(separator: " ")) failed: " +
                    (message?.isEmpty == false ? message! : "exit \(process.terminationStatus)")
            )
        }

        return output
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

    private func parseOutdatedPackages(from data: Data) throws -> [OutdatedPackageRecord] {
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase

        let report: OutdatedReport
        do {
            report = try decoder.decode(OutdatedReport.self, from: data)
        } catch {
            throw HomebrewUpdateCheckerError.invalidResponse(
                "failed to parse Homebrew outdated package report: \(error.localizedDescription)"
            )
        }

        let formulae = report.formulae.map { formula in
            OutdatedPackageRecord(
                name: "brew:\(formula.name)",
                currentVersion: installedVersionText(formula.installedVersions),
                latestVersion: formula.currentVersion ?? "available"
            )
        }
        let casks = report.casks.map { cask in
            OutdatedPackageRecord(
                name: "cask:\(cask.name)",
                currentVersion: installedVersionText(cask.installedVersions),
                latestVersion: cask.currentVersion ?? "available"
            )
        }

        return (formulae + casks).sorted {
            $0.name.packageSearchOrderName < $1.name.packageSearchOrderName
        }
    }

    private func installedVersionText(_ versions: [String]?) -> String {
        let nonEmptyVersions = (versions ?? []).filter { !$0.isEmpty }
        return nonEmptyVersions.isEmpty ? "installed" : nonEmptyVersions.joined(separator: ", ")
    }
}
