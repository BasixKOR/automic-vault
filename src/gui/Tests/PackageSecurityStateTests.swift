import XCTest
@testable import AutomicVaultApp

final class PackageSecurityStateTests: XCTestCase {
    func testInstalledSummaryHomepageIsAvailableFromFallbackDetail() throws {
        let record = PackageRecord(
            name: "brew:curl",
            source: .formula(rootFormula: "curl"),
            version: "8.20.0",
            description: "Get a file from an HTTP, HTTPS or FTP server",
            homepage: "https://curl.se",
            securityState: nil
        )

        XCTAssertEqual(record.fallbackDetail.homepageURL?.absoluteString, "https://curl.se")
    }

    func testHomebrewFormulaPageIsNotUsedAsFallbackHomepage() throws {
        let record = PackageRecord(
            name: "brew:ripgrep",
            source: .formula(rootFormula: "ripgrep"),
            version: "14.1.1",
            description: "Search tool",
            homepage: "https://formulae.brew.sh/formula/ripgrep",
            securityState: nil
        )

        XCTAssertNil(record.fallbackDetail.homepageURL)
    }

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

    func testDossierWarningContentIncludesFullHazardOutput() throws {
        let detail = try decodePackageDetail(
            packageName: "brew:curl",
            formula: "curl",
            securityState: """
            {
              "isotopeName": "curl",
              "installIsInsecure": true,
              "remediationAvailable": false,
              "reasons": [
                "curl netrc file contains plaintext credentials: /Users/test/.netrc",
                "curl config includes a plaintext bearer token"
              ],
              "error": null
            }
            """
        )

        let warning = try XCTUnwrap(DossierSecurityWarningContent(detail: detail))

        XCTAssertEqual(warning.headline, "LOCAL SECRET EXPOSURE")
        XCTAssertTrue(warning.body.contains("plaintext secret exposure"))
        XCTAssertEqual(
            warning.reasons,
            [
                "curl netrc file contains plaintext credentials: /Users/test/.netrc",
                "curl config includes a plaintext bearer token",
            ]
        )
        XCTAssertNil(warning.detectorError)
    }

    func testDossierWarningContentIncludesDetectorErrorWithoutNotice() throws {
        let detail = try decodePackageDetail(
            packageName: "brew:not-a-real-detector-test-package",
            formula: "not-a-real-detector-test-package",
            securityState: """
            {
              "isotopeName": "test-isotope",
              "installIsInsecure": false,
              "remediationAvailable": true,
              "reasons": [],
              "error": "detector exited with status 1\\nstderr: permission denied"
            }
            """
        )

        XCTAssertNil(detail.securityNotice)

        let warning = try XCTUnwrap(DossierSecurityWarningContent(detail: detail))

        XCTAssertEqual(warning.headline, "DETECTOR NEEDS REVIEW")
        XCTAssertEqual(
            warning.body,
            "The detector for isotope:test-isotope did not complete cleanly."
        )
        XCTAssertEqual(
            warning.detectorError,
            "detector exited with status 1\nstderr: permission denied"
        )
    }

    func testSecurityCatalogNoticeUsesRadioisotopeJustificationAndCaveats() throws {
        let bundle = try makeSecurityCatalogBundle(combinedJSON: """
        {
          "schema": 1,
          "sources": {
            "isotopes": {
              "curl": {
                "name": "isotope:curl",
                "modifies": "brew:curl",
                "repository": "automic-vault/radioisotopes",
                "justification": {
                  "title": "Plain Text HTTP Credentials",
                  "detail": "`curl` can read reusable HTTP credentials from ~/.netrc and ~/.curlrc.\\n\\nAutomic Vault currently detects this exposure but does not yet provide a\\nmigration or package modification for curl.\\n"
                },
                "caveats": [
                  "We detect non-empty netrc passwords.",
                  "We detect clear auth options and Authorization headers in ~/.curlrc.",
                  "Per-command credentials passed directly on the command line are not scanned."
                ]
              }
            }
          }
        }
        """)
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

        let notice = try XCTUnwrap(SecurityCatalog(bundle: bundle).notice(for: detail))

        XCTAssertEqual(notice.headline, "Plain Text HTTP Credentials")
        XCTAssertTrue(notice.body.contains("`curl` can read reusable HTTP credentials"))
        XCTAssertTrue(notice.body.contains("migration or package modification for curl"))
        XCTAssertEqual(
            notice.reasons,
            ["curl netrc file contains plaintext credentials: /Users/test/.netrc"]
        )
        XCTAssertEqual(
            notice.caveats,
            .bullets([
                "We detect non-empty netrc passwords.",
                "We detect clear auth options and Authorization headers in ~/.curlrc.",
                "Per-command credentials passed directly on the command line are not scanned.",
            ])
        )
        XCTAssertEqual(
            notice.learnMoreURL.absoluteString,
            "https://github.com/automic-vault/radioisotopes/tree/main/curl#readme"
        )
    }

    func testDetectorOnlyNoticeLinksToRadioisotopeReadme() throws {
        let detail = try decodePackageDetail(
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

        let notice = try XCTUnwrap(
            SecurityCatalog(bundle: Bundle(for: Self.self)).notice(for: detail)
        )

        XCTAssertEqual(
            notice.learnMoreURL.absoluteString,
            "https://github.com/automic-vault/radioisotopes/tree/main/curl#readme"
        )
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

    func testInstalledDetectorErrorCountsAsMainWindowSecurityAlert() {
        let record = PackageRecord(
            name: "brew:git",
            source: .formula(rootFormula: "git"),
            version: "2.54.0",
            description: "Distributed revision control system",
            securityState: PackageSecurityState(
                isotopeName: "git",
                installIsInsecure: false,
                reasons: [],
                error: "Detector failed"
            )
        )

        XCTAssertTrue(record.hasMainWindowSecurityAlert)
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

    func testHazardousPulseResultBecomesInstalledLocalDetection() throws {
        let result = PackageSearchResult(
            name: "flyctl",
            source: .formula(rootFormula: "flyctl"),
            version: "0.4.54",
            description: "Command-line tools for fly.io services",
            homepage: nil,
            dependencies: [],
            securityState: PackageSecurityState(
                isotopeName: "flyctl",
                installIsInsecure: true,
                remediationAvailable: true,
                reasons: ["flyctl config file contains a plaintext access token"],
                error: nil
            ),
            pulseKind: "updated"
        )

        let hazard = try XCTUnwrap(result.detectedLocalHazardPresentation(freshness: 0.4))

        XCTAssertEqual(hazard.lookupName, "brew:flyctl")
        XCTAssertEqual(hazard.detail.packageName, "gone:flyctl")
        XCTAssertEqual(hazard.detail.qualifiedName, "gone:flyctl")
        XCTAssertEqual(hazard.detail.installPackageNames, ["brew:flyctl"])
        XCTAssertEqual(hazard.presentation.selectionID, "gone:flyctl")
        XCTAssertEqual(hazard.presentation.packageName, "gone:flyctl")
        XCTAssertTrue(hazard.presentation.hasActivePlainTextSecretAlert)
        XCTAssertFalse(hazard.presentation.plainTextSecretAlertIsGhosted)
        guard case .installed(let record) = hazard.presentation.item else {
            return XCTFail("hazardous pulse results should be rendered as installed rows")
        }
        XCTAssertEqual(record.name, "gone:flyctl")
        XCTAssertEqual(record.installPackageNames, ["brew:flyctl"])
    }

    func testDetectedLocalHazardKeepsAlertAfterDetailLoadWithoutSecurityState() throws {
        let result = PackageSearchResult(
            name: "brew:supabase",
            source: .formula(rootFormula: "supabase"),
            version: nil,
            description: "Detector flagged local plaintext credential exposure",
            homepage: nil,
            dependencies: [],
            securityState: PackageSecurityState(
                isotopeName: "supabase-cli",
                installIsInsecure: true,
                remediationAvailable: true,
                reasons: ["Supabase access token is readable by /usr/bin/security"],
                error: nil
            ),
            pulseKind: nil
        )
        let hazard = try XCTUnwrap(result.detectedLocalHazardPresentation(freshness: 0.4))
        guard case .installed(let record) = hazard.presentation.item else {
            return XCTFail("detected local hazards should be rendered as installed rows")
        }
        XCTAssertEqual(record.version, "")
        let resolvedDetail = try decodePackageDetail(
            packageName: "brew:supabase",
            formula: "supabase",
            securityState: "null"
        )

        XCTAssertEqual(hazard.presentation.selectionID, "gone:supabase")
        XCTAssertEqual(hazard.presentation.preferredDetailLookupName, "brew:supabase")
        XCTAssertTrue(
            hazard.presentation.hasMainWindowSecurityAlert(resolvedDetail: resolvedDetail)
        )

        let preserved = resolvedDetail.preservingLocalSecurityContext(from: hazard.detail)
        XCTAssertEqual(preserved.packageName, "gone:supabase")
        XCTAssertEqual(preserved.qualifiedName, "gone:supabase")
        XCTAssertEqual(preserved.installPackageNames, ["brew:supabase"])
        XCTAssertEqual(preserved.securityState, hazard.detail.securityState)

        let warning = try XCTUnwrap(DossierSecurityWarningContent(detail: preserved))
        XCTAssertEqual(
            warning.reasons,
            ["Supabase access token is readable by /usr/bin/security"]
        )
    }

    func testMacOSSystemHazardUsesSystemPrefixEvenWhenRemediable() throws {
        let result = PackageSearchResult(
            name: "curl",
            source: .formula(rootFormula: "curl"),
            version: "8.20.0",
            description: "Get a file from an HTTP, HTTPS or FTP server",
            homepage: nil,
            dependencies: [],
            securityState: PackageSecurityState(
                isotopeName: "curl",
                installIsInsecure: true,
                remediationAvailable: true,
                reasons: ["curl netrc file contains plaintext credentials"],
                error: nil
            ),
            pulseKind: "updated"
        )

        let hazard = try XCTUnwrap(result.detectedLocalHazardPresentation(freshness: 0.4))

        XCTAssertEqual(hazard.lookupName, "brew:curl")
        XCTAssertEqual(hazard.detail.packageName, "sys:curl")
        XCTAssertEqual(hazard.detail.qualifiedName, "sys:curl")
        XCTAssertEqual(hazard.presentation.selectionID, "sys:curl")
        guard case .installed(let record) = hazard.presentation.item else {
            return XCTFail("hazardous system results should be rendered as installed rows")
        }
        XCTAssertEqual(record.name, "sys:curl")
        XCTAssertEqual(record.installPackageNames, ["brew:curl"])
    }

    func testInstalledAutomicVaultCurlKeepsUnprefixedName() {
        let presentation = PackagePresentation(
            item: .installed(PackageRecord(
                name: "curl",
                source: .formula(rootFormula: "curl"),
                version: "8.20.0",
                description: "Get a file from an HTTP, HTTPS or FTP server",
                securityState: PackageSecurityState(
                    isotopeName: "curl",
                    installIsInsecure: true,
                    remediationAvailable: true,
                    reasons: ["curl netrc file contains plaintext credentials"],
                    error: nil
                )
            )),
            detail: nil,
            freshness: 0
        )

        XCTAssertEqual(presentation.selectionID, "curl")
        XCTAssertEqual(presentation.packageName, "curl")
        XCTAssertEqual(presentation.displayName, "curl")
    }

    func testInstalledHomebrewCurlKeepsBrewPrefix() {
        let package = HomebrewMigrationPackage(
            name: "brew:curl",
            version: "8.20.0",
            description: "Get a file from an HTTP, HTTPS or FTP server",
            tap: "homebrew/core",
            isMigratable: true,
            securityState: PackageSecurityState(
                isotopeName: "curl",
                installIsInsecure: true,
                remediationAvailable: true,
                reasons: ["curl netrc file contains plaintext credentials"],
                error: nil
            )
        )
        let presentation = PackagePresentation(
            item: .installed(package.record),
            detail: nil,
            freshness: 0
        )

        XCTAssertEqual(presentation.selectionID, "brew:curl")
        XCTAssertEqual(presentation.packageName, "brew:curl")
        XCTAssertEqual(presentation.displayName, "brew:curl")
    }

    func testGoneRadioisotopeHazardUsesInstallPathInsteadOfConversion() throws {
        let detail = try decodePackageDetail(
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
        let plan = NucleusBridge.IsotopeMigrationPlan(
            isotopeName: "huggingface-cli",
            replacesPackage: nil,
            modifiesPackage: "brew:hf",
            isRadioisotope: true,
            hasMigration: true
        )

        XCTAssertFalse(PackageSecurityRules.shouldConvertRadioisotope(detail: detail, plan: plan))
    }

    func testManagedModifiedPackageUsesRadioisotopeConversion() throws {
        let detail = try decodePackageDetail(
            packageName: "brew:hf",
            formula: "hf",
            installed: true,
            installRoot: "/opt/hf",
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
        let plan = NucleusBridge.IsotopeMigrationPlan(
            isotopeName: "huggingface-cli",
            replacesPackage: nil,
            modifiesPackage: "brew:hf",
            isRadioisotope: true,
            hasMigration: true
        )

        XCTAssertTrue(PackageSecurityRules.shouldConvertRadioisotope(detail: detail, plan: plan))
    }

    func testHomebrewModifiedPackageUsesInstallPathInsteadOfConversion() throws {
        let detail = try decodePackageDetail(
            packageName: "brew:hf",
            formula: "hf",
            installed: true,
            installRoot: "/opt/homebrew/Cellar/hf",
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
        let plan = NucleusBridge.IsotopeMigrationPlan(
            isotopeName: "huggingface-cli",
            replacesPackage: nil,
            modifiesPackage: "brew:hf",
            isRadioisotope: true,
            hasMigration: true
        )

        XCTAssertFalse(PackageSecurityRules.shouldConvertRadioisotope(detail: detail, plan: plan))
    }

    @MainActor
    func testRootInstalledPackageShowsImmutableBadge() {
        let model = MainWindowModel()
        let package = installedPresentation(
            named: "brew:rg",
            source: .formula(rootFormula: "rg"),
            installRoot: "/opt/rg"
        )

        XCTAssertEqual(model.packageBadge(for: package), .immutable)
    }

    @MainActor
    func testHomebrewInstalledPackageDoesNotShowImmutableBadge() {
        let model = MainWindowModel()
        let package = installedPresentation(
            named: "brew:rg",
            source: .formula(rootFormula: "rg"),
            installRoot: "/opt/homebrew/Cellar/rg"
        )

        XCTAssertNil(model.packageBadge(for: package))
    }

    @MainActor
    func testIsotopePackageShowsHardenedInsteadOfImmutableBadge() {
        let model = MainWindowModel()
        let package = installedPresentation(
            named: "isotope:rg",
            source: .isotope(isotopeName: "rg"),
            installRoot: "/opt/isotopes/rg"
        )

        XCTAssertEqual(model.packageBadge(for: package), .hardened)
        XCTAssertTrue(model.isHardened(package))
    }

    @MainActor
    func testInsecureIsotopePackageDoesNotShowHardenedBadge() {
        let model = MainWindowModel()
        let package = installedPresentation(
            named: "isotope:flyctl",
            source: .isotope(isotopeName: "flyctl"),
            installRoot: "/opt/isotopes/flyctl",
            securityState: PackageSecurityState(
                isotopeName: "flyctl",
                installIsInsecure: true,
                remediationAvailable: true,
                reasons: ["flyctl config file contains a plaintext access token"],
                error: nil
            )
        )

        XCTAssertEqual(model.packageBadge(for: package), .vulnerable)
        XCTAssertFalse(model.isHardened(package))
    }

    @MainActor
    func testGeigerProtocolPackageShowsVulnerableBadgeWithoutSecurityState() {
        let model = MainWindowModel()
        let result = PackageSearchResult(
            name: "brew:supabase-cli",
            source: .formula(rootFormula: "supabase-cli"),
            version: nil,
            description: "Detector flagged local plaintext credential exposure",
            homepage: nil,
            dependencies: [],
            securityState: nil,
            pulseKind: nil
        )
        let package = PackagePresentation(
            item: .available(result),
            detail: result.fallbackDetail,
            freshness: 0,
            presentationID: "geiger:brew:supabase-cli"
        )

        XCTAssertEqual(model.packageBadge(for: package), .vulnerable)
    }

    private func decodePackageDetail(
        packageName: String = "brew:git",
        formula: String = "git",
        installed: Bool = false,
        installRoot: String? = nil,
        securityState: String
    ) throws -> PackageDetail {
        let json = """
        {
          "packageName": "\(packageName)",
          "qualifiedName": "\(packageName)",
          "installRoot": "\(installRoot ?? "/opt/homebrew/Cellar/\(formula)")",
          "installed": \(installed),
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
        }
        """
        return try JSONDecoder().decode(PackageDetail.self, from: Data(json.utf8))
    }

    private func makeSecurityCatalogBundle(combinedJSON: String) throws -> Bundle {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("SecurityCatalogTests-\(UUID().uuidString).bundle")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let infoPlist = """
        <?xml version="1.0" encoding="UTF-8"?>
        <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
        <plist version="1.0">
        <dict>
          <key>CFBundleIdentifier</key>
          <string>com.automic-vault.SecurityCatalogTests</string>
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

    private func installedPresentation(
        named name: String,
        source: PackageSource? = nil,
        installRoot: String? = nil,
        securityState: PackageSecurityState? = nil
    ) -> PackagePresentation {
        PackagePresentation(
            item: .installed(PackageRecord(
                name: name,
                source: source,
                version: "1.0",
                description: nil,
                securityState: securityState,
                installRoot: installRoot
            )),
            detail: nil,
            freshness: 0
        )
    }
}
