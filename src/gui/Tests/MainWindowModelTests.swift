import Foundation
import XCTest
@testable import AutomicVaultApp

final class MainWindowModelTests: XCTestCase {
    func testSidebarGroupsMoveCatalogSectionsIntoEcosystem() {
        XCTAssertEqual(
            MainWindowSection.librarySections,
            [.installed, .geigerCounter, .outdated]
        )
        XCTAssertEqual(
            MainWindowSection.ecosystemSections,
            [.newUpdated, .allPackages]
        )
    }

    func testCategorySectionsAreAlphabetizedByDisplayedTitleWithOtherLast() {
        let sections = MainWindowSection.categorySections
        XCTAssertEqual(sections.last, .other)

        let regularTitles = sections.dropLast().map(\.title)
        let sortedTitles = regularTitles.sorted {
            $0.localizedStandardCompare($1) == .orderedAscending
        }

        XCTAssertEqual(regularTitles, sortedTitles)
    }

    @MainActor
    func testAllPackagesLoadsNextPageWhenScrolledNearEnd() async throws {
        let requests = PageRequestRecorder()
        let model = MainWindowModel(
            availablePackagesFetcher: { offset, _ in
                requests.append(offset)
                switch offset {
                case 0:
                    return PackageSearchPage(
                        packages: [
                            Self.packageSearchResult(name: "brew:alpha"),
                            Self.packageSearchResult(name: "brew:bravo"),
                        ],
                        totalCount: 3,
                        nextOffset: 2
                    )
                case 2:
                    return PackageSearchPage(
                        packages: [
                            Self.packageSearchResult(name: "brew:charlie"),
                        ],
                        totalCount: 3,
                        nextOffset: nil
                    )
                default:
                    return PackageSearchPage(
                        packages: [],
                        totalCount: 3,
                        nextOffset: nil
                    )
                }
            }
        )
        defer { model.stop() }

        model.selectedSection = .allPackages
        await waitUntil(model.displayedPackages.count == 2)

        let lastPackage = try XCTUnwrap(model.displayedPackages.last)
        model.loadNextPageIfNeeded(after: lastPackage)

        await waitUntil(model.displayedPackages.count == 3)

        XCTAssertEqual(requests.values, [0, 2])
        XCTAssertEqual(
            model.displayedPackages.map(\.selectionID),
            ["brew:alpha", "brew:bravo", "brew:charlie"]
        )
    }

    @MainActor
    func testCategorySectionUsesDatabaseCategoryMetadata() async throws {
        let model = MainWindowModel(
            availablePackagesFetcher: { _, _ in
                PackageSearchPage(
                    packages: [
                        Self.packageSearchResult(
                            name: "brew:uv",
                            category: "developer-tools"
                        ),
                        Self.packageSearchResult(
                            name: "brew:gh",
                            category: "developer-tools"
                        ),
                        Self.packageSearchResult(
                            name: "brew:sops",
                            category: "security"
                        ),
                        Self.packageSearchResult(
                            name: "cask:codex",
                            category: nil
                        ),
                    ],
                    totalCount: 4,
                    nextOffset: nil,
                    categoryCounts: [
                        "developer-tools": 2,
                        "security": 1,
                        "other": 1,
                    ]
                )
            }
        )
        defer { model.stop() }

        model.selectedSection = .developerTools
        await waitUntil(model.displayedPackages.count == 2)

        XCTAssertEqual(model.count(for: .developerTools), 2)
        XCTAssertEqual(model.count(for: .security), 1)
        XCTAssertEqual(model.count(for: .other), 1)
        XCTAssertEqual(
            model.displayedPackages.map(\.selectionID),
            ["brew:uv", "brew:gh"]
        )

        model.selectedSection = .security

        XCTAssertEqual(model.displayedPackages.map(\.selectionID), ["brew:sops"])
    }

    @MainActor
    func testPackageLinksPreferExplicitRepositoryAndDocsMetadata() throws {
        let model = MainWindowModel()
        defer { model.stop() }
        let detail = PackageSearchResult(
            name: "brew:uv",
            source: .formula(rootFormula: "uv"),
            version: "0.11.17",
            description: "Python package manager",
            homepage: "https://docs.astral.sh/uv/",
            repository: "https://github.com/astral-sh/uv",
            upstreamDocs: "https://docs.astral.sh/uv",
            docs: ["https://docs.astral.sh/uv"],
            category: "developer-tools",
            dependencies: [],
            securityState: nil,
            pulseKind: nil
        ).fallbackDetail

        XCTAssertEqual(
            model.linkURL(for: .homepage, detail: detail)?.absoluteString,
            "https://docs.astral.sh/uv/"
        )
        XCTAssertEqual(
            model.linkURL(for: .repository, detail: detail)?.absoluteString,
            "https://github.com/astral-sh/uv"
        )
        XCTAssertEqual(
            model.linkURL(for: .documentation, detail: detail)?.absoluteString,
            "https://docs.astral.sh/uv"
        )
    }

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

    func testGroupedVersionedFormulaLoadsDetailFromInstalledFormula() {
        let record = PackageRecord(
            name: "node",
            source: .formula(rootFormula: "node@24"),
            version: "24.11.1",
            description: "Platform built on V8",
            securityState: nil,
            installPackageNames: ["node@24"],
            installedVersions: ["24.11.1"]
        )
        let presentation = PackagePresentation(
            item: .installed(record),
            detail: record.fallbackDetail,
            freshness: 0
        )

        XCTAssertEqual(presentation.selectionID, "node")
        XCTAssertEqual(presentation.preferredDetailLookupName, "brew:node@24")
        XCTAssertEqual(record.fallbackDetail.helperPackageNames, ["node@24"])
    }

    func testPackagePackInstallTargetsUseSourceQualifiedNames() {
        XCTAssertEqual(
            PackagePack.agent.installPackageNames.prefix(2),
            ["cask:codex", "brew:claude-code"]
        )
        XCTAssertEqual(
            PackagePack.agenticToolkit.installPackageNames.first,
            "brew:ffmpeg-full"
        )
        XCTAssertEqual(
            PackagePack.unixPlusPlus.installPackageNames.first,
            "brew:bat"
        )
        XCTAssertEqual(
            PackagePack.agent.installPackageNames.count,
            PackagePack.agent.packageNames.count
        )
        XCTAssertEqual(
            PackagePack.agenticToolkit.installPackageNames.count,
            PackagePack.agenticToolkit.packageNames.count
        )
        XCTAssertEqual(
            PackagePack.unixPlusPlus.installPackageNames.count,
            PackagePack.unixPlusPlus.packageNames.count
        )
    }

    func testAppBadgeCountCombinesNucleusOutdatedPackagesAndSecurityAlerts() {
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
            refreshedAt: Date(),
            lastError: nil
        )

        XCTAssertEqual(snapshot.appBadgeCount, 3)
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

    private static func packageSearchResult(
        name: String,
        category: String? = nil
    ) -> PackageSearchResult {
        PackageSearchResult(
            name: name,
            source: .formula(rootFormula: name.replacingOccurrences(of: "brew:", with: "")),
            version: "1.0",
            description: "\(name) package",
            homepage: nil,
            category: category,
            dependencies: [],
            securityState: nil,
            pulseKind: nil
        )
    }

    @MainActor
    private func waitUntil(
        _ condition: @autoclosure @escaping @MainActor () -> Bool,
        file: StaticString = #filePath,
        line: UInt = #line
    ) async {
        for _ in 0..<50 {
            if condition() {
                return
            }
            try? await Task.sleep(for: .milliseconds(20))
        }
        XCTFail("Timed out waiting for condition", file: file, line: line)
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

private final class PageRequestRecorder: @unchecked Sendable {
    private let lock = NSLock()
    private var offsets: [Int] = []

    var values: [Int] {
        lock.withLock { offsets }
    }

    func append(_ offset: Int) {
        lock.withLock {
            offsets.append(offset)
        }
    }
}
