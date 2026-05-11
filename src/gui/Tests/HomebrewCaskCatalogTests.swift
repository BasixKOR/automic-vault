import XCTest
@testable import AutomicVaultApp

final class HomebrewCaskCatalogTests: XCTestCase {
    func testGuiAppCaskWithoutBinaryArtifactIsIncluded() throws {
        let cask = try Self.decodeSingleCask(
            token: "iterm2",
            artifacts: #"[{"app":["iTerm.app"]}]"#
        )

        XCTAssertTrue(cask.isGuiAppCask)
        XCTAssertEqual(cask.record.name, "cask:iterm2")
        XCTAssertEqual(cask.detail.installRoot, "/Applications/iTerm.app")
        XCTAssertEqual(cask.detail.managementBackend, .homebrewCask)
    }

    func testCaskWithBinaryArtifactIsExcluded() throws {
        let cask = try Self.decodeSingleCask(
            token: "visual-studio-code",
            artifacts: #"[{"app":["Visual Studio Code.app"]},{"binary":["/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code"]}]"#
        )

        XCTAssertFalse(cask.isGuiAppCask)
    }

    func testDisabledAndDeprecatedCasksAreExcluded() throws {
        let disabled = try Self.decodeSingleCask(
            token: "disabled-app",
            artifacts: #"[{"app":["Disabled.app"]}]"#,
            disabled: true
        )
        let deprecated = try Self.decodeSingleCask(
            token: "deprecated-app",
            artifacts: #"[{"app":["Deprecated.app"]}]"#,
            deprecated: true
        )

        XCTAssertFalse(disabled.isGuiAppCask)
        XCTAssertFalse(deprecated.isGuiAppCask)
    }

    func testPulseMetadataMapsRubySourcePathToTokenAndKind() {
        let gitLog = """
        __DATE__2026-05-10T12:00:00Z
        A\tCasks/a/acme-app.rb
        M\tCasks/v/visual-studio-code.rb
        __DATE__2026-05-09T12:00:00Z
        M\tCasks/a/acme-app.rb
        """

        let events = HomebrewCaskCatalog.parsePulseEvents(
            fromGitLog: Data(gitLog.utf8),
            limit: 10
        )

        XCTAssertEqual(events, [
            HomebrewCaskCatalog.PulseEvent(
                token: "acme-app",
                lastUpdatedAt: "2026-05-10T12:00:00Z",
                pulseKind: "new"
            ),
            HomebrewCaskCatalog.PulseEvent(
                token: "visual-studio-code",
                lastUpdatedAt: "2026-05-10T12:00:00Z",
                pulseKind: "updated"
            ),
        ])
    }

    func testMissingBrewReportsUnavailable() {
        let catalog = HomebrewCaskCatalog(
            brewPath: "/tmp/automic-vault-test-missing-brew",
            allowPathLookup: false
        )

        XCTAssertFalse(catalog.isHomebrewAvailable())
    }

    private static func decodeSingleCask(
        token: String,
        artifacts: String,
        disabled: Bool = false,
        deprecated: Bool = false
    ) throws -> HomebrewCaskCatalog.Cask {
        let json = """
        {
          "casks": [
            {
              "token": "\(token)",
              "full_token": "\(token)",
              "name": ["\(token)"],
              "desc": "Fixture app",
              "homepage": "https://example.com",
              "version": "1.0",
              "installed": "1.0",
              "artifacts": \(artifacts),
              "deprecated": \(deprecated),
              "disabled": \(disabled),
              "ruby_source_path": "Casks/\(token.prefix(1))/\(token).rb"
            }
          ]
        }
        """
        return try XCTUnwrap(HomebrewCaskCatalog.decodeCasks(from: Data(json.utf8)).first)
    }
}
