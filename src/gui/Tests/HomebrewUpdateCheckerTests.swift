import XCTest
@testable import AutomicVaultApp

final class HomebrewUpdateCheckerTests: XCTestCase {
    func testRefreshDoesNotRunUpdateAndDrainsLargeHomebrewInfoOutput() throws {
        let brewPath = try installBrewFixture()
        let checker = HomebrewUpdateChecker(
            brewPath: brewPath.path,
            commandTimeout: 5,
            fileManager: .default
        )

        let packages = try checker.refreshOutdatedPackagesSync()

        XCTAssertEqual(packages, [
            OutdatedPackageRecord(
                name: "brew:railway",
                currentVersion: "4.57.1",
                latestVersion: "4.57.3"
            ),
        ])
    }

    private func installBrewFixture() throws -> URL {
        let directory = temporaryDirectory()
            .appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(
            at: directory,
            withIntermediateDirectories: true
        )

        let installedJSON = """
        {"formulae":[{"name":"railway","full_name":"railway","tap":"homebrew/core","installed":[{"installed_on_request":true}],"padding":"\(String(repeating: "x", count: 100_000))"}],"casks":[]}
        """
        let outdatedJSON = """
        {"formulae":[{"name":"railway","full_name":"railway","installed_versions":["4.57.1"],"current_version":"4.57.3"}],"casks":[]}
        """
        let script = """
        #!/bin/sh
        case "$*" in
        "info --json=v2 --installed")
          cat <<'JSON'
        \(installedJSON)
        JSON
          ;;
        "outdated --json=v2")
          cat <<'JSON'
        \(outdatedJSON)
        JSON
          ;;
        *)
          echo "unexpected brew arguments: $*" >&2
          exit 64
          ;;
        esac
        """

        let brewPath = directory.appendingPathComponent("brew")
        try Data(script.utf8).write(to: brewPath)
        try FileManager.default.setAttributes(
            [.posixPermissions: 0o755],
            ofItemAtPath: brewPath.path
        )
        return brewPath
    }

    private func temporaryDirectory() -> URL {
        URL(fileURLWithPath: NSTemporaryDirectory(), isDirectory: true)
            .appendingPathComponent("AutomicVaultHomebrewUpdateCheckerTests", isDirectory: true)
    }
}
