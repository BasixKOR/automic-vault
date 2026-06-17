import Foundation
import XCTest
@testable import AutomicVaultApp

final class MainWindowModelTests: XCTestCase {
    func testPackageDisplayTitleSplitsVersionSuffixes() {
        XCTAssertEqual(
            PackageDisplayTitle(displayName: "python@3.13"),
            PackageDisplayTitle(name: "python", versionSuffix: "@3.13")
        )
        XCTAssertEqual(
            PackageDisplayTitle(displayName: "openssl@3"),
            PackageDisplayTitle(name: "openssl", versionSuffix: "@3")
        )
        XCTAssertEqual(
            PackageDisplayTitle(displayName: "@openai/codex"),
            PackageDisplayTitle(name: "@openai/codex")
        )
        XCTAssertEqual(
            PackageDisplayTitle(displayName: "@openai/codex@1.2.3"),
            PackageDisplayTitle(name: "@openai/codex", versionSuffix: "@1.2.3")
        )
        XCTAssertEqual(
            PackageDisplayTitle(displayName: "openssl@stable"),
            PackageDisplayTitle(name: "openssl@stable")
        )
        XCTAssertEqual(
            PackageDisplayTitle(
                displayName: "node",
                latestVersionedBases: ["node"]
            ),
            PackageDisplayTitle(name: "node", versionSuffix: "@latest")
        )
        XCTAssertEqual(
            PackageDisplayTitle(
                displayName: "node@24",
                latestVersionedBases: ["node"]
            ),
            PackageDisplayTitle(name: "node", versionSuffix: "@24")
        )
        XCTAssertEqual(
            PackageDisplayTitle(
                displayName: "nodenv",
                latestVersionedBases: ["node"]
            ),
            PackageDisplayTitle(name: "nodenv")
        )
        XCTAssertEqual(
            PackageDisplayTitle.versionedBase(displayName: "node@24"),
            "node"
        )
    }

    func testSidebarGroupsPutCatalogShortcutsBelowCategories() {
        XCTAssertEqual(
            MainWindowSection.librarySections,
            [.installed, .geigerCounter, .outdated]
        )
        XCTAssertEqual(
            MainWindowSection.categoryShortcutSections,
            [.newUpdated, .allPackages]
        )
    }

    func testWebsiteIndexDecodesBlogPostsFromWebsiteJSON() throws {
        let json = """
        {
          "blog_posts": [
            {
              "title": "The Agentic Toolkit",
              "url": "https://www.automicvault.com/blog/agentic-toolkit/",
              "description": "An installable pack of Homebrew packages.",
              "date_published": "2026-06-02"
            }
          ]
        }
        """

        let index = try JSONDecoder().decode(WebsiteIndex.self, from: Data(json.utf8))

        XCTAssertEqual(index.blogPosts.count, 1)
        XCTAssertEqual(index.blogPosts.first?.title, "The Agentic Toolkit")
        XCTAssertEqual(index.blogPosts.first?.datePublished, "2026-06-02")
    }

    @MainActor
    func testAboutSectionLoadsBlogPostsWithEmptyDossierSelection() async throws {
        let model = MainWindowModel(
            blogPostsFetcher: {
                [
                    WebsiteBlogPost(
                        title: "Older Post",
                        url: "https://www.automicvault.com/blog/older/",
                        description: "Older article.",
                        datePublished: "2026-05-01"
                    ),
                    WebsiteBlogPost(
                        title: "The Agentic Toolkit",
                        url: "https://www.automicvault.com/blog/agentic-toolkit/",
                        description: "An installable pack of Homebrew packages.",
                        datePublished: "2026-06-02"
                    ),
                ]
            }
        )
        defer { model.stop() }

        model.selectedSection = .about
        await waitUntil(model.displayedPackages.count == 2)

        XCTAssertNil(model.count(for: .about))
        XCTAssertEqual(
            model.displayedPackages.map(model.displayName),
            ["The Agentic Toolkit", "Older Post"]
        )

        let post = try XCTUnwrap(model.displayedPackages.first)
        XCTAssertEqual(model.packageDescription(for: post), "An installable pack of Homebrew packages.")
        XCTAssertEqual(model.versionText(for: post), "2026-06-02")

        model.select(post)

        XCTAssertEqual(model.selectedItemID, "blog:https://www.automicvault.com/blog/agentic-toolkit/")
        XCTAssertFalse(model.isLoadingDetail)
        XCTAssertNil(model.selectedDetail)
        XCTAssertEqual(
            model.selectedURL(for: .homepage)?.absoluteString,
            "https://www.automicvault.com/blog/agentic-toolkit/"
        )
        XCTAssertEqual(
            model.selectedURL(for: .repository)?.absoluteString,
            "https://www.automicvault.com/blog/agentic-toolkit/"
        )
        XCTAssertEqual(model.highlightedLinkTab(for: .repository), .homepage)
    }

    func testSidebarAlertCountsDisplayZeroForPersistentSections() {
        XCTAssertTrue(MainWindowSection.outdated.shouldDisplaySidebarCount(0))
        XCTAssertTrue(MainWindowSection.outdated.shouldDisplaySidebarCount(1))
        XCTAssertTrue(MainWindowSection.geigerCounter.shouldDisplaySidebarCount(0))
        XCTAssertTrue(MainWindowSection.geigerCounter.shouldDisplaySidebarCount(1))
        XCTAssertFalse(MainWindowSection.newUpdated.shouldDisplaySidebarCount(0))
    }

    func testSidebarAlertCountsOnlyHighlightPositiveValues() {
        XCTAssertFalse(MainWindowSection.outdated.shouldHighlightSidebarCount(0))
        XCTAssertTrue(MainWindowSection.outdated.shouldHighlightSidebarCount(1))
        XCTAssertFalse(MainWindowSection.geigerCounter.shouldHighlightSidebarCount(0))
        XCTAssertTrue(MainWindowSection.geigerCounter.shouldHighlightSidebarCount(1))
        XCTAssertFalse(MainWindowSection.newUpdated.shouldHighlightSidebarCount(0))
        XCTAssertTrue(MainWindowSection.newUpdated.shouldHighlightSidebarCount(1))
    }

    @MainActor
    func testPersistentSidebarCountsShowZeroWhenThereAreNoAlertsOrUpdates() {
        let model = MainWindowModel()
        defer { model.stop() }

        XCTAssertEqual(model.count(for: .geigerCounter), 0)
        XCTAssertEqual(model.count(for: .outdated), 0)
    }

    func testCategorySectionsAreAlphabetizedByDisplayedTitle() {
        let sections = MainWindowSection.categorySections
        XCTAssertFalse(sections.contains(.other))
        XCTAssertTrue(sections.contains(.toys))
        XCTAssertEqual(MainWindowSection.toys.categoryIdentifier, "toys")

        let titles = sections.map(\.title)
        let sortedTitles = titles.sorted {
            $0.localizedStandardCompare($1) == .orderedAscending
        }

        XCTAssertEqual(titles, sortedTitles)
    }

    @MainActor
    func testStartPreloadsCategoryCountsWhileInstalledSectionIsSelected() async {
        let requests = CategoryPageRequestRecorder()
        let categoryCounts = [
            "developer-tools": 2,
            "security": 1,
        ]
        let model = MainWindowModel(
            cliToolsRecommendationProvider: { nil },
            installedPackagesFetcher: { [] },
            outdatedPackagesFetcher: { [] },
            availablePackagesFetcher: { offset, _, category, sortOrder in
                requests.append(offset: offset, category: category, sortOrder: sortOrder)
                return PackageSearchPage(
                    packages: [],
                    totalCount: 3,
                    nextOffset: nil,
                    categoryCounts: categoryCounts
                )
            },
            pulsePackagesFetcher: { _, _ in
                PackageSearchPage(packages: [], totalCount: 0, nextOffset: nil)
            },
            geigerPackagesFetcher: { _, _ in
                PackageSearchPage(packages: [], totalCount: 0, nextOffset: nil)
            }
        )
        defer { model.stop() }

        XCTAssertEqual(model.selectedSection, .installed)

        model.start()
        await waitUntil(model.count(for: .developerTools) == 2)

        XCTAssertEqual(model.selectedSection, .installed)
        XCTAssertEqual(model.count(for: .security), 1)
        XCTAssertEqual(
            requests.values,
            [.init(offset: 0, category: nil, sortOrder: .rank)]
        )
    }

    @MainActor
    func testStartPreloadsCategoryCountsAfterInstalledReloadCompletes() async {
        let order = StartupPreloadOrderRecorder()
        let model = MainWindowModel(
            cliToolsRecommendationProvider: { nil },
            installedPackagesFetcher: {
                Thread.sleep(forTimeInterval: 0.05)
                order.markInstalledReloadFinished()
                return []
            },
            outdatedPackagesFetcher: { [] },
            availablePackagesFetcher: { _, _, _, _ in
                order.recordAvailablePackagesRequest()
                return PackageSearchPage(
                    packages: [],
                    totalCount: 2,
                    nextOffset: nil,
                    categoryCounts: [
                        "developer-tools": 2,
                    ]
                )
            },
            pulsePackagesFetcher: { _, _ in
                PackageSearchPage(packages: [], totalCount: 0, nextOffset: nil)
            },
            geigerPackagesFetcher: { _, _ in
                PackageSearchPage(packages: [], totalCount: 0, nextOffset: nil)
            }
        )
        defer { model.stop() }

        model.start()
        await waitUntil(model.count(for: .developerTools) == 2)

        XCTAssertFalse(order.didRequestAvailablePackagesBeforeInstalledReloadFinished)
    }

    @MainActor
    func testStartFocusesSecurityAlertsWhenInstalledHazardsExist() async {
        let state = securityState(
            isotopeName: "gh",
            reason: "GitHub token is stored in plaintext"
        )
        let record = PackageRecord(
            name: "brew:gh",
            source: .formula(rootFormula: "gh"),
            version: "2.49.0",
            description: "GitHub command line tool",
            securityState: state
        )
        let model = MainWindowModel(
            cliToolsRecommendationProvider: { nil },
            installedPackagesFetcher: { [record] },
            outdatedPackagesFetcher: { [] },
            geigerPackagesFetcher: { _, _ in
                PackageSearchPage(packages: [], totalCount: 0, nextOffset: nil)
            }
        )
        defer { model.stop() }

        model.start()
        await waitUntil(model.selectedSection == .geigerCounter)

        XCTAssertEqual(model.activeSidebarSection, .geigerCounter)
        XCTAssertEqual(model.count(for: .geigerCounter), 1)
        XCTAssertEqual(model.displayedPackages.map(\.selectionID), ["brew:gh"])
    }

    @MainActor
    func testStartFocusesSecurityAlertsWhenDetectorHazardsExist() async {
        let detectorResult = PackageSearchResult(
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
        let model = MainWindowModel(
            cliToolsRecommendationProvider: { nil },
            installedPackagesFetcher: { [] },
            outdatedPackagesFetcher: { [] },
            geigerPackagesFetcher: { _, _ in
                PackageSearchPage(packages: [detectorResult], totalCount: 1, nextOffset: nil)
            }
        )
        defer { model.stop() }

        model.start()
        await waitUntil(
            model.selectedSection == .geigerCounter
                && model.displayedPackages.map(\.selectionID) == ["gone:supabase-cli"]
        )

        XCTAssertEqual(model.activeSidebarSection, .geigerCounter)
        XCTAssertEqual(model.count(for: .geigerCounter), 1)
    }

    @MainActor
    func testStartupSecurityAlertFocusDoesNotOverrideUserSidebarSelection() async {
        let state = securityState(
            isotopeName: "gh",
            reason: "GitHub token is stored in plaintext"
        )
        let record = PackageRecord(
            name: "brew:gh",
            source: .formula(rootFormula: "gh"),
            version: "2.49.0",
            description: "GitHub command line tool",
            securityState: state
        )
        let model = MainWindowModel(
            cliToolsRecommendationProvider: { nil },
            installedPackagesFetcher: {
                Thread.sleep(forTimeInterval: 0.05)
                return [record]
            },
            outdatedPackagesFetcher: { [] },
            geigerPackagesFetcher: { _, _ in
                PackageSearchPage(packages: [], totalCount: 0, nextOffset: nil)
            }
        )
        defer { model.stop() }

        model.start()
        model.selectSection(.outdated)
        await waitUntil(model.count(for: .geigerCounter) == 1)

        XCTAssertEqual(model.selectedSection, .outdated)
        XCTAssertEqual(model.activeSidebarSection, .outdated)
    }

    @MainActor
    func testAllPackagesLoadsNextPageWhenScrolledNearEnd() async throws {
        let requests = PageRequestRecorder()
        let model = MainWindowModel(
            availablePackagesFetcher: { offset, _, _, _ in
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
    func testSecurityRecommendationsLoadsRecommendationPackages() async throws {
        let requests = PageRequestRecorder()
        let model = MainWindowModel(
            securityRecommendationPackagesFetcher: { offset, _ in
                requests.append(offset)
                return PackageSearchPage(
                    packages: [
                        Self.packageSearchResult(
                            name: "brew:awscli",
                            description: "Plain Text AWS Credentials"
                        )
                    ],
                    totalCount: 1,
                    nextOffset: nil
                )
            }
        )
        defer { model.stop() }

        model.selectedSection = .securityRecommendations
        await waitUntil(
            model.displayedPackages.map(\.selectionID) == ["security-recommendation:brew:awscli"]
        )

        XCTAssertEqual(requests.values, [0])
        XCTAssertEqual(model.count(for: .securityRecommendations), 1)
        XCTAssertEqual(
            model.displayedPackages.first?.detail?.homebrewInfo?.description,
            "Plain Text AWS Credentials"
        )
        XCTAssertEqual(
            model.displayedPackages.first.map(model.securityRecommendationSeverityLevel),
            3
        )
    }

    @MainActor
    func testAutomicVaultInstallBadgeShowsInSearchAndCategoryListings() {
        let model = MainWindowModel()
        defer { model.stop() }
        let result = Self.packageSearchResult(
            name: "brew:node@24",
            category: "language-runtime",
            installsHardened: true
        )
        let package = PackagePresentation(
            item: .available(result),
            detail: result.fallbackDetail,
            freshness: 0
        )

        model.selectedSection = .languageRuntime
        XCTAssertEqual(model.packageListBadges(for: package), [.automicVault])

        model.searchText = "node"
        XCTAssertEqual(model.packageListBadges(for: package), [.automicVault])

        model.searchText = ""
        model.selectedSection = .newUpdated
        XCTAssertEqual(model.packageListBadges(for: package), [])
    }

    @MainActor
    func testCategorySectionUsesDatabaseCategoryMetadata() async throws {
        let model = MainWindowModel(
            availablePackagesFetcher: { _, _, _, _ in
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
                            name: "brew:toybox",
                            category: "toys"
                        ),
                        Self.packageSearchResult(
                            name: "cask:codex",
                            category: nil
                        ),
                    ],
                    totalCount: 5,
                    nextOffset: nil,
                    categoryCounts: [
                        "developer-tools": 2,
                        "security": 1,
                        "toys": 1,
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
        XCTAssertEqual(model.count(for: .toys), 1)
        XCTAssertEqual(model.count(for: .other), 1)
        XCTAssertEqual(
            model.displayedPackages.map(\.selectionID),
            ["brew:uv", "brew:gh"]
        )

        model.selectedSection = .security
        await waitUntil(model.displayedPackages.map(\.selectionID) == ["brew:sops"])

        XCTAssertEqual(model.displayedPackages.map(\.selectionID), ["brew:sops"])

        model.selectedSection = .toys
        await waitUntil(model.displayedPackages.map(\.selectionID) == ["brew:toybox"])

        XCTAssertEqual(model.displayedPackages.map(\.selectionID), ["brew:toybox"])
    }

    @MainActor
    func testCategorySectionRequestsCategoryFilteredCatalogPage() async throws {
        let requests = CategoryPageRequestRecorder()
        let categoryCounts = [
            "developer-tools": 2,
            "productivity": 1,
        ]
        let model = MainWindowModel(
            availablePackagesFetcher: { offset, _, category, sortOrder in
                requests.append(offset: offset, category: category, sortOrder: sortOrder)
                switch category {
                case nil:
                    return PackageSearchPage(
                        packages: [
                            Self.packageSearchResult(
                                name: "brew:a2ps",
                                category: "productivity"
                            ),
                        ],
                        totalCount: 3,
                        nextOffset: 1,
                        categoryCounts: categoryCounts
                    )
                case "developer-tools":
                    return PackageSearchPage(
                        packages: [
                            Self.packageSearchResult(
                                name: "brew:uv",
                                homepage: "https://docs.astral.sh/uv/",
                                category: "developer-tools"
                            ),
                            Self.packageSearchResult(
                                name: "brew:gh",
                                category: "developer-tools"
                            ),
                        ],
                        totalCount: 2,
                        nextOffset: nil,
                        categoryCounts: categoryCounts
                    )
                default:
                    return PackageSearchPage(
                        packages: [],
                        totalCount: 0,
                        nextOffset: nil,
                        categoryCounts: categoryCounts
                    )
                }
            }
        )
        defer { model.stop() }

        model.selectedSection = .allPackages
        await waitUntil(model.displayedPackages.count == 1)

        model.selectedSection = .developerTools
        await waitUntil(model.displayedPackages.count == 2)

        XCTAssertEqual(
            requests.values.map(\.category),
            [nil, "developer-tools"]
        )
        XCTAssertEqual(
            requests.values.map(\.sortOrder),
            [.rank, .rank]
        )
        XCTAssertEqual(
            model.displayedPackages.map(\.selectionID),
            ["brew:uv", "brew:gh"]
        )

        let package = try XCTUnwrap(model.displayedPackages.first)
        model.select(package)

        XCTAssertEqual(model.selectedPackage?.selectionID, "brew:uv")
        XCTAssertEqual(model.selectedDetail?.packageName, "brew:uv")
        XCTAssertEqual(
            model.selectedURL(for: .homepage)?.absoluteString,
            "https://docs.astral.sh/uv/"
        )
    }

    @MainActor
    func testCategorySortOrderCanSwitchFromRankToAlphabetical() async throws {
        let requests = CategoryPageRequestRecorder()
        let model = MainWindowModel(
            availablePackagesFetcher: { offset, _, category, sortOrder in
                requests.append(offset: offset, category: category, sortOrder: sortOrder)
                guard category == "developer-tools" else {
                    return PackageSearchPage(packages: [], totalCount: 0, nextOffset: nil)
                }
                switch sortOrder {
                case .rank:
                    return PackageSearchPage(
                        packages: [
                            Self.packageSearchResult(
                                name: "brew:zulu",
                                category: "developer-tools",
                                rank: 1
                            ),
                            Self.packageSearchResult(
                                name: "brew:alpha",
                                category: "developer-tools",
                                rank: 2
                            ),
                        ],
                        totalCount: 2,
                        nextOffset: nil
                    )
                case .alphabetical:
                    return PackageSearchPage(
                        packages: [
                            Self.packageSearchResult(
                                name: "brew:alpha",
                                category: "developer-tools",
                                rank: 2
                            ),
                            Self.packageSearchResult(
                                name: "brew:zulu",
                                category: "developer-tools",
                                rank: 1
                            ),
                        ],
                        totalCount: 2,
                        nextOffset: nil
                    )
                }
            }
        )
        defer { model.stop() }

        XCTAssertEqual(model.categoryPackageSortOrder, .rank)
        XCTAssertEqual(model.categorySortButtonTitle, "Sort: Popularity")

        model.selectedSection = .developerTools
        await waitUntil(model.displayedPackages.map(\.selectionID) == ["brew:zulu", "brew:alpha"])

        model.selectCategorySortOrder(.alphabetical)
        await waitUntil(model.displayedPackages.map(\.selectionID) == ["brew:alpha", "brew:zulu"])

        XCTAssertEqual(
            requests.values,
            [
                .init(offset: 0, category: "developer-tools", sortOrder: .rank),
                .init(offset: 0, category: "developer-tools", sortOrder: .alphabetical),
            ]
        )
        XCTAssertEqual(model.categorySortButtonTitle, "Sort: A-Z")
    }

    @MainActor
    func testNewUpdatedSectionShowsUpdatedPackagesWithoutCountingThemAsNew() async throws {
        let model = MainWindowModel(
            pulsePackagesFetcher: { _, _ in
                PackageSearchPage(
                    packages: [
                        Self.packageSearchResult(
                            name: "npm:tsx",
                            homepage: "https://tsx.is",
                            category: nil,
                            pulseKind: "updated"
                        ),
                    ],
                    totalCount: 1,
                    nextOffset: nil
                )
            }
        )
        defer { model.stop() }

        model.selectedSection = .newUpdated
        await waitUntil(model.displayedPackages.count == 1)
        let package = try XCTUnwrap(model.displayedPackages.first)

        XCTAssertNil(model.count(for: .newUpdated))
        XCTAssertEqual(model.displayedPackages.map(\.selectionID), ["pulse:npm:tsx"])
        XCTAssertEqual(model.packageListBadges(for: package), [])
    }

    @MainActor
    func testNewUpdatedSidebarDoesNotCountUpdatedPackagesBeforeFirstClick() async throws {
        let suiteName = "MainWindowModelTests.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }

        let model = MainWindowModel(
            pulsePackagesFetcher: { _, _ in
                PackageSearchPage(
                    packages: [
                        Self.packageSearchResult(
                            name: "brew:updated",
                            pulseKind: "updated",
                            lastUpdatedAt: "2026-05-28T12:30:00Z"
                        ),
                    ],
                    totalCount: 1,
                    nextOffset: nil
                )
            },
            userDefaults: defaults
        )
        defer { model.stop() }

        model.selectedSection = .newUpdated
        await waitUntil(model.displayedPackages.count == 1)

        XCTAssertNil(model.count(for: .newUpdated))
    }

    @MainActor
    func testNewUpdatedSidebarCountUsesPulsePackagesSinceLastClick() async throws {
        let suiteName = "MainWindowModelTests.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }
        defaults.set(
            try XCTUnwrap(Self.date("2026-05-28T12:00:00Z")),
            forKey: MainWindowModel.newUpdatedLastClickedAtDefaultsKey
        )

        let model = MainWindowModel(
            pulsePackagesFetcher: { _, _ in
                PackageSearchPage(
                    packages: [
                        Self.packageSearchResult(
                            name: "brew:newer",
                            pulseKind: "new",
                            lastUpdatedAt: "2026-05-28T12:00:01Z"
                        ),
                        Self.packageSearchResult(
                            name: "brew:older",
                            pulseKind: "new",
                            lastUpdatedAt: "2026-05-28T11:59:59Z"
                        ),
                        Self.packageSearchResult(
                            name: "brew:undated",
                            pulseKind: "new"
                        ),
                        Self.packageSearchResult(
                            name: "brew:updated",
                            pulseKind: "updated",
                            lastUpdatedAt: "2026-05-28T12:30:00Z"
                        ),
                    ],
                    totalCount: 4,
                    nextOffset: nil
                )
            },
            userDefaults: defaults
        )
        defer { model.stop() }

        model.selectedSection = .newUpdated
        await waitUntil(model.displayedPackages.count == 4)

        XCTAssertEqual(model.count(for: .newUpdated), 2)
        let badgesBySelectionID = Dictionary(
            uniqueKeysWithValues: model.displayedPackages.map {
                ($0.selectionID, model.packageListBadges(for: $0))
            }
        )
        XCTAssertEqual(try XCTUnwrap(badgesBySelectionID["pulse:brew:newer"]), [.new])
        XCTAssertEqual(try XCTUnwrap(badgesBySelectionID["pulse:brew:older"]), [.new])
        XCTAssertEqual(try XCTUnwrap(badgesBySelectionID["pulse:brew:undated"]), [.new])
        XCTAssertEqual(try XCTUnwrap(badgesBySelectionID["pulse:brew:updated"]), [.new])
    }

    @MainActor
    func testClickingNewUpdatedSidebarDefersResetDisplayUntilLeavingSection() async throws {
        let suiteName = "MainWindowModelTests.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }

        let model = MainWindowModel(
            pulsePackagesFetcher: { _, _ in
                PackageSearchPage(
                    packages: [
                        Self.packageSearchResult(
                            name: "brew:example",
                            pulseKind: "new",
                            lastUpdatedAt: "2001-01-01T00:00:00Z"
                        ),
                    ],
                    totalCount: 1,
                    nextOffset: nil
                )
            },
            userDefaults: defaults
        )
        defer { model.stop() }

        model.selectedSection = .newUpdated
        await waitUntil(model.displayedPackages.count == 1)
        XCTAssertEqual(model.count(for: .newUpdated), 1)

        model.selectSection(.newUpdated)

        XCTAssertEqual(model.count(for: .newUpdated), 1)
        XCTAssertNotNil(
            defaults.object(forKey: MainWindowModel.newUpdatedLastClickedAtDefaultsKey) as? Date
        )

        model.selectSection(.installed)

        XCTAssertNil(model.count(for: .newUpdated))
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
    func testOutdatedPackageReleaseNotesHighlightRepositoryTabWhenHomepageIsNotGitHub() throws {
        let model = MainWindowModel()
        defer { model.stop() }
        let detail = PackageRecord(
            name: "brew:uv",
            source: .formula(rootFormula: "uv"),
            version: "0.8.0",
            description: "Python package manager",
            homepage: "https://docs.astral.sh/uv/",
            repository: "https://github.com/astral-sh/uv",
            latestVersion: "0.9.0",
            securityState: nil
        ).fallbackDetail

        XCTAssertEqual(
            model.linkURL(for: .homepage, detail: detail)?.absoluteString,
            "https://github.com/astral-sh/uv/releases/latest"
        )
        XCTAssertEqual(
            model.linkURL(for: .repository, detail: detail)?.absoluteString,
            "https://github.com/astral-sh/uv"
        )
        XCTAssertEqual(model.highlightedLinkTab(for: .homepage, detail: detail), .repository)
        XCTAssertEqual(model.highlightedLinkTab(for: .repository, detail: detail), .repository)
    }

    @MainActor
    func testPackageDetailDecodesRepoAliasForOutdatedReleaseNotes() throws {
        let model = MainWindowModel()
        defer { model.stop() }
        let json = """
        {
          "packageName": "brew:uv",
          "qualifiedName": "brew:uv",
          "installRoot": "/opt/homebrew/Cellar/uv",
          "installed": true,
          "source": {"kind": "formula", "rootFormula": "uv"},
          "sourceError": null,
          "aliases": [],
          "aliasesError": null,
          "installedVersion": "0.8.0",
          "latestVersion": "0.9.0",
          "latestVersionError": null,
          "executablePaths": [],
          "executablePathsError": null,
          "popularity": null,
          "lastUpdatedAt": null,
          "homebrewInfo": {
            "formula": "uv",
            "description": "Python package manager",
            "homepage": "https://docs.astral.sh/uv/",
            "repo": "astral-sh/uv",
            "license": null,
            "dependencies": []
          },
          "homebrewInfoError": null,
          "npmHomepage": null,
          "npmPackageInfoError": null,
          "securityState": null,
          "installPackageNames": null,
          "versionOptions": []
        }
        """
        let detail = try JSONDecoder().decode(PackageDetail.self, from: Data(json.utf8))

        XCTAssertEqual(
            model.linkURL(for: .homepage, detail: detail)?.absoluteString,
            "https://github.com/astral-sh/uv/releases/latest"
        )
        XCTAssertEqual(
            model.linkURL(for: .repository, detail: detail)?.absoluteString,
            "https://github.com/astral-sh/uv"
        )
        XCTAssertEqual(model.highlightedLinkTab(for: .homepage, detail: detail), .repository)
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

    @MainActor
    func testSearchDossierShowsVersionForAvailablePackage() {
        let model = MainWindowModel()
        defer { model.stop() }
        model.searchText = "openclaw"
        let result = PackageSearchResult(
            name: "npm:openclaw",
            source: .npm(packageName: "openclaw"),
            version: nil,
            description: "Multi-channel AI gateway",
            homepage: nil,
            dependencies: [],
            securityState: nil,
            pulseKind: nil
        )
        let detail = PackageRecord(
            name: "npm:openclaw",
            source: .npm(packageName: "openclaw"),
            version: "",
            description: "Multi-channel AI gateway",
            latestVersion: "2026.5.22",
            securityState: nil
        ).fallbackDetail
        let presentation = PackagePresentation(
            item: .available(result),
            detail: result.fallbackDetail,
            freshness: 0
        )

        XCTAssertEqual(model.versionText(for: presentation), "NPM")
        XCTAssertEqual(
            model.dossierVersionText(
                for: presentation,
                detail: detail
            ),
            "2026.5.22"
        )
    }

    @MainActor
    func testSearchDossierKeepsInstalledPackageVersion() throws {
        let model = MainWindowModel()
        defer { model.stop() }
        model.searchText = "rg"
        let package = installedPresentation(version: "1.0", latestVersion: "2.0")

        XCTAssertEqual(
            model.dossierVersionText(
                for: package,
                detail: try XCTUnwrap(package.detail)
            ),
            "1.0"
        )
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

    func testBlankLatestVersionsAreNotFlaggedAsOutdated() {
        let snapshot = NucleusStatusSnapshot(
            installedCount: 10,
            hazardousPackageCount: 1,
            outdatedPackages: [
                OutdatedPackageRecord(
                    name: "codex",
                    currentVersion: "0.139.0",
                    latestVersion: ""
                ),
                OutdatedPackageRecord(
                    name: "isotope:uv",
                    currentVersion: "0.11.19",
                    latestVersion: "0.11.21"
                ),
            ],
            refreshedAt: Date(),
            lastError: nil
        )

        XCTAssertEqual(snapshot.flaggedOutdatedPackageCount, 1)
        XCTAssertEqual(snapshot.appBadgeCount, 2)
        XCTAssertEqual(snapshot.flaggedOutdatedPackages.map(\.name), ["isotope:uv"])
        XCTAssertNil(snapshot.outdatedPackagesByName["codex"])
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
    func testSearchKeepsVersionedAliasAfterCanonicalDetailLoads() throws {
        let node = searchPresentation(
            name: "node",
            formula: "node",
            description: "JavaScript runtime"
        )
        let node26 = searchPresentation(
            name: "node@26",
            formula: "node@26",
            description: "JavaScript runtime"
        )
        let loadedNode26Detail = try XCTUnwrap(node26.detail).withPackageIdentity(
            packageName: "node",
            installPackageNames: ["node"]
        )
        let loadedNode26 = PackagePresentation(
            item: node26.item,
            detail: loadedNode26Detail,
            freshness: node26.freshness,
            presentationID: node26.presentationID
        )

        let merged = MainWindowModel.mergedSearchPackages(
            installed: [],
            daemon: [node, loadedNode26]
        )

        XCTAssertEqual(
            merged.map(\.selectionID),
            ["search:node", "search:node@26"]
        )
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
        description: String? = nil,
        homepage: String? = nil,
        category: String? = nil,
        installsHardened: Bool = false,
        pulseKind: String? = nil,
        rank: UInt32? = nil,
        lastUpdatedAt: String? = nil
    ) -> PackageSearchResult {
        PackageSearchResult(
            name: name,
            source: .formula(rootFormula: name.replacingOccurrences(of: "brew:", with: "")),
            version: "1.0",
            description: description ?? "\(name) package",
            homepage: homepage,
            category: category,
            dependencies: [],
            installsHardened: installsHardened,
            rank: rank,
            lastUpdatedAt: lastUpdatedAt,
            securityState: nil,
            pulseKind: pulseKind
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

private final class CategoryPageRequestRecorder: @unchecked Sendable {
    struct Request: Equatable {
        let offset: Int
        let category: String?
        let sortOrder: CategoryPackageSortOrder
    }

    private let lock = NSLock()
    private var requests: [Request] = []

    var values: [Request] {
        lock.withLock { requests }
    }

    func append(
        offset: Int,
        category: String?,
        sortOrder: CategoryPackageSortOrder
    ) {
        lock.withLock {
            requests.append(
                Request(offset: offset, category: category, sortOrder: sortOrder)
            )
        }
    }
}

private final class StartupPreloadOrderRecorder: @unchecked Sendable {
    private let lock = NSLock()
    private var installedReloadFinished = false
    private var requestedAvailablePackagesBeforeInstalledReloadFinished = false

    var didRequestAvailablePackagesBeforeInstalledReloadFinished: Bool {
        lock.withLock { requestedAvailablePackagesBeforeInstalledReloadFinished }
    }

    func markInstalledReloadFinished() {
        lock.withLock {
            installedReloadFinished = true
        }
    }

    func recordAvailablePackagesRequest() {
        lock.withLock {
            if !installedReloadFinished {
                requestedAvailablePackagesBeforeInstalledReloadFinished = true
            }
        }
    }
}
