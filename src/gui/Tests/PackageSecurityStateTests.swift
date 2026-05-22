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
        let lookupDetail = try decodePackageDetail(
            packageName: "brew:curl",
            formula: "curl",
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
        let detail = lookupDetail.withPackageIdentity(
            packageName: "sys:curl",
            installPackageNames: ["brew:curl"]
        )
        let record = PackageRecord(
            name: "sys:curl",
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
        XCTAssertEqual(record.name, "sys:curl")
        XCTAssertEqual(detail.packageName, "sys:curl")
        XCTAssertEqual(detail.qualifiedName, "sys:curl")
        XCTAssertEqual(detail.installPackageNames, ["brew:curl"])
        XCTAssertEqual(detail.source, .formula(rootFormula: "curl"))
        XCTAssertEqual("sys:curl".packageSearchOrderName, "curl")
        XCTAssertTrue(detail.isSystemDetectorOnlyHazard)
        XCTAssertEqual(presentation.plainTextSecretAlertSource, .isotope)
        XCTAssertNotNil(detail.securityNotice)
        XCTAssertTrue(presentation.hasActivePlainTextSecretAlert)
        XCTAssertFalse(presentation.plainTextSecretAlertIsGhosted)
    }

    func testSystemDetectorHazardsSortWithInstalledPackagesByToolName() {
        let presentations = [
            installedPresentation(named: "sys:curl"),
            installedPresentation(named: "brew:bat"),
            installedPresentation(named: "ack"),
            installedPresentation(named: "sys:git"),
        ].sorted(by: PackagePresentation.sortsByPackageSearchOrder)

        XCTAssertEqual(
            presentations.map(\.selectionID),
            ["ack", "brew:bat", "sys:curl", "sys:git"]
        )
    }

    func testGoneDetectorHazardsSortByToolName() {
        let presentations = [
            installedPresentation(named: "gone:hf"),
            installedPresentation(named: "brew:git"),
        ].sorted(by: PackagePresentation.sortsByPackageSearchOrder)

        XCTAssertEqual(
            presentations.map(\.selectionID),
            ["brew:git", "gone:hf"]
        )
        XCTAssertEqual("gone:hf".packageSearchOrderName, "hf")
    }

    func testLocalHazardsMixWithInstalledPackagesByToolName() throws {
        let safe = installedPresentation(named: "brew:ack")
        let hazardDetail = try decodePackageDetail(
            packageName: "gone:hf",
            formula: "hf",
            securityState: """
            {
              "isotopeName": "huggingface-cli",
              "installIsInsecure": true,
              "remediationAvailable": true,
              "reasons": ["Hugging Face token file contains a plaintext token"],
              "error": null
            }
            """
        )
        let hazard = PackagePresentation(
            item: .installed(PackageRecord(
                name: "gone:hf",
                source: .formula(rootFormula: "hf"),
                version: "1.15.0",
                description: "Client library for huggingface.co hub",
                securityState: hazardDetail.securityState,
                installPackageNames: ["brew:hf"]
            )),
            detail: hazardDetail,
            freshness: 0
        )

        XCTAssertEqual(hazard.plainTextSecretAlertSource, .isotope)
        XCTAssertTrue(hazard.hasActivePlainTextSecretAlert)
        XCTAssertEqual(
            [hazard, safe]
                .sorted(by: PackagePresentation.sortsByPackageSearchOrder)
                .map(\.selectionID),
            ["brew:ack", "gone:hf"]
        )
    }

    @MainActor
    func testSystemDetectorOnlyDossierHidesPackageActions() throws {
        let lookupDetail = try decodePackageDetail(
            packageName: "brew:curl",
            formula: "curl",
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
        let detail = lookupDetail.withPackageIdentity(
            packageName: "sys:curl",
            installPackageNames: ["brew:curl"]
        )

        let view = DossierView(frame: NSRect(x: 0, y: 0, width: 290, height: 800))
        view.render(detail: detail, animation: .none)
        view.layoutSubtreeIfNeeded()

        let visibleButtonTitles = view.subviews
            .compactMap { $0 as? NSButton }
            .filter { !$0.isHidden }
            .map(\.title)

        XCTAssertEqual(visibleButtonTitles, ["LEARN MORE"])
    }

    @MainActor
    func testDossierSecurityNoticeUsesWrappedTextFields() throws {
        let detail = try decodePackageDetail(
            packageName: "brew:curl",
            formula: "curl",
            securityState: """
            {
              "isotopeName": "curl",
              "installIsInsecure": true,
              "remediationAvailable": false,
              "reasons": ["curl netrc file contains plaintext credentials: /Users/test/.netrc"],
              "error": null
            }
            """
        )

        XCTAssertNotNil(detail.securityNotice)

        let view = DossierView(frame: NSRect(x: 0, y: 0, width: 290, height: 800))
        view.render(detail: detail, animation: .none)
        view.layoutSubtreeIfNeeded()

        let noticeFields = view.subviews.compactMap { $0 as? NSTextField }
            .filter { $0.attributedStringValue.length > 0 }
        let bodyField = try XCTUnwrap(
            noticeFields.first {
                $0.attributedStringValue.string.contains("Automic Vault")
            }
        )

        XCTAssertEqual(bodyField.lineBreakMode, .byWordWrapping)
        let paragraphStyle = bodyField.attributedStringValue.attribute(
            .paragraphStyle,
            at: 0,
            effectiveRange: nil
        ) as? NSParagraphStyle
        XCTAssertEqual(paragraphStyle?.lineBreakMode, .byWordWrapping)
        XCTAssertTrue(bodyField.cell?.wraps == true)
        XCTAssertGreaterThan(bodyField.frame.height, 34)
        XCTAssertLessThanOrEqual(bodyField.frame.maxX, view.bounds.maxX)
        for field in noticeFields {
            let cellHeight = field.cell?.cellSize(forBounds: NSRect(
                x: 0,
                y: 0,
                width: field.frame.width,
                height: .greatestFiniteMagnitude
            )).height ?? 0
            XCTAssertGreaterThanOrEqual(field.frame.height, ceil(cellHeight))
        }
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

    private func installedPresentation(named name: String) -> PackagePresentation {
        PackagePresentation(
            item: .installed(PackageRecord(
                name: name,
                source: nil,
                version: "1.0",
                description: nil,
                securityState: nil
            )),
            detail: nil,
            freshness: 0
        )
    }
}
