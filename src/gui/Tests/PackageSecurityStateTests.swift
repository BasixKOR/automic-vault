import XCTest
@testable import AutomicVaultApp

final class PackageSecurityStateTests: XCTestCase {
    func testDetectorOnlySecurityStateDecodesWithoutRemediationAction() throws {
        let detail = try decodePackageDetail(
            packageName: "brew:curl",
            formula: "curl",
            securityState: """
            {
              "isotopeName": "git",
              "installIsInsecure": true,
              "remediationAvailable": false,
              "reasons": ["Git credential store contains plaintext credentials"],
              "error": null
            }
            """
        )

        let state = try XCTUnwrap(detail.securityState)
        XCTAssertFalse(state.remediationAvailable)
        XCTAssertEqual(state.reasons, ["Git credential store contains plaintext credentials"])

        let notice = try XCTUnwrap(
            SecurityCatalog(bundle: Bundle(for: Self.self)).notice(for: detail)
        )
        XCTAssertEqual(notice.source, .isotope)
        XCTAssertNil(notice.applyPackageName)
        XCTAssertEqual(notice.headline, "LOCAL SECRET EXPOSURE")
        XCTAssertTrue(notice.body.contains("does not yet provide migration"))
        XCTAssertEqual(notice.reasons, state.reasons)
    }

    func testMissingRemediationAvailabilityDefaultsToAvailable() throws {
        let detail = try decodePackageDetail(
            securityState: """
            {
              "isotopeName": "aws-cli",
              "installIsInsecure": true,
              "reasons": ["AWS shared credentials file contains plaintext credentials"],
              "error": null
            }
            """
        )

        let state = try XCTUnwrap(detail.securityState)
        XCTAssertTrue(state.remediationAvailable)

        let notice = try XCTUnwrap(
            SecurityCatalog(bundle: Bundle(for: Self.self)).notice(for: detail)
        )
        XCTAssertEqual(notice.applyPackageName, "isotope:aws-cli")
        XCTAssertEqual(notice.headline, "PLAIN TEXT SECRET")
    }

    func testAvailableSearchResultExposesDetectorOnlyHazardSource() {
        let result = PackageSearchResult(
            name: "git",
            source: .formula(rootFormula: "git"),
            version: "2.54.0",
            description: "Distributed revision control system",
            homepage: nil,
            dependencies: [],
            securityState: PackageSecurityState(
                isotopeName: "git",
                installIsInsecure: true,
                remediationAvailable: false,
                reasons: ["Git credential store contains plaintext credentials"],
                error: nil
            ),
            pulseKind: nil
        )
        let presentation = PackagePresentation(
            item: .available(result),
            detail: result.fallbackDetail,
            freshness: 0
        )

        XCTAssertEqual(presentation.plainTextSecretAlertSource, .isotope)
        XCTAssertTrue(presentation.hasPlainTextSecretAlert)
        XCTAssertTrue(presentation.hasActivePlainTextSecretAlert)
        XCTAssertFalse(presentation.plainTextSecretAlertIsGhosted)
    }

    func testLocalDetectorHazardInInstalledPanelIsActiveEvenWhenPackageIsNotInstalled() throws {
        let detail = try decodePackageDetail(
            securityState: """
            {
              "isotopeName": "curl",
              "installIsInsecure": true,
              "remediationAvailable": false,
              "reasons": ["curl netrc file contains plaintext credentials"],
              "error": null
            }
            """
        )
        let record = PackageRecord(
            name: "brew:curl",
            source: .formula(rootFormula: "curl"),
            version: "8.20.0",
            description: "Get a file from an HTTP, HTTPS or FTP server",
            securityState: detail.securityState,
            installRoot: detail.installRoot,
            installPackageNames: ["brew:curl"]
        )
        let presentation = PackagePresentation(
            item: .installed(record),
            detail: detail,
            freshness: 0
        )

        XCTAssertFalse(detail.installed)
        XCTAssertEqual(presentation.plainTextSecretAlertSource, .isotope)
        XCTAssertTrue(presentation.hasActivePlainTextSecretAlert)
        XCTAssertFalse(presentation.plainTextSecretAlertIsGhosted)
    }

    private func decodePackageDetail(
        packageName: String = "brew:git",
        formula: String = "git",
        securityState: String
    ) throws -> PackageDetail {
        let json = """
        {
          "packageName": "\(packageName)",
          "qualifiedName": "\(packageName)",
          "installRoot": "/opt/homebrew/Cellar/\(formula)",
          "installed": false,
          "source": {"kind": "formula", "rootFormula": "\(formula)"},
          "sourceError": null,
          "aliases": [],
          "aliasesError": null,
          "installedVersion": null,
          "latestVersion": "1.0",
          "latestVersionError": null,
          "executablePaths": [],
          "executablePathsError": null,
          "popularity": null,
          "lastUpdatedAt": null,
          "homebrewInfo": null,
          "homebrewInfoError": null,
          "npmHomepage": null,
          "npmPackageInfoError": null,
          "securityState": \(securityState),
          "installPackageNames": null,
          "homebrewMigration": null,
          "versionOptions": [],
          "managementBackend": "nucleus"
        }
        """
        return try JSONDecoder().decode(PackageDetail.self, from: Data(json.utf8))
    }
}
