import Foundation
import Darwin

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
    private static let defaultBrewPath = "/opt/homebrew/bin/brew"
    private static let defaultCommandTimeout: TimeInterval = 120

    private struct OutdatedReport: Decodable {
        let formulae: [OutdatedFormula]
        let casks: [OutdatedCask]
    }

    private struct InstalledReport: Decodable {
        let formulae: [InstalledFormula]
        let casks: [InstalledCask]
    }

    private struct OutdatedFormula: Decodable {
        let name: String
        let fullName: String?
        let installedVersions: [String]?
        let currentVersion: String?
    }

    private struct OutdatedCask: Decodable {
        let name: String
        let installedVersions: [String]?
        let currentVersion: String?
    }

    private struct InstalledFormula: Decodable {
        let name: String
        let fullName: String?
        let tap: String?
        let installed: [InstalledFormulaVersion]

        var isInstalledOnRequest: Bool {
            installed.contains { $0.installedOnRequest }
        }

        var migrationDisplayName: String {
            guard tap != nil && tap != "homebrew/core" else {
                return name
            }
            guard let fullName, fullName.contains("/") else {
                return name
            }
            return fullName
        }
    }

    private struct InstalledFormulaVersion: Decodable {
        let installedOnRequest: Bool
    }

    private struct InstalledCask: Decodable {
        let token: String?
    }

    private let brewPath: String
    private let commandTimeout: TimeInterval
    private let fileManager: FileManager

    init(
        brewPath: String = HomebrewUpdateChecker.defaultBrewPath,
        commandTimeout: TimeInterval = HomebrewUpdateChecker.defaultCommandTimeout,
        fileManager: FileManager = .default
    ) {
        self.brewPath = brewPath
        self.commandTimeout = commandTimeout
        self.fileManager = fileManager
    }

    func refreshOutdatedPackages() async throws -> [OutdatedPackageRecord] {
        try await Task.detached(priority: .utility) {
            try self.refreshOutdatedPackagesSync()
        }.value
    }

    func refreshOutdatedPackagesSync() throws -> [OutdatedPackageRecord] {
        guard fileManager.isExecutableFile(atPath: brewPath) else {
            return []
        }

        let installedOutput = try runBrew(arguments: ["info", "--json=v2", "--installed"])
        let output = try runBrew(arguments: ["outdated", "--json=v2"])
        let installedPackages = try parseInstalledPackageNames(from: installedOutput)
        return try parseOutdatedPackages(from: output, installedPackages: installedPackages)
    }

    private func runBrew(arguments: [String]) throws -> Data {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: brewPath)
        process.arguments = arguments
        process.environment = homebrewEnvironment()

        let stdout = Pipe()
        let stderr = Pipe()
        process.standardOutput = stdout
        process.standardError = stderr

        let termination = DispatchSemaphore(value: 0)
        process.terminationHandler = { _ in
            termination.signal()
        }

        do {
            try process.run()
        } catch {
            throw HomebrewUpdateCheckerError.commandFailed(
                "failed to run \(brewPath): \(error.localizedDescription)"
            )
        }

        let readGroup = DispatchGroup()
        let stdoutReader = readPipe(stdout, group: readGroup)
        let stderrReader = readPipe(stderr, group: readGroup)

        if termination.wait(timeout: .now() + timeoutInterval()) == .timedOut {
            terminate(process: process, termination: termination)
            readGroup.wait()
            throw HomebrewUpdateCheckerError.commandFailed(
                "\(brewPath) \(arguments.joined(separator: " ")) timed out after " +
                    "\(Int(commandTimeout))s"
            )
        }

        readGroup.wait()
        let output = stdoutReader()
        let errorData = stderrReader()

        guard process.terminationStatus == 0 else {
            let message = String(data: errorData, encoding: .utf8)?
                .trimmingCharacters(in: .whitespacesAndNewlines)
            throw HomebrewUpdateCheckerError.commandFailed(
                "\(brewPath) \(arguments.joined(separator: " ")) failed: " +
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

    private func readPipe(_ pipe: Pipe, group: DispatchGroup) -> () -> Data {
        let lock = NSLock()
        var data = Data()
        group.enter()
        DispatchQueue.global(qos: .utility).async {
            let readData = pipe.fileHandleForReading.readDataToEndOfFile()
            lock.lock()
            data = readData
            lock.unlock()
            group.leave()
        }
        return {
            lock.lock()
            defer { lock.unlock() }
            return data
        }
    }

    private func timeoutInterval() -> DispatchTimeInterval {
        .milliseconds(max(1, Int(commandTimeout * 1000)))
    }

    private func terminate(process: Process, termination: DispatchSemaphore) {
        if process.isRunning {
            process.terminate()
        }
        if termination.wait(timeout: .now() + .seconds(2)) == .timedOut,
           process.isRunning {
            Darwin.kill(process.processIdentifier, SIGKILL)
            termination.wait()
        }
    }

    private func parseInstalledPackageNames(from data: Data) throws -> [String: String] {
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase

        let report: InstalledReport
        do {
            report = try decoder.decode(InstalledReport.self, from: data)
        } catch {
            throw HomebrewUpdateCheckerError.invalidResponse(
                "failed to parse Homebrew installed package report: \(error.localizedDescription)"
            )
        }

        var packagesByName: [String: String] = [:]
        for formula in report.formulae where formula.isInstalledOnRequest {
            let displayName = "brew:\(formula.migrationDisplayName)"
            packagesByName["brew:\(formula.name)"] = displayName
            if let fullName = formula.fullName, fullName.isEmpty == false {
                packagesByName["brew:\(fullName)"] = displayName
            }
        }
        for cask in report.casks {
            guard let token = cask.token, token.isEmpty == false else {
                continue
            }
            packagesByName["cask:\(token)"] = "cask:\(token)"
        }
        return packagesByName
    }

    private func parseOutdatedPackages(
        from data: Data,
        installedPackages: [String: String]
    ) throws -> [OutdatedPackageRecord] {
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

        let formulae = report.formulae.compactMap { formula -> OutdatedPackageRecord? in
            guard let displayName = installedDisplayName(
                for: ["brew:\(formula.name)", formula.fullName.map { "brew:\($0)" }],
                installedPackages: installedPackages
            ) else {
                return nil
            }
            return OutdatedPackageRecord(
                name: displayName,
                currentVersion: installedVersionText(formula.installedVersions),
                latestVersion: formula.currentVersion ?? "available"
            )
        }
        let casks = report.casks.compactMap { cask -> OutdatedPackageRecord? in
            let name = "cask:\(cask.name)"
            guard let displayName = installedPackages[name] else {
                return nil
            }
            return OutdatedPackageRecord(
                name: displayName,
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

    private func installedDisplayName(
        for candidates: [String?],
        installedPackages: [String: String]
    ) -> String? {
        for candidate in candidates.compactMap({ $0 }) {
            if let displayName = installedPackages[candidate] {
                return displayName
            }
        }
        return nil
    }
}
