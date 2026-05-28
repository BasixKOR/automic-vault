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

    private static func date(_ raw: String) -> Date? {
        ISO8601DateFormatter().date(from: raw)
    }
}
