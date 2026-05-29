import Foundation
import XCTest
@testable import AutomicVaultApp

final class MainWindowModelTests: XCTestCase {
    @MainActor
    func testPulseTimestampUsesHoursUntilSixtyHours() throws {
        let referenceDate = try XCTUnwrap(Self.date("2026-05-28T12:00:00Z"))
        let result = pulseResult(
            lastUpdatedAt: "2026-05-26T01:00:00Z",
            pulseKind: "updated"
        )

        XCTAssertEqual(
            MainWindowModel.pulseListTimestampText(
                for: result,
                relativeTo: referenceDate
            ),
            "Updated 59 hours ago"
        )
    }

    @MainActor
    func testPulseTimestampFallsBackAfterSixtyHours() throws {
        let referenceDate = try XCTUnwrap(Self.date("2026-05-28T12:00:00Z"))
        let result = pulseResult(
            lastUpdatedAt: "2026-05-25T23:00:00Z",
            pulseKind: "updated"
        )

        let text = MainWindowModel.pulseListTimestampText(
            for: result,
            relativeTo: referenceDate
        )

        XCTAssertTrue(text.hasPrefix("Updated "))
        XCTAssertFalse(text.contains("61 hours ago"))
    }

    @MainActor
    func testNewPulseTimestampShowsAgeWithoutUpdatedPrefix() throws {
        let referenceDate = try XCTUnwrap(Self.date("2026-05-28T12:00:00Z"))
        let result = pulseResult(
            lastUpdatedAt: "2026-05-28T00:00:00.000Z",
            pulseKind: "new"
        )

        XCTAssertEqual(
            MainWindowModel.pulseListTimestampText(
                for: result,
                relativeTo: referenceDate
            ),
            "12 hours ago"
        )
    }

    func testAvailableNpmPackageShowsSourceLabelInsteadOfLatestVersion() {
        let result = PackageSearchResult(
            name: "npm:openclaw",
            source: .npm(packageName: "openclaw"),
            version: "2026.5.22",
            description: "Multi-channel AI gateway",
            homepage: nil,
            dependencies: [],
            securityState: nil,
            pulseKind: nil
        )
        let presentation = PackagePresentation(
            item: .available(result),
            detail: result.fallbackDetail,
            freshness: 0
        )

        XCTAssertEqual(presentation.versionText, "NPM")
    }

    func testUninstalledFormulaInstallsThroughUnqualifiedAutoTarget() {
        let result = PackageSearchResult(
            name: "uv",
            source: .formula(rootFormula: "uv"),
            version: "0.8.23",
            description: "Python package manager",
            homepage: nil,
            dependencies: [],
            securityState: nil,
            pulseKind: nil
        )

        XCTAssertEqual(result.fallbackDetail.helperPackageNames, ["uv"])
        XCTAssertEqual(result.fallbackDetail.installCommand, "av install uv")
    }

    func testInstalledFormulaKeepsExplicitBrewTarget() {
        let record = PackageRecord(
            name: "uv",
            source: .formula(rootFormula: "uv"),
            version: "0.8.23",
            description: "Python package manager",
            securityState: nil
        )

        XCTAssertEqual(record.fallbackDetail.helperPackageNames, ["brew:uv"])
    }

    func testAppBadgeCountCombinesOutdatedPackagesAndSecurityAlerts() {
        let snapshot = NucleusStatusSnapshot(
            installedCount: 10,
            hazardousPackageCount: 2,
            outdatedPackages: [
                OutdatedPackageRecord(
                    name: "brew:rg",
                    currentVersion: "1.0",
                    latestVersion: "2.0"
                )
            ],
            homebrewOutdatedPackages: [
                OutdatedPackageRecord(
                    name: "brew:uv",
                    currentVersion: "0.7.0",
                    latestVersion: "0.8.0"
                )
            ],
            refreshedAt: Date(),
            lastError: nil
        )

        XCTAssertEqual(snapshot.appBadgeCount, 4)
    }

    @MainActor
    func testSearchDeselectsAndRestoresSidebarSection() {
        let model = MainWindowModel()
        defer { model.stop() }
        model.selectedSection = .outdated

        XCTAssertEqual(model.activeSidebarSection, .outdated)

        model.searchText = "openclaw"

        XCTAssertTrue(model.isSearchActive)
        XCTAssertNil(model.activeSidebarSection)

        model.searchText = ""

        XCTAssertFalse(model.isSearchActive)
        XCTAssertEqual(model.activeSidebarSection, .outdated)
    }

    @MainActor
    func testSelectingSidebarSectionCancelsSearch() {
        let model = MainWindowModel()
        defer { model.stop() }
        model.selectedSection = .outdated
        model.searchText = "openclaw"
        let initialDeactivationRequestID = model.searchDeactivationRequestID

        model.selectSection(.newUpdated)

        XCTAssertEqual(model.searchText, "")
        XCTAssertEqual(model.selectedSection, .newUpdated)
        XCTAssertEqual(model.activeSidebarSection, .newUpdated)
        XCTAssertEqual(
            model.searchDeactivationRequestID,
            initialDeactivationRequestID + 1
        )
    }

    @MainActor
    func testSelectingSidebarSectionDeactivatesEmptySearchField() {
        let model = MainWindowModel()
        defer { model.stop() }
        let initialDeactivationRequestID = model.searchDeactivationRequestID

        model.selectSection(.newUpdated)

        XCTAssertEqual(model.searchText, "")
        XCTAssertEqual(model.selectedSection, .newUpdated)
        XCTAssertEqual(
            model.searchDeactivationRequestID,
            initialDeactivationRequestID + 1
        )
    }

    @MainActor
    func testSearchUsesNeutralVersionTextDespiteSelectedSection() {
        let model = MainWindowModel()
        defer { model.stop() }
        model.selectedSection = .outdated
        let package = installedPresentation(
            version: "1.0",
            latestVersion: "2.0"
        )

        XCTAssertEqual(model.versionText(for: package), "1.0 → 2.0")

        model.searchText = "rg"

        XCTAssertEqual(model.versionText(for: package), "1.0")
        XCTAssertEqual(model.packageInlineBadges(for: package), [])
    }

    @MainActor
    func testSearchPrefersInstalledPackageOverMatchingDaemonResult() {
        let installedFlyctlRecord = PackageRecord(
            name: "flyctl",
            source: .formula(rootFormula: "flyctl"),
            version: "0.4.57",
            description: "Command-line tools for fly.io services",
            securityState: nil
        )
        let installedFlyctl = PackagePresentation(
            item: .installed(installedFlyctlRecord),
            detail: installedFlyctlRecord.fallbackDetail,
            freshness: 0
        )
        let daemonFlyctl = searchPresentation(
            name: "flyctl",
            formula: "flyctl",
            description: "Command-line tools for fly.io services"
        )
        let daemonFlye = searchPresentation(
            name: "flye",
            formula: "flye",
            description: "De novo assembler for single molecule sequencing reads"
        )

        let merged = MainWindowModel.mergedSearchPackages(
            installed: [installedFlyctl],
            daemon: [daemonFlye, daemonFlyctl]
        )

        XCTAssertEqual(merged.map(\.selectionID), ["flyctl", "search:flye"])
    }

    @MainActor
    func testSecurityAlertsPreferInstalledPackageOverMatchingDetectorRow() throws {
        let flyctlState = securityState(
            isotopeName: "flyctl",
            reason: "flyctl config file contains a plaintext access token"
        )
        let installedFlyctlRecord = PackageRecord(
            name: "flyctl",
            source: .formula(rootFormula: "flyctl"),
            version: "0.4.57",
            description: "Command-line tools for fly.io services",
            securityState: flyctlState
        )
        let installedFlyctl = PackagePresentation(
            item: .installed(installedFlyctlRecord),
            detail: installedFlyctlRecord.fallbackDetail,
            freshness: 0
        )
        let detectedFlyctl = try XCTUnwrap(
            PackageSearchResult(
                name: "flyctl",
                source: .formula(rootFormula: "flyctl"),
                version: nil,
                description: "Detector flagged local plaintext credential exposure",
                homepage: nil,
                dependencies: [],
                securityState: flyctlState,
                pulseKind: nil
            )
            .detectedLocalHazardPresentation(freshness: 0)?
            .presentation
        )
        let detectedSupabase = try XCTUnwrap(
            PackageSearchResult(
                name: "supabase-cli",
                source: .formula(rootFormula: "supabase-cli"),
                version: nil,
                description: "Detector flagged local plaintext credential exposure",
                homepage: nil,
                dependencies: [],
                securityState: securityState(
                    isotopeName: "supabase-cli",
                    reason: "Supabase access token is readable by /usr/bin/security"
                ),
                pulseKind: nil
            )
            .detectedLocalHazardPresentation(freshness: 0)?
            .presentation
        )

        let alerts = MainWindowModel.securityAlertPackages(
            installed: [installedFlyctl],
            geiger: [detectedFlyctl, detectedSupabase]
        )

        XCTAssertEqual(
            alerts.map(\.selectionID),
            ["flyctl", "gone:supabase-cli"]
        )
    }

    @MainActor
    func testSecurityAlertsDedupeInstalledIsotopeAgainstStaleGeigerRow() {
        let flyctlState = securityState(
            isotopeName: "flyctl",
            reason: "flyctl config file contains a plaintext access token"
        )
        let installedFlyctlRecord = PackageRecord(
            name: "isotope:flyctl",
            source: .isotope(isotopeName: "flyctl"),
            version: "0.4.57",
            description: "Command-line tools for fly.io services",
            securityState: flyctlState
        )
        let installedFlyctl = PackagePresentation(
            item: .installed(installedFlyctlRecord),
            detail: installedFlyctlRecord.fallbackDetail,
            freshness: 0
        )
        let staleGeigerFlyctlResult = PackageSearchResult(
            name: "brew:flyctl",
            source: .formula(rootFormula: "flyctl"),
            version: nil,
            description: "Detector flagged local plaintext credential exposure",
            homepage: nil,
            dependencies: [],
            securityState: nil,
            pulseKind: nil
        )
        let staleGeigerFlyctl = PackagePresentation(
            item: .available(staleGeigerFlyctlResult),
            detail: staleGeigerFlyctlResult.fallbackDetail,
            freshness: 0,
            presentationID: "geiger:brew:flyctl"
        )

        let alerts = MainWindowModel.securityAlertPackages(
            installed: [installedFlyctl],
            geiger: [staleGeigerFlyctl]
        )

        XCTAssertEqual(alerts.map(\.selectionID), ["isotope:flyctl"])
    }

    @MainActor
    func testOutdatedAutomicVaultCLTAppearsInOutdatedSection() throws {
        let recommendation = PackageRecommendation.automicVaultCLT(
            installedVersion: "1.0",
            latestVersion: "2.0",
            missingToolNames: []
        )
        let model = MainWindowModel(
            cliToolsRecommendationProvider: { recommendation },
            initialAutomicVaultCLTRecommendation: recommendation
        )
        defer { model.stop() }

        model.selectedSection = .outdated

        XCTAssertFalse(model.shouldShowAutomicVaultCLTInstallButton)
        XCTAssertTrue(model.shouldUpdateAutomicVaultCLTWithUpdateAll)
        XCTAssertEqual(model.outdatedUpdatePackageNames, ["av"])
        XCTAssertEqual(model.count(for: .outdated), 1)

        let package = try XCTUnwrap(model.displayedPackages.first)
        XCTAssertEqual(package.selectionID, PackageRecommendation.automicVaultCLTName)
        XCTAssertEqual(model.displayName(for: package), PackageRecommendation.automicVaultCLTName)
        XCTAssertEqual(model.versionText(for: package), "v1.0 → v2.0")

        model.select(package)

        XCTAssertFalse(model.isLoadingDetail)
        XCTAssertEqual(model.selectedDetail?.packageName, PackageRecommendation.automicVaultCLTName)
    }

    @MainActor
    func testMissingAutomicVaultCLTRequestsInstallFromToolbarState() throws {
        let recommendation = PackageRecommendation.automicVaultCLT(
            installedVersion: nil,
            latestVersion: "2.0",
            missingToolNames: ["av"]
        )
        let model = MainWindowModel(
            cliToolsRecommendationProvider: { recommendation },
            initialAutomicVaultCLTRecommendation: recommendation
        )
        defer { model.stop() }

        model.selectedSection = .outdated

        XCTAssertTrue(model.shouldShowAutomicVaultCLTInstallButton)
        XCTAssertTrue(model.canRequestAutomicVaultCLTInstall)
        XCTAssertFalse(model.shouldUpdateAutomicVaultCLTWithUpdateAll)
        XCTAssertTrue(model.displayedPackages.isEmpty)

        model.requestAutomicVaultCLTInstall()

        let request = try XCTUnwrap(model.packageOperationRequest)
        XCTAssertEqual(request.kind, .install)
        XCTAssertEqual(request.packageNames, ["av"])
        XCTAssertEqual(request.displayName, "av")
        XCTAssertTrue(request.isAutomicVaultCLT)
        XCTAssertFalse(request.isXcodeCLT)
    }

    @MainActor
    func testDossierPrimaryActionFollowsInstallState() {
        let model = MainWindowModel()
        defer { model.stop() }
        let available = PackageSearchResult(
            name: "brew:fd",
            source: .formula(rootFormula: "fd"),
            version: "10.0",
            description: "Find entries",
            homepage: nil,
            dependencies: [],
            securityState: nil,
            pulseKind: nil
        )
        let installed = installedPresentation(version: "1.0", latestVersion: "1.0")
        let outdated = installedPresentation(version: "1.0", latestVersion: "2.0")

        XCTAssertEqual(
            model.dossierPrimaryPackageAction(for: available.fallbackDetail),
            .install
        )
        XCTAssertEqual(
            model.dossierPrimaryPackageAction(for: installed.detail!),
            .uninstall
        )
        XCTAssertEqual(
            model.dossierPrimaryPackageAction(for: outdated.detail!),
            .update
        )
    }

    @MainActor
    func testDossierPrimaryActionHardensInsecureInstalledBrewFormula() throws {
        let model = MainWindowModel()
        defer { model.stop() }
        let state = securityState(
            isotopeName: "gh",
            reason: "GitHub token is stored in plaintext"
        )
        let record = PackageRecord(
            name: "brew:gh",
            source: .formula(rootFormula: "gh"),
            version: "2.49.0",
            description: "GitHub command line tool",
            securityState: state,
            installRoot: "/opt/homebrew/Cellar/gh",
            installPackageNames: ["brew:gh"]
        )
        let package = PackagePresentation(
            item: .installed(record),
            detail: record.fallbackDetail,
            freshness: 0
        )
        let detail = try XCTUnwrap(package.detail)

        XCTAssertEqual(model.dossierPrimaryPackageAction(for: detail), .harden)
        XCTAssertTrue(model.canRequestDossierPackageAction(.harden, detail: detail))
        XCTAssertEqual(PackageOperationKind.harden.title, "Harden")
        XCTAssertEqual(PackageOperationKind.harden.progressTitle, "Hardening")

        model.requestDossierPackageAction(.harden, detail: detail, package: package)

        let request = try XCTUnwrap(model.packageOperationRequest)
        XCTAssertEqual(request.kind, .harden)
        XCTAssertEqual(request.packageNames, ["isotope:gh"])
        XCTAssertEqual(request.migrationIsotopeName, "gh")
        XCTAssertEqual(request.displayName, "gh")
    }

    @MainActor
    func testDossierPrimaryActionHidesDetectorOnlySecurityIssue() throws {
        let model = MainWindowModel()
        defer { model.stop() }
        let detail = PackageSearchResult(
            name: "curl",
            source: .formula(rootFormula: "curl"),
            version: nil,
            description: "Detector flagged local plaintext credential exposure",
            homepage: nil,
            dependencies: [],
            securityState: PackageSecurityState(
                isotopeName: "curl",
                installIsInsecure: true,
                remediationAvailable: false,
                reasons: ["curl netrc file contains plaintext credentials"],
                error: nil
            ),
            pulseKind: nil
        )
        .detectedLocalHazardPresentation(freshness: 0)?
        .detail

        let unwrappedDetail = try XCTUnwrap(detail)
        XCTAssertNil(model.dossierPrimaryPackageAction(for: unwrappedDetail))
        XCTAssertFalse(model.canRequestDossierPackageAction(.install, detail: unwrappedDetail))
        XCTAssertFalse(model.canRequestDossierPackageAction(.harden, detail: unwrappedDetail))
    }

    @MainActor
    func testDossierPrimaryActionHardensInstallableIsotopeWithoutMigration() throws {
        let bundle = try makeSecurityCatalogBundle(combinedJSON: """
        {
          "sources": {
            "isotopes": {
              "supabase-cli": {
                "name": "isotope:supabase-cli",
                "replaces": "brew:supabase",
                "repository": "automic-vault/supabase-cli",
                "releaseUrl": "https://github.com/automic-vault/supabase-cli/releases/tag/v2.101.0",
                "archiveUrl": "https://example.test/supabase-cli.tgz"
              }
            }
          }
        }
        """)
        let model = MainWindowModel(securityCatalog: SecurityCatalog(bundle: bundle))
        defer { model.stop() }
        let detail = PackageSearchResult(
            name: "supabase-cli",
            source: .formula(rootFormula: "supabase-cli"),
            version: nil,
            description: "Detector flagged local plaintext credential exposure",
            homepage: nil,
            dependencies: [],
            securityState: PackageSecurityState(
                isotopeName: "supabase-cli",
                installIsInsecure: true,
                remediationAvailable: false,
                reasons: ["Supabase access token is readable by /usr/bin/security"],
                error: nil
            ),
            pulseKind: nil
        )
        .detectedLocalHazardPresentation(freshness: 0)?
        .detail

        let unwrappedDetail = try XCTUnwrap(detail)
        XCTAssertEqual(model.dossierPrimaryPackageAction(for: unwrappedDetail), .harden)
        XCTAssertTrue(model.canRequestDossierPackageAction(.harden, detail: unwrappedDetail))
        XCTAssertEqual(
            model.linkURL(for: .homepage, detail: unwrappedDetail)?.absoluteString,
            "https://github.com/automic-vault/supabase-cli/releases/tag/v2.101.0"
        )
        XCTAssertEqual(
            model.linkURL(for: .repository, detail: unwrappedDetail)?.absoluteString,
            "https://github.com/automic-vault/supabase-cli"
        )
        XCTAssertEqual(
            model.linkURL(for: .documentation, detail: unwrappedDetail)?.absoluteString,
            "https://github.com/automic-vault/supabase-cli/wiki"
        )
    }

    @MainActor
    func testDossierActionRequestUsesHelperPackageNames() throws {
        let model = MainWindowModel()
        defer { model.stop() }
        let package = installedPresentation(version: "1.0", latestVersion: "2.0")
        let detail = try XCTUnwrap(package.detail)

        model.requestDossierPackageAction(.update, detail: detail, package: package)

        let request = try XCTUnwrap(model.packageOperationRequest)
        XCTAssertEqual(request.kind, .update)
        XCTAssertEqual(request.packageNames, ["brew:rg"])
        XCTAssertEqual(request.displayName, "rg")
    }

    private func pulseResult(
        lastUpdatedAt: String?,
        pulseKind: String
    ) -> PackageSearchResult {
        PackageSearchResult(
            name: "brew:example",
            source: .formula(rootFormula: "example"),
            version: "1.0",
            description: "Example package",
            homepage: nil,
            dependencies: [],
            lastUpdatedAt: lastUpdatedAt,
            securityState: nil,
            pulseKind: pulseKind
        )
    }

    private func installedPresentation(
        version: String,
        latestVersion: String
    ) -> PackagePresentation {
        let record = PackageRecord(
            name: "brew:rg",
            source: .formula(rootFormula: "rg"),
            version: version,
            description: "Search tool",
            latestVersion: latestVersion,
            securityState: nil
        )
        return PackagePresentation(
            item: .installed(record),
            detail: record.fallbackDetail,
            freshness: 0
        )
    }

    private func searchPresentation(
        name: String,
        formula: String,
        description: String
    ) -> PackagePresentation {
        let result = PackageSearchResult(
            name: name,
            source: .formula(rootFormula: formula),
            version: nil,
            description: description,
            homepage: nil,
            dependencies: [],
            securityState: nil,
            pulseKind: nil
        )
        return PackagePresentation(
            item: .available(result),
            detail: result.fallbackDetail,
            freshness: 0,
            presentationID: "search:\(name)"
        )
    }

    private func securityState(isotopeName: String, reason: String) -> PackageSecurityState {
        PackageSecurityState(
            isotopeName: isotopeName,
            installIsInsecure: true,
            remediationAvailable: true,
            reasons: [reason],
            error: nil
        )
    }

    private func makeSecurityCatalogBundle(combinedJSON: String) throws -> Bundle {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("MainWindowModelTests-\(UUID().uuidString).bundle")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let infoPlist = """
        <?xml version="1.0" encoding="UTF-8"?>
        <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
        <plist version="1.0">
        <dict>
          <key>CFBundleIdentifier</key>
          <string>com.automic-vault.MainWindowModelTests</string>
          <key>CFBundlePackageType</key>
          <string>BNDL</string>
        </dict>
        </plist>
        """
        try Data(combinedJSON.utf8).write(to: directory.appendingPathComponent("combined.json"))
        try Data("[]".utf8).write(to: directory.appendingPathComponent("enrichment-manifests.json"))
        try Data(infoPlist.utf8).write(to: directory.appendingPathComponent("Info.plist"))
        return try XCTUnwrap(Bundle(url: directory))
    }

    private static func date(_ raw: String) -> Date? {
        ISO8601DateFormatter().date(from: raw)
    }
}
