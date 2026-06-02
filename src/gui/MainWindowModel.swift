import AppKit
import Combine
import Foundation

enum MainWindowSection: String, CaseIterable, Identifiable {
    case installed
    case securityRecommendations
    case geigerCounter
    case newUpdated
    case outdated
    case allPackages
    case developerTools
    case cloudInfrastructure
    case networking
    case system
    case security
    case data
    case languageRuntime
    case media
    case productivity
    case science
    case games
    case toys
    case other
    case settings
    case about

    var id: String { rawValue }

    static let librarySections: [MainWindowSection] = [
        .installed,
        .securityRecommendations,
        .geigerCounter,
        .outdated
    ]

    private static let unsortedCategorySections: [MainWindowSection] = [
        .developerTools,
        .cloudInfrastructure,
        .networking,
        .system,
        .security,
        .data,
        .languageRuntime,
        .media,
        .productivity,
        .science,
        .games,
        .toys,
        .other
    ]

    static var categorySections: [MainWindowSection] {
        unsortedCategorySections.sorted { left, right in
            if left == .other {
                return false
            }
            if right == .other {
                return true
            }
            let comparison = left.title.localizedStandardCompare(right.title)
            if comparison == .orderedSame {
                return left.rawValue < right.rawValue
            }
            return comparison == .orderedAscending
        }
    }

    static let categoryShortcutSections: [MainWindowSection] = [
        .newUpdated,
        .allPackages
    ]

    static let utilitySections: [MainWindowSection] = [
        .settings,
        .about
    ]

    var title: String {
        switch self {
        case .installed:
            return L10n.string("Installed")
        case .securityRecommendations:
            return L10n.string("Security Recommendations")
        case .geigerCounter:
            return L10n.string("Security Alerts")
        case .newUpdated:
            return L10n.string("New / Updated")
        case .outdated:
            return L10n.string("Outdated")
        case .allPackages:
            return L10n.string("All Packages")
        case .developerTools:
            return L10n.string("Developer Tools")
        case .cloudInfrastructure:
            return L10n.string("Cloud Infrastructure")
        case .networking:
            return L10n.string("Networking")
        case .system:
            return L10n.string("System")
        case .security:
            return L10n.string("Security")
        case .data:
            return L10n.string("Data")
        case .languageRuntime:
            return L10n.string("Language Runtime")
        case .media:
            return L10n.string("Media")
        case .productivity:
            return L10n.string("Productivity")
        case .science:
            return L10n.string("Science")
        case .games:
            return L10n.string("Games")
        case .toys:
            return L10n.string("Toys")
        case .other:
            return L10n.string("Other")
        case .settings:
            return L10n.string("Settings")
        case .about:
            return L10n.string("About")
        }
    }

    var systemImage: String {
        switch self {
        case .installed:
            return "shippingbox"
        case .securityRecommendations:
            return "lock.shield"
        case .geigerCounter:
            return "exclamationmark.shield"
        case .newUpdated:
            return "sparkles"
        case .outdated:
            return "clock"
        case .allPackages:
            return "cube"
        case .developerTools:
            return "chevron.left.forwardslash.chevron.right"
        case .cloudInfrastructure:
            return "cloud"
        case .networking:
            return "network"
        case .system:
            return "gearshape"
        case .security:
            return "shield"
        case .data:
            return "chart.bar.doc.horizontal"
        case .languageRuntime:
            return "curlybraces"
        case .media:
            return "play.rectangle"
        case .productivity:
            return "checklist"
        case .science:
            return "atom"
        case .games:
            return "gamecontroller"
        case .toys:
            return "puzzlepiece"
        case .other:
            return "ellipsis"
        case .settings:
            return "gear"
        case .about:
            return "info.circle"
        }
    }

    var categoryIdentifier: String? {
        switch self {
        case .developerTools:
            return "developer-tools"
        case .cloudInfrastructure:
            return "cloud-infrastructure"
        case .networking:
            return "networking"
        case .system:
            return "system"
        case .security:
            return "security"
        case .data:
            return "data"
        case .languageRuntime:
            return "language-runtime"
        case .media:
            return "media"
        case .productivity:
            return "productivity"
        case .science:
            return "science"
        case .games:
            return "games"
        case .toys:
            return "toys"
        case .other:
            return "other"
        case .installed, .securityRecommendations, .geigerCounter, .newUpdated, .outdated,
             .allPackages, .settings, .about:
            return nil
        }
    }
}

enum MainWindowLinkTab: String, CaseIterable, Identifiable {
    case homepage
    case repository
    case documentation

    var id: String { rawValue }

    var title: String {
        switch self {
        case .homepage:
            return L10n.string("Homepage")
        case .repository:
            return L10n.string("Repository")
        case .documentation:
            return L10n.string("Documentation")
        }
    }
}

enum MainWindowPackageBadge: Hashable {
    case new
    case vulnerable
    case hardened
    case immutable
    case outdated
}

enum PackageOperationKind: String, CaseIterable, Identifiable {
    case install
    case update
    case uninstall
    case harden

    var id: String { rawValue }

    var title: String {
        switch self {
        case .install:
            return L10n.string("Install")
        case .update:
            return L10n.string("Update")
        case .uninstall:
            return L10n.string("Uninstall")
        case .harden:
            return L10n.string("Harden")
        }
    }

    var progressTitle: String {
        switch self {
        case .install:
            return L10n.string("Installing")
        case .update:
            return L10n.string("Updating")
        case .uninstall:
            return L10n.string("Uninstalling")
        case .harden:
            return L10n.string("Hardening")
        }
    }

    var progressSheetTitle: String {
        switch self {
        case .install:
            return L10n.string("Install Package")
        case .update:
            return L10n.string("Update Package")
        case .uninstall:
            return L10n.string("Uninstall Package")
        case .harden:
            return L10n.string("Harden Package")
        }
    }

    var successOperationTitle: String {
        switch self {
        case .install:
            return L10n.string("Install Complete")
        case .update:
            return L10n.string("Update Complete")
        case .uninstall:
            return L10n.string("Uninstall Complete")
        case .harden:
            return L10n.string("Hardening Complete")
        }
    }

    var failureOperationTitle: String {
        switch self {
        case .install:
            return L10n.string("Install Halted")
        case .update:
            return L10n.string("Update Halted")
        case .uninstall:
            return L10n.string("Uninstall Halted")
        case .harden:
            return L10n.string("Hardening Halted")
        }
    }

}

struct PackageOperationRequest: Equatable {
    let id: Int
    let kind: PackageOperationKind
    let packageNames: [String]
    let displayName: String
    let isAutomicVaultCLT: Bool
    let isXcodeCLT: Bool
    let migrationIsotopeName: String?
}

@MainActor
final class MainWindowModel: ObservableObject {
    @Published var selectedSection: MainWindowSection = .installed {
        didSet {
            if selectedSection != .newUpdated {
                newUpdatedSelectionDisplayCount = nil
            }
            if selectedSection != oldValue {
                clearSelectedPackage()
            }
            ensureSelectedSectionLoaded()
            updateSelectedSectionLoadingState()
        }
    }
    @Published var searchText = "" {
        didSet {
            scheduleSearch()
        }
    }
    @Published private(set) var categoryPackageSortOrder: CategoryPackageSortOrder = .rank
    @Published private(set) var packages: [PackagePresentation] = []
    @Published private(set) var securityRecommendationPackages: [PackagePresentation] = []
    @Published private(set) var geigerPackages: [PackagePresentation] = []
    @Published private(set) var catalogPackages: [PackagePresentation] = []
    @Published private(set) var pulsePackages: [PackagePresentation] = []
    @Published private(set) var searchResults: [PackagePresentation] = []
    @Published private(set) var snapshot = NucleusStatusSnapshot.empty
    @Published private(set) var selectedItemID: String?
    @Published private(set) var isReloading = false
    @Published private(set) var isLoadingSectionPage = false
    @Published private(set) var isSearching = false
    @Published private(set) var isLoadingDetail = false
    @Published private(set) var statusMessage: String?
    @Published private(set) var lastErrorMessage: String?
    @Published private(set) var searchFocusRequestID = 0
    @Published private(set) var searchDeactivationRequestID = 0
    @Published private(set) var updateAllRequestID = 0
    @Published private(set) var isUpdatingAll = false
    @Published private(set) var packageOperationRequest: PackageOperationRequest?
    @Published private(set) var activePackageOperation: PackageOperationRequest?
    @Published private(set) var automicVaultCLTRecommendation: PackageRecommendation?
    private(set) var newUpdatedLastClickedAt: Date?

    nonisolated private static let pageSize = 96
    nonisolated private static let paginationPrefetchThreshold = 12
    nonisolated static let newUpdatedLastClickedAtDefaultsKey =
        "MainWindowModel.newUpdatedLastClickedAt"
    private let statusStore = NucleusStatusStore()
    private let userDefaults: UserDefaults
    private var newUpdatedSelectionDisplayCount: Int?
    private var snapshotObserver: NSObjectProtocol?
    private var reloadRequestID = 0
    private var searchRequestID = 0
    private var sectionPageRequestIDs: [SectionPageKind: Int] = [:]
    private var detailRequestID = 0
    private var packageOperationRequestID = 0
    private var detailsByPackageName: [String: PackageDetail] = [:]
    private var securityRecommendationTotalCount: Int?
    private var geigerTotalCount: Int?
    private var catalogTotalCount: Int?
    private var catalogCategoryCounts: [String: Int] = [:]
    private var categoryPackagesByPageKey: [CategoryCatalogPageKey: [PackagePresentation]] = [:]
    private var categoryTotalCountsByPageKey: [CategoryCatalogPageKey: Int] = [:]
    private var pulseTotalCount: Int?
    private var searchTotalCount = 0
    private var sectionPageNextOffsets: [SectionPageKind: Int] = [:]
    private var searchNextOffset: Int?
    private var transientStatusTask: Task<Void, Never>?
    private var searchTask: Task<Void, Never>?
    private var sectionPageTasks: [SectionPageKind: Task<Void, Never>] = [:]
    private var loadingSectionKinds = Set<SectionPageKind>()
    private var staleSectionKinds = Set<SectionPageKind>()
    private var pendingHardeningSelection: PackageHardeningContext?
    private let cliToolsRecommendationProvider: () -> PackageRecommendation?
    private let securityCatalog: SecurityCatalog
    private let availablePackagesFetcher: (
        Int,
        Int,
        String?,
        CategoryPackageSortOrder
    ) throws -> PackageSearchPage
    private let pulsePackagesFetcher: (Int, Int) throws -> PackageSearchPage
    private let securityRecommendationPackagesFetcher: (Int, Int) throws -> PackageSearchPage
    private let geigerPackagesFetcher: (Int, Int) throws -> PackageSearchPage
    private let searchPackagesFetcher: (String, Int, Int) throws -> PackageSearchPage

    init(
        cliToolsRecommendationProvider: @escaping () -> PackageRecommendation? = {
            NucleusBridge().cliToolsRecommendation()
        },
        initialAutomicVaultCLTRecommendation: PackageRecommendation? = nil,
        securityCatalog: SecurityCatalog = .shared,
        availablePackagesFetcher: @escaping (
            Int,
            Int,
            String?,
            CategoryPackageSortOrder
        ) throws -> PackageSearchPage = {
            offset,
            limit,
            category,
            sortOrder in
            try MainWindowModel.fetchAvailablePackages(
                offset: offset,
                limit: limit,
                category: category,
                sortOrder: sortOrder
            )
        },
        pulsePackagesFetcher: @escaping (Int, Int) throws -> PackageSearchPage = {
            offset,
            limit in
            try MainWindowModel.fetchPulsePackages(offset: offset, limit: limit)
        },
        securityRecommendationPackagesFetcher: @escaping (
            Int,
            Int
        ) throws -> PackageSearchPage = {
            offset,
            limit in
            try MainWindowModel.fetchSecurityRecommendationPackages(offset: offset, limit: limit)
        },
        geigerPackagesFetcher: @escaping (Int, Int) throws -> PackageSearchPage = {
            offset,
            limit in
            try MainWindowModel.fetchGeigerPackages(offset: offset, limit: limit)
        },
        searchPackagesFetcher: @escaping (String, Int, Int) throws -> PackageSearchPage = {
            query,
            offset,
            limit in
            try MainWindowModel.searchPackages(query: query, offset: offset, limit: limit)
        },
        userDefaults: UserDefaults = .standard
    ) {
        self.cliToolsRecommendationProvider = cliToolsRecommendationProvider
        automicVaultCLTRecommendation = initialAutomicVaultCLTRecommendation
        self.securityCatalog = securityCatalog
        self.availablePackagesFetcher = availablePackagesFetcher
        self.pulsePackagesFetcher = pulsePackagesFetcher
        self.securityRecommendationPackagesFetcher = securityRecommendationPackagesFetcher
        self.geigerPackagesFetcher = geigerPackagesFetcher
        self.searchPackagesFetcher = searchPackagesFetcher
        self.userDefaults = userDefaults
        newUpdatedLastClickedAt = userDefaults.object(
            forKey: Self.newUpdatedLastClickedAtDefaultsKey
        ) as? Date
    }

    var installedCount: Int {
        snapshot.installedCount > 0 ? snapshot.installedCount : packages.count
    }

    var selectedPackage: PackagePresentation? {
        guard let selectedItemID else { return nil }
        return allKnownPackages.first { $0.selectionID == selectedItemID }
    }

    var selectedDetail: PackageDetail? {
        guard let selectedPackage else {
            return nil
        }
        return detailsByPackageName[selectedPackage.selectionID] ?? selectedPackage.detail
    }

    var displayedPackages: [PackagePresentation] {
        packages(for: selectedSection)
    }

    var isSearchActive: Bool {
        searchText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
    }

    var activeSidebarSection: MainWindowSection? {
        isSearchActive ? nil : selectedSection
    }

    var canUpdateAllOutdated: Bool {
        activeSidebarSection == .outdated
            && !isUpdatingAll
            && activePackageOperation == nil
            && !outdatedUpdatePackageNames.isEmpty
    }

    var shouldShowCategorySortControl: Bool {
        !isSearchActive && selectedSection.categoryIdentifier != nil
    }

    var categorySortButtonTitle: String {
        L10n.format("Sort: %@", categoryPackageSortOrder.title)
    }

    var isPackageMutationInFlight: Bool {
        isUpdatingAll || activePackageOperation != nil
    }

    var shouldShowAutomicVaultCLTInstallButton: Bool {
        isInstallingAutomicVaultCLT
            || automicVaultCLTRecommendation?.missingPackageNames.isEmpty == false
    }

    var canRequestAutomicVaultCLTInstall: Bool {
        automicVaultCLTRecommendation?.missingPackageNames.isEmpty == false
            && !isPackageMutationInFlight
    }

    var isInstallingAutomicVaultCLT: Bool {
        activePackageOperation?.isAutomicVaultCLT == true
            && activePackageOperation?.kind == .install
    }

    var shouldUpdateAutomicVaultCLTWithUpdateAll: Bool {
        automicVaultCLTRecommendation?.isInstalled == true
            && automicVaultCLTRecommendation?.isOutdated == true
    }

    var outdatedUpdatePackageNames: [String] {
        var seen = Set<String>()
        var names: [String] = []

        func append(_ name: String) {
            let normalized = name.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !normalized.isEmpty,
                  seen.insert(normalized).inserted else {
                return
            }
            names.append(normalized)
        }

        snapshot.outdatedPackages.forEach { append($0.name) }
        packages
            .filter(isOutdated)
            .compactMap(\.packageName)
            .forEach(append)
        if shouldUpdateAutomicVaultCLTWithUpdateAll {
            append("av")
        }

        return names.sorted { left, right in
            let leftOrderName = left.packageSearchOrderName
            let rightOrderName = right.packageSearchOrderName
            if leftOrderName == rightOrderName {
                return left < right
            }
            return leftOrderName < rightOrderName
        }
    }

    private var pulsePackageCount: Int? {
        guard pulsePackages.isEmpty == false || pulseTotalCount != nil else {
            return nil
        }
        return pulsePackages.filter(isNewPackageSinceLastNewUpdatedClick).count
    }

    private var displayedNewUpdatedPackageCount: Int? {
        if selectedSection == .newUpdated,
           let count = positiveSidebarCount(newUpdatedSelectionDisplayCount) {
            return count
        }
        return positiveSidebarCount(pulsePackageCount)
    }

    func start() {
        installSnapshotObserverIfNeeded()
        applyStatusSnapshot(statusStore.loadSnapshot())
        reloadPackages()
        statusStore.requestRefresh()
    }

    func stop() {
        if let snapshotObserver {
            DistributedNotificationCenter.default().removeObserver(snapshotObserver)
        }
        transientStatusTask?.cancel()
        searchTask?.cancel()
        sectionPageTasks.values.forEach { $0.cancel() }
        sectionPageTasks.removeAll()
        loadingSectionKinds.removeAll()
        staleSectionKinds.removeAll()
        sectionPageNextOffsets.removeAll()
        searchNextOffset = nil
        updateSelectedSectionLoadingState()
    }

    func reloadPackages() {
        reloadRequestID += 1
        let requestID = reloadRequestID
        let cliToolsRecommendationProvider = cliToolsRecommendationProvider
        isReloading = true
        lastErrorMessage = nil
        statusMessage = L10n.string("Loading packages from the protocol daemon")
        markDynamicSectionPagesStale()
        preloadSidebarCountData()

        Task.detached(priority: .userInitiated) {
            let cltRecommendation = cliToolsRecommendationProvider()
            await MainActor.run {
                self.finishAutomicVaultCLTRecommendationReload(
                    cltRecommendation,
                    requestID: requestID
                )
            }

            let result = Result { try Self.fetchInstalledPackages() }
            await MainActor.run {
                self.finishInstalledReload(
                    result,
                    cltRecommendation: cltRecommendation,
                    requestID: requestID
                )
            }

            guard case .success = result else {
                return
            }

            let outdated = (try? Self.fetchOutdatedPackages()) ?? []
            await MainActor.run {
                self.finishOutdatedReload(outdated, requestID: requestID)
            }
        }
        statusStore.requestRefresh()
    }

    func select(_ package: PackagePresentation) {
        selectedItemID = package.selectionID
        if case .recommendation = package.item {
            if let detail = package.detail {
                detailsByPackageName[package.selectionID] = detail
                detailsByPackageName[detail.packageName] = detail
            }
            isLoadingDetail = false
            return
        }
        loadDetail(for: package)
    }

    func loadNextPageIfNeeded(after package: PackagePresentation) {
        guard shouldPrefetchPage(after: package) else {
            return
        }

        if isSearchActive {
            loadNextSearchPageIfNeeded()
            return
        }

        guard let kind = sectionPageKind(for: selectedSection) else {
            return
        }
        loadNextSectionPageIfNeeded(kind: kind)
    }

    private func clearSelectedPackage() {
        selectedItemID = nil
        detailRequestID += 1
        isLoadingDetail = false
    }

    func requestSearchFocus() {
        searchFocusRequestID += 1
    }

    func selectSection(_ section: MainWindowSection) {
        if section == .newUpdated {
            newUpdatedSelectionDisplayCount = positiveSidebarCount(
                selectedSection == .newUpdated
                    ? (newUpdatedSelectionDisplayCount ?? pulsePackageCount)
                    : pulsePackageCount
            )
            recordNewUpdatedSidebarClick()
        } else {
            newUpdatedSelectionDisplayCount = nil
        }
        selectedSection = section
        if isSearchActive {
            searchText = ""
        }
        searchDeactivationRequestID += 1
    }

    func requestOutdatedUpdateAll() {
        guard canUpdateAllOutdated else {
            if outdatedUpdatePackageNames.isEmpty {
                showTransientStatus(L10n.string("No outdated packages to update"))
            } else if isPackageMutationInFlight {
                showTransientStatus(L10n.string("Package operation already in progress"))
            }
            return
        }
        updateAllRequestID += 1
    }

    func requestAutomicVaultCLTInstall() {
        guard canRequestAutomicVaultCLTInstall else {
            if isPackageMutationInFlight {
                showTransientStatus(L10n.string("Package operation already in progress"))
            } else {
                showTransientStatus(
                    L10n.string("Automic Vault command line tool is already installed")
                )
            }
            return
        }
        packageOperationRequestID += 1
        packageOperationRequest = PackageOperationRequest(
            id: packageOperationRequestID,
            kind: .install,
            packageNames: ["av"],
            displayName: "av",
            isAutomicVaultCLT: true,
            isXcodeCLT: false,
            migrationIsotopeName: nil
        )
    }

    func beginOutdatedUpdateAll(packageCount: Int) {
        transientStatusTask?.cancel()
        isUpdatingAll = true
        lastErrorMessage = nil
        statusMessage = L10n.format("Updating %@", Self.packageCountText(packageCount))
    }

    func finishOutdatedUpdateAll(
        _ result: Result<NukeHelperResult, Error>,
        refreshAfterSuccess: Bool
    ) {
        isUpdatingAll = false
        switch result {
        case .success(let helperResult):
            if refreshAfterSuccess {
                statusMessage = L10n.format("%@; refreshing packages", helperResult.message)
                reloadPackages()
            } else {
                showTransientStatus(helperResult.message)
            }
        case .failure(let error):
            lastErrorMessage = error.localizedDescription
            statusMessage = L10n.string("Update all failed")
        }
    }

    func dossierPrimaryPackageAction(for detail: PackageDetail) -> PackageOperationKind? {
        if securityHardeningPackageName(for: detail) != nil {
            return .harden
        }
        if detail.securityState?.installIsInsecure == true {
            return nil
        }
        guard detail.installed else {
            return .install
        }
        return detail.isOutdated ? .update : .uninstall
    }

    func canRequestDossierPackageAction(
        _ action: PackageOperationKind,
        detail: PackageDetail
    ) -> Bool {
        guard !isPackageMutationInFlight,
              hasPackageOperationTarget(for: detail, action: action) else {
            return false
        }
        if action != .harden,
           detail.securityState?.installIsInsecure == true {
            return false
        }
        switch action {
        case .install:
            return !detail.installed
        case .update:
            return detail.installed && detail.isOutdated
        case .uninstall:
            return detail.installed
                && !detail.isAutomicVaultCLT
                && !detail.isXcodeCLT
        case .harden:
            return securityHardeningPackageName(for: detail) != nil
        }
    }

    func requestDossierPackageAction(
        _ action: PackageOperationKind,
        detail: PackageDetail,
        package: PackagePresentation
    ) {
        guard canRequestDossierPackageAction(action, detail: detail) else {
            showTransientStatus(L10n.string("Package operation is unavailable"))
            return
        }
        let packageNames = packageOperationPackageNames(for: detail, action: action)
        packageOperationRequestID += 1
        packageOperationRequest = PackageOperationRequest(
            id: packageOperationRequestID,
            kind: action,
            packageNames: packageNames,
            displayName: displayName(for: package),
            isAutomicVaultCLT: detail.isAutomicVaultCLT,
            isXcodeCLT: detail.isXcodeCLT,
            migrationIsotopeName: action == .harden
                ? detail.securityState?.isotopeName.trimmingCharacters(in: .whitespacesAndNewlines)
                : nil
        )
    }

    func beginPackageOperation(_ request: PackageOperationRequest) {
        transientStatusTask?.cancel()
        activePackageOperation = request
        lastErrorMessage = nil
        statusMessage = L10n.format("%@ %@", request.kind.progressTitle, request.displayName)
    }

    func finishPackageOperation(
        _ request: PackageOperationRequest,
        _ result: Result<NukeHelperResult, Error>,
        refreshAfterSuccess: Bool
    ) {
        guard activePackageOperation?.id == request.id else {
            return
        }
        activePackageOperation = nil
        switch result {
        case .success(let helperResult):
            if request.isAutomicVaultCLT {
                automicVaultCLTRecommendation = nil
            }
            if request.kind == .harden {
                retireCompletedHardening(request)
            }
            if refreshAfterSuccess {
                statusMessage = L10n.format("%@; refreshing packages", helperResult.message)
                reloadPackages()
            } else {
                showTransientStatus(helperResult.message)
            }
        case .failure(let error):
            lastErrorMessage = error.localizedDescription
            statusMessage = L10n.format("%@ failed", request.kind.title)
        }
    }

    func selectedURL(for tab: MainWindowLinkTab) -> URL? {
        guard let detail = selectedDetail else {
            return nil
        }
        return linkURL(for: tab, detail: detail)
    }

    func linkURL(for tab: MainWindowLinkTab, detail: PackageDetail) -> URL? {
        let homepageURL = detail.homepageURL ?? catalogHomepageURL(for: detail)
        switch tab {
        case .homepage:
            return homepageURL
        case .repository:
            return detail.repositoryURL ?? githubRepositoryURL(from: homepageURL)
        case .documentation:
            return detail.upstreamDocsURL ?? documentationURL(from: homepageURL)
        }
    }

    private func catalogHomepageURL(for detail: PackageDetail) -> URL? {
        if detail.securityState?.needsMainWindowSecurityAlert == true {
            return securityCatalog.homepageURL(for: detail)
        }
        if case .isotope = detail.source {
            return securityCatalog.homepageURL(for: detail)
        }
        return nil
    }

    func open(url: URL?) {
        guard let url else {
            return
        }
        NSWorkspace.shared.open(url)
    }

    func showTransientStatus(_ message: String) {
        transientStatusTask?.cancel()
        statusMessage = message
        transientStatusTask = Task { [weak self] in
            try? await Task.sleep(for: .seconds(4))
            guard !Task.isCancelled else { return }
            self?.statusMessage = nil
        }
    }

    func selectCategorySortOrder(_ sortOrder: CategoryPackageSortOrder) {
        guard categoryPackageSortOrder != sortOrder else {
            return
        }
        categoryPackageSortOrder = sortOrder
        ensureSelectedSectionLoaded()
        updateSelectedSectionLoadingState()
    }

    func count(for section: MainWindowSection) -> Int? {
        switch section {
        case .installed:
            return installedCount
        case .securityRecommendations:
            return securityRecommendationTotalCount
                ?? (securityRecommendationPackages.isEmpty
                    ? nil
                    : securityRecommendationPackages.count)
        case .geigerCounter:
            return geigerCounterCount
        case .newUpdated:
            return displayedNewUpdatedPackageCount
        case .outdated:
            return max(outdatedUpdatePackageNames.count, snapshot.flaggedOutdatedPackageCount)
        case .allPackages:
            return catalogTotalCount ?? (catalogPackages.isEmpty ? nil : catalogPackages.count)
        case .settings, .about:
            return nil
        case .developerTools, .cloudInfrastructure, .networking, .system, .security,
             .data, .languageRuntime, .media, .productivity, .science, .games, .toys, .other:
            guard let category = section.categoryIdentifier else { return nil }
            if let count = catalogCategoryCounts[category] {
                return count
            }
            let pageKey = CategoryCatalogPageKey(
                category: category,
                sortOrder: categoryPackageSortOrder
            )
            if let count = categoryTotalCountsByPageKey[pageKey] {
                return count
            }
            let loadedPackages = categorySourcePackages(for: category)
            guard loadedPackages.isEmpty == false else { return nil }
            return loadedPackages.count
        }
    }

    func packageBadge(for package: PackagePresentation) -> MainWindowPackageBadge? {
        if needsHardening(package) {
            return .vulnerable
        }
        if isInstalledAsIsotope(package) {
            return .hardened
        }
        if isGeigerProtocolPackage(package) {
            return .vulnerable
        }
        if isInstalledAsRoot(package) {
            return .immutable
        }
        return nil
    }

    func packageListBadges(for package: PackagePresentation) -> [MainWindowPackageBadge] {
        var badges: [MainWindowPackageBadge] = []
        if let badge = packageBadge(for: package) {
            badges.append(badge)
        }
        return badges
    }

    func packageInlineBadges(for package: PackagePresentation) -> [MainWindowPackageBadge] {
        guard !isSearchActive,
              selectedSection == .installed,
              isOutdated(package) else {
            return []
        }
        return [.outdated]
    }

    private func needsHardening(_ package: PackagePresentation) -> Bool {
        let detail = detailsByPackageName[package.selectionID] ?? package.detail
        return package.hasMainWindowSecurityAlert(resolvedDetail: detail)
    }

    private func isInstalledAsIsotope(_ package: PackagePresentation) -> Bool {
        if package.isInstalledIsotope {
            return true
        }
        let detail = detailsByPackageName[package.selectionID] ?? package.detail
        if detail?.installed == true,
           case .isotope = detail?.source {
            return true
        }
        return false
    }

    private func isInstalledAsRoot(_ package: PackagePresentation) -> Bool {
        if isInstalledAsIsotope(package) {
            return false
        }

        if let detail = detailsByPackageName[package.selectionID] ?? package.detail,
           detail.installed {
            return isRootInstall(detail.installRoot)
        }

        if case .installed(let record) = package.item,
           let installRoot = record.installRoot {
            return isRootInstall(installRoot)
        }

        return false
    }

    private func isRootInstall(_ installRoot: String) -> Bool {
        let trimmedRoot = installRoot.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmedRoot.isEmpty == false else {
            return false
        }
        return trimmedRoot != "/opt/homebrew"
            && !trimmedRoot.hasPrefix("/opt/homebrew/")
    }

    func isHardened(
        _ package: PackagePresentation,
        detail detailOverride: PackageDetail? = nil
    ) -> Bool {
        let detail = detailOverride ?? detailsByPackageName[package.selectionID] ?? package.detail
        if package.hasMainWindowSecurityAlert(resolvedDetail: detail) {
            return false
        }
        if package.isInstalledIsotope {
            return true
        }
        if case .isotope = detail?.source {
            return true
        }
        return detail?.securityState != nil
    }

    func displayName(for package: PackagePresentation) -> String {
        strippedPackagePrefix(package.displayName)
    }

    func packageDescription(for package: PackagePresentation) -> String {
        let detail = detailsByPackageName[package.selectionID] ?? package.detail
        if let text = detail?.homebrewInfo?.description,
           text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false {
            return text
        }
        if case .installed = package.item {
            return detail?.primaryDescription
                ?? L10n.string("Installed component record available in the local vault.")
        }
        return package.listSecondaryText
    }

    func versionText(for package: PackagePresentation) -> String {
        switch package.item {
        case .installed(let record):
            if !isSearchActive,
               selectedSection == .outdated,
               let latestVersion = record.latestVersion,
               latestVersion.isEmpty == false,
               latestVersion != record.version {
                return "\(record.version) → \(latestVersion)"
            }
            return record.version
        case .recommendation, .available, .command:
            return package.versionText
        }
    }

    func relativeLastUpdatedText(for detail: PackageDetail?) -> String {
        guard let raw = detail?.lastUpdatedAt,
              let date = Self.parseISO8601Date(raw) else {
            return relativeRefreshText
        }
        return Self.relativeFormatter.localizedString(for: date, relativeTo: Date())
    }

    func pulseListTimestampText(for result: PackageSearchResult) -> String {
        Self.pulseListTimestampText(for: result, relativeTo: Date())
    }

    static func pulseListTimestampText(
        for result: PackageSearchResult,
        relativeTo referenceDate: Date
    ) -> String {
        guard let raw = result.lastUpdatedAt,
              let date = parseISO8601Date(raw) else {
            return result.isNewPulse
                ? L10n.string("recently")
                : L10n.string("Updated recently")
        }
        let ageText = relativeAgeText(for: date, relativeTo: referenceDate)
        return result.isNewPulse ? ageText : L10n.format("Updated %@", ageText)
    }

    var relativeRefreshText: String {
        guard snapshot.refreshedAt > .distantPast else {
            return L10n.string("Not yet refreshed")
        }
        return Self.relativeFormatter.localizedString(
            for: snapshot.refreshedAt,
            relativeTo: Date()
        )
    }

    var outdatedPackageNames: Set<String> {
        Set(snapshot.outdatedPackages.map(\.name))
    }

    private var allKnownPackages: [PackagePresentation] {
        var seen = Set<String>()
        var result: [PackagePresentation] = []
        let categoryPackages = categoryPackagesByPageKey.values.flatMap { $0 }
        for package in packages
            + localOutdatedPackages
            + securityRecommendationPackages
            + geigerPackages
            + catalogPackages
            + categoryPackages
            + pulsePackages
            + searchResults {
            if seen.insert(package.selectionID).inserted {
                result.append(package)
            }
        }
        return result
    }

    private func hasPackageOperationTarget(
        for detail: PackageDetail,
        action: PackageOperationKind
    ) -> Bool {
        detail.isAutomicVaultCLT
            || detail.isXcodeCLT
            || !packageOperationPackageNames(for: detail, action: action).isEmpty
    }

    private func packageOperationPackageNames(
        for detail: PackageDetail,
        action: PackageOperationKind
    ) -> [String] {
        if detail.isAutomicVaultCLT {
            return ["av"]
        }
        if detail.isXcodeCLT {
            return [PackageRecommendation.xcodeCLTName]
        }
        if action == .harden {
            if let packageName = securityHardeningPackageName(for: detail) {
                return [packageName]
            }
            return []
        }
        var seen = Set<String>()
        var names: [String] = []
        for packageName in detail.helperPackageNames {
            let trimmed = packageName.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !trimmed.isEmpty,
                  seen.insert(trimmed).inserted else {
                continue
            }
            names.append(trimmed)
        }
        return names
    }

    private func securityHardeningPackageName(for detail: PackageDetail) -> String? {
        guard let securityState = detail.securityState,
              securityState.installIsInsecure else {
            return nil
        }
        let packageName = securityCatalog.notice(for: detail)?.applyPackageName?
            .trimmingCharacters(in: .whitespacesAndNewlines)
        guard packageName?.isEmpty == false else {
            return nil
        }
        return packageName
    }

    private var geigerCounterCount: Int? {
        let count = geigerActionPackages.count
        return count > 0 ? count : nil
    }

    private var geigerActionPackages: [PackagePresentation] {
        Self.securityAlertPackages(
            installed: packages.filter(isGeigerActionPackage),
            geiger: geigerPackages
        )
    }

    private var catalogSourcePackages: [PackagePresentation] {
        catalogPackages
    }

    private func categorySourcePackages(for category: String) -> [PackagePresentation] {
        let pageKey = CategoryCatalogPageKey(
            category: category,
            sortOrder: categoryPackageSortOrder
        )
        if let packages = categoryPackagesByPageKey[pageKey],
           packages.isEmpty == false || categoryTotalCountsByPageKey[pageKey] != nil {
            return packages
        }
        guard categoryPackageSortOrder == .rank else {
            return []
        }
        return catalogSourcePackages.filter {
            packageCategoryIdentifier($0) == category
        }
    }

    private func uniquePackages(_ source: [PackagePresentation]) -> [PackagePresentation] {
        var seen = Set<String>()
        var result: [PackagePresentation] = []
        for package in source where seen.insert(package.selectionID).inserted {
            result.append(package)
        }
        return result
    }

    static func securityAlertPackages(
        installed installedPackages: [PackagePresentation],
        geiger geigerPackages: [PackagePresentation]
    ) -> [PackagePresentation] {
        uniqueSecurityAlertPackages(installedPackages + geigerPackages)
    }

    static func mergedSearchPackages(
        installed installedPackages: [PackagePresentation],
        daemon daemonPackages: [PackagePresentation]
    ) -> [PackagePresentation] {
        var seen = Set<String>()
        var result: [PackagePresentation] = []

        for package in installedPackages + daemonPackages {
            let keys = packageIdentityDeduplicationKeys(for: package)
            guard keys.contains(where: seen.contains) == false else {
                continue
            }
            keys.forEach { seen.insert($0) }
            result.append(package)
        }

        return result
    }

    private static func uniqueSecurityAlertPackages(
        _ source: [PackagePresentation]
    ) -> [PackagePresentation] {
        var seen = Set<String>()
        var result: [PackagePresentation] = []

        for package in source {
            let keys = securityAlertDeduplicationKeys(for: package)
            guard keys.contains(where: seen.contains) == false else {
                continue
            }
            keys.forEach { seen.insert($0) }
            result.append(package)
        }

        return result
    }

    private static func securityAlertDeduplicationKeys(
        for package: PackagePresentation
    ) -> [String] {
        var seen = Set<String>()
        var keys: [String] = []

        func append(_ key: String?) {
            guard let key = key?.trimmingCharacters(in: .whitespacesAndNewlines),
                  key.isEmpty == false else {
                return
            }
            let normalized = key.lowercased()
            guard seen.insert(normalized).inserted else {
                return
            }
            keys.append(normalized)
        }

        func appendSubject(_ key: String?) {
            guard let subject = securityAlertSubjectName(for: key) else {
                return
            }
            append("subject:\(subject)")
        }

        if let state = securityState(for: package),
           state.needsMainWindowSecurityAlert {
            append("security:\(state.isotopeName)")
            appendSubject(state.isotopeName)
        }

        packageIdentityDeduplicationKeys(for: package).forEach { key in
            append(key)
            appendSubject(key)
        }

        return keys.isEmpty ? [package.selectionID.lowercased()] : keys
    }

    private static func packageIdentityDeduplicationKeys(
        for package: PackagePresentation
    ) -> [String] {
        var seen = Set<String>()
        var keys: [String] = []

        func append(_ key: String?) {
            guard let key = key?.trimmingCharacters(in: .whitespacesAndNewlines),
                  key.isEmpty == false else {
                return
            }
            let normalized = key.lowercased()
            guard seen.insert(normalized).inserted else {
                return
            }
            keys.append(normalized)
        }

        switch package.item {
        case .installed(let record):
            append(record.name)
            record.installPackageNames?.forEach(append)
            append(sourceQualifiedName(for: record.source))
        case .available(let result):
            append(result.name)
            append(result.detailLookupName)
            append(sourceQualifiedName(for: result.source))
        case .recommendation(let recommendation):
            append(recommendation.detail.packageName)
            append(recommendation.detail.qualifiedName)
            recommendation.detail.installPackageNames?.forEach(append)
            append(sourceQualifiedName(for: recommendation.detail.source))
        case .command(let command):
            append(command.selectionID)
        }

        if let detail = package.detail {
            append(detail.packageName)
            append(detail.qualifiedName)
            detail.installPackageNames?.forEach(append)
            append(sourceQualifiedName(for: detail.source))
        }

        append(package.packageName)
        append(package.selectionID)
        append(package.preferredDetailLookupName)

        return keys.isEmpty ? [package.selectionID.lowercased()] : keys
    }

    private static func securityAlertSubjectName(for key: String?) -> String? {
        guard var value = key?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased(),
              value.isEmpty == false else {
            return nil
        }

        let prefixes = [
            "search:",
            "geiger:",
            "brew:",
            "cask:",
            "gone:",
            "sys:",
            "isotope:",
            "npm:",
            "pip:"
        ]
        var strippedPrefix = true
        while strippedPrefix {
            strippedPrefix = false
            for prefix in prefixes where value.hasPrefix(prefix) {
                value = String(value.dropFirst(prefix.count))
                strippedPrefix = true
                break
            }
        }

        let subject = value.packageSearchOrderName
            .split(separator: "/", omittingEmptySubsequences: true)
            .last
            .map(String.init)
            ?? value
        return subject.isEmpty ? nil : subject
    }

    private static func securityState(
        for package: PackagePresentation
    ) -> PackageSecurityState? {
        if let state = package.detail?.securityState {
            return state
        }
        switch package.item {
        case .installed(let record):
            return record.securityState
        case .recommendation(let recommendation):
            return recommendation.detail.securityState
        case .available(let result):
            return result.securityState
        case .command:
            return nil
        }
    }

    private static func sourceQualifiedName(for source: PackageSource?) -> String? {
        switch source {
        case .formula(let rootFormula):
            return "brew:\(rootFormula)"
        case .cask(let caskName):
            return "cask:\(caskName)"
        case .isotope(let isotopeName):
            return "isotope:\(isotopeName)"
        case .vendor(let vendorName):
            return vendorName
        case .npm(let packageName):
            return "npm:\(packageName)"
        case .pip(let packageName):
            return "pip:\(packageName)"
        case .none:
            return nil
        }
    }

    private func packages(for section: MainWindowSection) -> [PackagePresentation] {
        let query = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        if query.isEmpty == false {
            return mergedSearchPackages(query: query)
        }
        guard section != .settings, section != .about else {
            return []
        }

        let source: [PackagePresentation]
        switch section {
        case .installed:
            source = packages
        case .securityRecommendations:
            source = securityRecommendationPackages
        case .geigerCounter:
            source = geigerActionPackages
        case .newUpdated:
            source = pulsePackages
        case .outdated:
            source = packages.filter(isOutdated) + localOutdatedPackages
        case .allPackages:
            source = catalogSourcePackages
        case .developerTools, .cloudInfrastructure, .networking, .system, .security,
             .data, .languageRuntime, .media, .productivity, .science, .games, .toys, .other:
            if let category = section.categoryIdentifier {
                source = categorySourcePackages(for: category)
            } else {
                source = []
            }
        case .settings, .about:
            source = []
        }

        return source.filter { package in
            sectionMatches(section, package: package)
        }
    }

    private func mergedSearchPackages(query: String) -> [PackagePresentation] {
        let installedMatches = packages.filter {
            packageMatchesQuery($0, query: query)
        }
        return Self.mergedSearchPackages(
            installed: installedMatches,
            daemon: searchResults
        )
    }

    private func sectionMatches(
        _ section: MainWindowSection,
        package: PackagePresentation
    ) -> Bool {
        switch section {
        case .installed, .securityRecommendations, .allPackages:
            return true
        case .geigerCounter:
            return isGeigerActionPackage(package)
        case .newUpdated:
            return true
        case .outdated:
            return isOutdated(package)
        case .developerTools, .cloudInfrastructure, .networking, .system, .security,
             .data, .languageRuntime, .media, .productivity, .science, .games, .toys, .other:
            guard let category = section.categoryIdentifier else {
                return false
            }
            return packageCategoryIdentifier(package) == category
        case .settings, .about:
            return false
        }
    }

    private func isGeigerActionPackage(_ package: PackagePresentation) -> Bool {
        packageBadge(for: package) == .vulnerable
    }

    private func packageCategoryIdentifier(_ package: PackagePresentation) -> String {
        if let trimmed = package.categoryIdentifier?
            .trimmingCharacters(in: .whitespacesAndNewlines),
           !trimmed.isEmpty {
            return trimmed
        }
        return "other"
    }

    private func isGeigerProtocolPackage(_ package: PackagePresentation) -> Bool {
        package.selectionID.hasPrefix("geiger:")
    }

    private func packageMatchesQuery(
        _ package: PackagePresentation,
        query: String
    ) -> Bool {
        let normalized = query.lowercased()
        return package.displayName.localizedCaseInsensitiveContains(normalized)
            || package.listSecondaryText.localizedCaseInsensitiveContains(normalized)
            || (package.detail?.primaryDescription.localizedCaseInsensitiveContains(normalized) ?? false)
    }

    private func isOutdated(_ package: PackagePresentation) -> Bool {
        if case .recommendation(let recommendation) = package.item {
            return recommendation.isOutdated
        }
        if let detail = detailsByPackageName[package.selectionID] ?? package.detail {
            return detail.isOutdated
        }
        guard let name = package.packageName else {
            return false
        }
        return outdatedPackageNames.contains(name)
    }

    private func isNewPackageSinceLastNewUpdatedClick(_ package: PackagePresentation) -> Bool {
        guard case .available(let result) = package.item,
              result.isNewPulse else {
            return false
        }
        guard let newUpdatedLastClickedAt else {
            return true
        }
        guard let raw = result.lastUpdatedAt,
              let packageUpdatedAt = Self.parseISO8601Date(raw) else {
            return false
        }
        return packageUpdatedAt > newUpdatedLastClickedAt
    }

    private func recordNewUpdatedSidebarClick() {
        let clickedAt = Date()
        newUpdatedLastClickedAt = clickedAt
        userDefaults.set(clickedAt, forKey: Self.newUpdatedLastClickedAtDefaultsKey)
    }

    private func positiveSidebarCount(_ count: Int?) -> Int? {
        guard let count, count > 0 else {
            return nil
        }
        return count
    }

    private func installSnapshotObserverIfNeeded() {
        guard snapshotObserver == nil else {
            return
        }
        snapshotObserver = statusStore.observeSnapshotChanges { [weak self] _ in
            Task { @MainActor in
                self?.applyStatusSnapshot(self?.statusStore.loadSnapshot() ?? .empty)
            }
        }
    }

    private func applyStatusSnapshot(_ snapshot: NucleusStatusSnapshot) {
        let shouldKeepDaemonSnapshot =
            snapshot.installedCount == 0
            && snapshot.outdatedPackages.isEmpty
            && packages.isEmpty == false
        self.snapshot = shouldKeepDaemonSnapshot
            ? self.snapshot.withRemoteDatabaseRefreshState(snapshot.remoteDatabaseRefreshState)
            : snapshot
        packages = packages.map { package in
            guard case .installed(let record) = package.item else {
                return package
            }
            let merged = mergeOutdatedState(into: record)
            let detail = installedDetail(for: merged, fallback: package.detail)
            return PackagePresentation(
                item: .installed(merged),
                detail: detail,
                freshness: package.freshness,
                presentationID: package.presentationID
            )
        }
    }

    private func finishInstalledReload(
        _ result: Result<[PackageRecord], Error>,
        cltRecommendation: PackageRecommendation?,
        requestID: Int
    ) {
        guard requestID == reloadRequestID else {
            return
        }

        isReloading = false
        automicVaultCLTRecommendation = cltRecommendation
        switch result {
        case .success(let installed):
            snapshot = NucleusStatusSnapshot(
                installedCount: installed.count,
                hazardousPackageCount: installed.filter(\.hasMainWindowSecurityAlert).count,
                outdatedPackages: snapshot.outdatedPackages,
                refreshedAt: Date(),
                lastError: nil,
                remoteDatabaseRefreshState: snapshot.remoteDatabaseRefreshState
            )

            packages = installed.map { record in
                let merged = mergeOutdatedState(into: record)
                let detail = installedDetail(for: merged)
                return PackagePresentation(
                    item: .installed(merged),
                    detail: detail,
                    freshness: Self.freshness(for: merged.name)
                )
            }
            let selectedPendingHardening = selectPendingHardeningPackageIfPossible()
            if !selectedPendingHardening {
                if pendingHardeningSelection != nil,
                   selectedItemID != nil,
                   selectedPackage == nil {
                    selectedItemID = nil
                }
                if let selectedItemID,
                   allKnownPackages.contains(where: { $0.selectionID == selectedItemID }) {
                    loadSelectedDetailIfPossible()
                } else {
                    selectedItemID = nil
                }
            }
            statusMessage = nil
            ensureSelectedSectionLoaded()
            preloadSidebarCountData()
        case .failure(let error):
            lastErrorMessage = error.localizedDescription
            statusMessage = L10n.string("Package refresh failed")
        }
    }

    private func finishAutomicVaultCLTRecommendationReload(
        _ recommendation: PackageRecommendation?,
        requestID: Int
    ) {
        guard requestID == reloadRequestID else {
            return
        }
        automicVaultCLTRecommendation = recommendation
    }

    private func finishOutdatedReload(
        _ outdated: [OutdatedPackageRecord],
        requestID: Int
    ) {
        guard requestID == reloadRequestID else {
            return
        }
        snapshot = NucleusStatusSnapshot(
            installedCount: snapshot.installedCount,
            hazardousPackageCount: snapshot.hazardousPackageCount,
            outdatedPackages: outdated,
            refreshedAt: snapshot.refreshedAt,
            lastError: snapshot.lastError,
            remoteDatabaseRefreshState: snapshot.remoteDatabaseRefreshState
        )
        packages = packages.map { package in
            guard case .installed(let record) = package.item else {
                return package
            }
            let merged = mergeOutdatedState(into: record)
            let detail = installedDetail(for: merged, fallback: package.detail)
            return PackagePresentation(
                item: .installed(merged),
                detail: detail,
                freshness: package.freshness,
                presentationID: package.presentationID
            )
        }
    }

    private func loadSelectedDetailIfPossible() {
        guard let package = selectedPackage else {
            return
        }
        loadDetail(for: package)
    }

    private func loadDetail(for package: PackagePresentation) {
        let packageName = detailLookupName(for: package)
        detailRequestID += 1
        let requestID = detailRequestID
        isLoadingDetail = true

        Task.detached(priority: .userInitiated) {
            let result = Result {
                try Self.fetchDetail(packageName: packageName)
            }
            await MainActor.run {
                self.finishDetailLoad(
                    result,
                    package: package,
                    requestID: requestID
                )
            }
        }
    }

    private func finishDetailLoad(
        _ result: Result<PackageDetail, Error>,
        package: PackagePresentation,
        requestID: Int
    ) {
        guard requestID == detailRequestID else {
            return
        }
        isLoadingDetail = false
        guard selectedItemID == package.selectionID else {
            return
        }
        switch result {
        case .success(let detail):
            let normalized = normalizedDetail(detail)
                .preservingLocalSecurityContext(from: package.detail)
            detailsByPackageName[package.selectionID] = normalized
            detailsByPackageName[detail.packageName] = normalized
            detailsByPackageName[detailLookupName(for: package)] = normalized
            packages = packages.map { current in
                guard current.selectionID == package.selectionID else {
                    return current
                }
                return PackagePresentation(
                    item: current.item,
                    detail: normalized,
                    freshness: current.freshness,
                    presentationID: current.presentationID
                )
            }
            catalogPackages = catalogPackages.updatingDetail(
                normalized,
                for: package.selectionID
            )
            securityRecommendationPackages = securityRecommendationPackages.updatingDetail(
                normalized,
                for: package.selectionID
            )
            categoryPackagesByPageKey = categoryPackagesByPageKey.mapValues {
                $0.updatingDetail(normalized, for: package.selectionID)
            }
            geigerPackages = geigerPackages.updatingDetail(
                normalized,
                for: package.selectionID
            )
            pulsePackages = pulsePackages.updatingDetail(
                normalized,
                for: package.selectionID
            )
            searchResults = searchResults.updatingDetail(
                normalized,
                for: package.selectionID
            )
        case .failure:
            if let fallback = package.detail {
                detailsByPackageName[package.selectionID] = fallback
            }
        }
    }

    private func normalizedDetail(_ detail: PackageDetail) -> PackageDetail {
        if let outdated = snapshot.outdatedPackagesByName[detail.packageName] {
            return detail.applying(outdated: outdated)
        }
        return detail
    }

    private func detailLookupName(for package: PackagePresentation) -> String {
        package.preferredDetailLookupName
    }

    private func presentation(
        for result: PackageSearchResult,
        prefix: String?
    ) -> PackagePresentation {
        let presentationID = prefix.map { "\($0):\(result.name)" }
        return PackagePresentation(
            item: .available(result),
            detail: detailsByPackageName[result.detailLookupName] ?? result.fallbackDetail,
            freshness: Self.freshness(for: result.detailLookupName),
            presentationID: presentationID
        )
    }

    private func scheduleSearch() {
        searchTask?.cancel()
        let query = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard query.isEmpty == false else {
            isSearching = false
            searchResults = []
            searchTotalCount = 0
            searchNextOffset = nil
            ensureSelectedSectionLoaded()
            updateSelectedSectionLoadingState()
            return
        }

        searchRequestID += 1
        let requestID = searchRequestID
        let searchPackagesFetcher = searchPackagesFetcher
        isSearching = true
        updateSelectedSectionLoadingState()
        searchTask = Task { [weak self] in
            try? await Task.sleep(for: .milliseconds(180))
            guard !Task.isCancelled else { return }
            let result = await Task.detached(priority: .userInitiated) {
                Result {
                    try searchPackagesFetcher(query, 0, Self.pageSize)
                }
            }.value
            await MainActor.run {
                self?.finishSearch(
                    result,
                    query: query,
                    requestID: requestID,
                    appending: false
                )
            }
        }
    }

    private func finishSearch(
        _ result: Result<PackageSearchPage, Error>,
        query: String,
        requestID: Int,
        appending: Bool
    ) {
        guard requestID == searchRequestID,
              query == searchText.trimmingCharacters(in: .whitespacesAndNewlines) else {
            return
        }
        isSearching = false
        switch result {
        case .success(let page):
            searchTotalCount = page.totalCount
            searchNextOffset = page.nextOffset
            let packages = page.packages.map {
                presentation(for: $0, prefix: "search")
            }
            if appending {
                searchResults = searchResults.appendingUniquePackages(packages)
            } else {
                searchResults = packages
            }
        case .failure(let error):
            if !appending {
                searchTotalCount = 0
                searchResults = []
                searchNextOffset = nil
            }
            lastErrorMessage = error.localizedDescription
        }
    }

    private func shouldPrefetchPage(after package: PackagePresentation) -> Bool {
        let visiblePackages = displayedPackages
        guard let index = visiblePackages.firstIndex(where: {
            $0.selectionID == package.selectionID
        }) else {
            return false
        }
        let remainingCount = visiblePackages.distance(
            from: index,
            to: visiblePackages.endIndex
        )
        return remainingCount <= Self.paginationPrefetchThreshold
    }

    private func loadNextSearchPageIfNeeded() {
        guard !isSearching,
              let nextOffset = searchNextOffset else {
            return
        }

        let query = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else {
            return
        }

        searchRequestID += 1
        let requestID = searchRequestID
        let searchPackagesFetcher = searchPackagesFetcher
        isSearching = true
        updateSelectedSectionLoadingState()

        searchTask = Task { [weak self] in
            let result = await Task.detached(priority: .userInitiated) {
                Result {
                    try searchPackagesFetcher(query, nextOffset, Self.pageSize)
                }
            }.value
            await MainActor.run {
                self?.finishSearch(
                    result,
                    query: query,
                    requestID: requestID,
                    appending: true
                )
            }
        }
    }

    private func ensureSelectedSectionLoaded() {
        let query = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard query.isEmpty else {
            return
        }
        guard let kind = sectionPageKind(for: selectedSection) else {
            return
        }
        loadSectionPageIfNeeded(kind: kind)
    }

    private func preloadSidebarCountData() {
        loadSectionPageIfNeeded(kind: .geiger)
        loadSectionPageIfNeeded(kind: .pulse)
        loadSectionPageIfNeeded(kind: .catalog(category: nil, sortOrder: .rank))
    }

    private func markDynamicSectionPagesStale() {
        markSectionPageStale(kind: .geiger)
        markSectionPageStale(kind: .pulse)
    }

    private func markSectionPageStale(kind: SectionPageKind) {
        staleSectionKinds.insert(kind)
        sectionPageTasks[kind]?.cancel()
        sectionPageTasks[kind] = nil
        loadingSectionKinds.remove(kind)
        sectionPageRequestIDs[kind] = (sectionPageRequestIDs[kind] ?? 0) + 1
        sectionPageNextOffsets[kind] = nil
        if case .catalog = kind {
            catalogCategoryCounts = [:]
            categoryPackagesByPageKey.removeAll()
            categoryTotalCountsByPageKey.removeAll()
        }
        updateSelectedSectionLoadingState()
    }

    private struct CategoryCatalogPageKey: Sendable, Hashable {
        let category: String
        let sortOrder: CategoryPackageSortOrder
    }

    private enum SectionPageKind: Sendable, Hashable {
        case securityRecommendations
        case geiger
        case catalog(category: String?, sortOrder: CategoryPackageSortOrder)
        case pulse

        init?(section: MainWindowSection, categorySortOrder: CategoryPackageSortOrder) {
            switch section {
            case .securityRecommendations:
                self = .securityRecommendations
            case .geigerCounter:
                self = .geiger
            case .newUpdated:
                self = .pulse
            case .allPackages:
                self = .catalog(category: nil, sortOrder: .rank)
            case .developerTools,
                 .cloudInfrastructure,
                 .networking,
                 .system,
                 .security,
                 .data,
                 .languageRuntime,
                 .media,
                 .productivity,
                 .science,
                 .games,
                 .toys,
                 .other:
                self = .catalog(
                    category: section.categoryIdentifier,
                    sortOrder: categorySortOrder
                )
            case .installed, .outdated, .settings, .about:
                return nil
            }
        }
    }

    private func sectionPageKind(for section: MainWindowSection) -> SectionPageKind? {
        SectionPageKind(section: section, categorySortOrder: categoryPackageSortOrder)
    }

    private func loadSectionPageIfNeeded(kind: SectionPageKind) {
        guard isSectionPageLoaded(kind) == false else {
            return
        }
        loadSectionPage(kind: kind, offset: 0)
    }

    private func isSectionPageLoaded(_ kind: SectionPageKind) -> Bool {
        guard staleSectionKinds.contains(kind) == false else {
            return false
        }
        switch kind {
        case .securityRecommendations:
            return securityRecommendationPackages.isEmpty == false
                || securityRecommendationTotalCount != nil
        case .geiger:
            return geigerPackages.isEmpty == false || geigerTotalCount != nil
        case .catalog(nil, _):
            return catalogPackages.isEmpty == false || catalogTotalCount != nil
        case .catalog(let category?, let sortOrder):
            let pageKey = CategoryCatalogPageKey(category: category, sortOrder: sortOrder)
            return categoryPackagesByPageKey[pageKey]?.isEmpty == false
                || categoryTotalCountsByPageKey[pageKey] != nil
        case .pulse:
            return pulsePackages.isEmpty == false || pulseTotalCount != nil
        }
    }

    private func loadNextSectionPageIfNeeded(kind: SectionPageKind) {
        guard let nextOffset = sectionPageNextOffsets[kind] else {
            return
        }
        loadSectionPage(kind: kind, offset: nextOffset)
    }

    private func loadSectionPage(kind: SectionPageKind, offset: Int) {
        guard loadingSectionKinds.contains(kind) == false else {
            return
        }
        let requestID = (sectionPageRequestIDs[kind] ?? 0) + 1
        sectionPageRequestIDs[kind] = requestID
        loadingSectionKinds.insert(kind)
        updateSelectedSectionLoadingState()
        lastErrorMessage = nil
        let fetcher = sectionPageFetcher(for: kind)
        sectionPageTasks[kind] = Task { [weak self] in
            let result = await Task.detached(priority: .userInitiated) {
                Result {
                    try fetcher(offset, Self.pageSize)
                }
            }.value
            await MainActor.run {
                self?.finishSectionPage(
                    result,
                    kind: kind,
                    offset: offset,
                    requestID: requestID
                )
            }
        }
    }

    private func finishSectionPage(
        _ result: Result<PackageSearchPage, Error>,
        kind: SectionPageKind,
        offset: Int,
        requestID: Int
    ) {
        guard requestID == sectionPageRequestIDs[kind] else {
            return
        }
        loadingSectionKinds.remove(kind)
        sectionPageTasks[kind] = nil
        staleSectionKinds.remove(kind)
        updateSelectedSectionLoadingState()
        switch result {
        case .success(let page):
            sectionPageNextOffsets[kind] = page.nextOffset
            let previousVisibleCount = displayedPackages.count
            switch kind {
            case .securityRecommendations:
                securityRecommendationTotalCount = page.totalCount
                let packages = page.packages.map {
                    presentation(for: $0, prefix: "security-recommendation")
                }
                securityRecommendationPackages = offset == 0
                    ? packages
                    : securityRecommendationPackages.appendingUniquePackages(packages)
            case .geiger:
                geigerTotalCount = page.totalCount
                let packages = page.packages.map { result in
                    result.detectedLocalHazardPresentation(
                        freshness: Self.freshness(for: result.detailLookupName)
                    )?.presentation ?? presentation(for: result, prefix: "geiger")
                }
                geigerPackages = offset == 0
                    ? packages
                    : geigerPackages.appendingUniquePackages(packages)
            case .catalog(let category, let sortOrder):
                if !page.categoryCounts.isEmpty {
                    catalogCategoryCounts = page.categoryCounts
                }
                let packages = page.packages.map {
                    presentation(for: $0, prefix: nil)
                }
                if let category {
                    let pageKey = CategoryCatalogPageKey(
                        category: category,
                        sortOrder: sortOrder
                    )
                    categoryTotalCountsByPageKey[pageKey] = page.totalCount
                    categoryPackagesByPageKey[pageKey] = offset == 0
                        ? packages
                        : (categoryPackagesByPageKey[pageKey] ?? [])
                            .appendingUniquePackages(packages)
                } else {
                    catalogTotalCount = page.totalCount
                    catalogPackages = offset == 0
                        ? packages
                        : catalogPackages.appendingUniquePackages(packages)
                }
            case .pulse:
                pulseTotalCount = page.totalCount
                let packages = page.packages.map {
                    presentation(for: $0, prefix: "pulse")
                }
                pulsePackages = offset == 0
                    ? packages
                    : pulsePackages.appendingUniquePackages(packages)
            }
            if offset > 0,
               kind == sectionPageKind(for: selectedSection),
               displayedPackages.count == previousVisibleCount {
                loadNextSectionPageIfNeeded(kind: kind)
            } else if isSelectedCategoryCatalogPage(kind),
                      displayedPackages.isEmpty,
                      page.nextOffset != nil {
                loadNextSectionPageIfNeeded(kind: kind)
            }
        case .failure(let error):
            lastErrorMessage = error.localizedDescription
        }
    }

    private func updateSelectedSectionLoadingState() {
        guard !isSearchActive else {
            isLoadingSectionPage = false
            return
        }
        guard let kind = sectionPageKind(for: selectedSection) else {
            isLoadingSectionPage = false
            return
        }
        isLoadingSectionPage = loadingSectionKinds.contains(kind)
    }

    private func isSelectedCategoryCatalogPage(_ kind: SectionPageKind) -> Bool {
        guard case .catalog(let category?, _) = kind else {
            return false
        }
        return selectedSection.categoryIdentifier == category
    }

    private func mergeOutdatedState(into record: PackageRecord) -> PackageRecord {
        if let outdated = snapshot.outdatedPackagesByName[record.name] {
            return record.applying(outdated: outdated)
        }
        return record
    }

    private func installedDetail(
        for record: PackageRecord,
        fallback: PackageDetail? = nil
    ) -> PackageDetail {
        if let cached = detailsByPackageName[record.name],
           !isStaleSecurityDetail(cached, for: record) {
            return cached
        }
        if let cached = detailsByPackageName[record.name],
           isStaleSecurityDetail(cached, for: record) {
            detailsByPackageName[record.name] = nil
        }
        if let fallback, !isStaleSecurityDetail(fallback, for: record) {
            return fallback
        }
        return record.fallbackDetail
    }

    private func isStaleSecurityDetail(
        _ detail: PackageDetail,
        for record: PackageRecord
    ) -> Bool {
        guard record.securityState?.needsMainWindowSecurityAlert != true else {
            return false
        }
        return detail.securityState?.needsMainWindowSecurityAlert == true
            || detail.packageName.isLocalDetectorDisplayPackageName
            || detail.qualifiedName.isLocalDetectorDisplayPackageName
    }

    private func retireCompletedHardening(_ request: PackageOperationRequest) {
        guard let context = PackageHardeningContext(request: request) else {
            return
        }

        let selectedWasRemediated = selectedPackage.map(context.matches) ?? false
        let immediateSelectionID = selectedWasRemediated
            ? packages.first(where: context.matches)?.selectionID
            : nil

        packages = packages.retiringSecurityReview(matching: context)
        securityRecommendationPackages = securityRecommendationPackages.retiringSecurityReview(
            matching: context
        )
        catalogPackages = catalogPackages.retiringSecurityReview(matching: context)
        categoryPackagesByPageKey = categoryPackagesByPageKey.mapValues {
            $0.retiringSecurityReview(matching: context)
        }
        pulsePackages = pulsePackages.retiringSecurityReview(matching: context)
        searchResults = searchResults.retiringSecurityReview(matching: context)
        geigerPackages.removeAll { context.matches($0) }
        staleSectionKinds.insert(.securityRecommendations)
        staleSectionKinds.insert(.geiger)

        detailsByPackageName = detailsByPackageName.filter { key, detail in
            !context.matches(key: key) && !context.matches(detail)
        }
        snapshot = snapshotWithHazardousPackageCount(geigerActionPackages.count)

        guard selectedWasRemediated else {
            return
        }

        pendingHardeningSelection = context
        if let immediateSelectionID {
            selectedItemID = immediateSelectionID
            loadSelectedDetailIfPossible()
        } else {
            selectedItemID = nil
            detailRequestID += 1
            isLoadingDetail = false
        }
    }

    @discardableResult
    private func selectPendingHardeningPackageIfPossible() -> Bool {
        guard let context = pendingHardeningSelection,
              let package = packages.first(where: context.matches) else {
            return false
        }
        pendingHardeningSelection = nil
        selectedItemID = package.selectionID
        loadSelectedDetailIfPossible()
        return true
    }

    private func snapshotWithHazardousPackageCount(
        _ hazardousPackageCount: Int
    ) -> NucleusStatusSnapshot {
        NucleusStatusSnapshot(
            installedCount: snapshot.installedCount,
            hazardousPackageCount: hazardousPackageCount,
            outdatedPackages: snapshot.outdatedPackages,
            refreshedAt: snapshot.refreshedAt,
            lastError: snapshot.lastError,
            remoteDatabaseRefreshState: snapshot.remoteDatabaseRefreshState
        )
    }

    private func githubRepositoryURL(from url: URL?) -> URL? {
        guard let url,
              url.host?.localizedCaseInsensitiveCompare("github.com") == .orderedSame else {
            return nil
        }
        let components = url.path
            .split(separator: "/", omittingEmptySubsequences: true)
            .prefix(2)
            .map(String.init)
        guard components.count == 2 else {
            return nil
        }
        return URL(string: "https://github.com/\(components[0])/\(components[1])")
    }

    private func documentationURL(from url: URL?) -> URL? {
        guard let url else {
            return nil
        }
        if let repository = githubRepositoryURL(from: url) {
            return repository.appendingPathComponent("wiki")
        }
        return url
    }

    private func strippedPackagePrefix(_ name: String) -> String {
        for prefix in ["brew:", "cask:", "isotope:", "npm:", "pip:", "sys:", "gone:"] {
            if let stripped = name.strippingPrefix(prefix), stripped.isEmpty == false {
                return stripped
            }
        }
        return name
    }

    private var localOutdatedPackages: [PackagePresentation] {
        guard let recommendation = automicVaultCLTRecommendation,
              recommendation.isInstalled,
              recommendation.isOutdated else {
            return []
        }
        return [
            PackagePresentation(
                item: .recommendation(recommendation),
                detail: recommendation.detail,
                freshness: Self.freshness(for: recommendation.packageName)
            )
        ]
    }

    private nonisolated static func fetchInstalledPackages() throws -> [PackageRecord] {
        let bridge = NucleusBridge(
            compatibilityPolicy: .protocolOnly,
            daemonOwnership: .owner
        )
        return try bridge.fetchPackages().sorted {
            let left = $0.name.packageSearchOrderName
            let right = $1.name.packageSearchOrderName
            if left == right {
                return $0.name < $1.name
            }
            return left < right
        }
    }

    private nonisolated static func fetchOutdatedPackages() throws -> [OutdatedPackageRecord] {
        try NucleusBridge(
            compatibilityPolicy: .protocolOnly,
            daemonOwnership: .owner
        )
            .fetchOutdatedPackages()
    }

    private func sectionPageFetcher(
        for kind: SectionPageKind
    ) -> (Int, Int) throws -> PackageSearchPage {
        switch kind {
        case .securityRecommendations:
            return securityRecommendationPackagesFetcher
        case .geiger:
            return geigerPackagesFetcher
        case .catalog(let category, let sortOrder):
            let availablePackagesFetcher = availablePackagesFetcher
            return { offset, limit in
                try availablePackagesFetcher(offset, limit, category, sortOrder)
            }
        case .pulse:
            return pulsePackagesFetcher
        }
    }

    private nonisolated static func fetchAvailablePackages(
        offset: Int,
        limit: Int,
        category: String? = nil,
        sortOrder: CategoryPackageSortOrder = .rank
    ) throws -> PackageSearchPage {
        let bridge = NucleusBridge(
            compatibilityPolicy: .protocolOnly,
            daemonOwnership: .owner
        )
        return try bridge.fetchAvailablePackages(
            offset: offset,
            limit: limit,
            category: category,
            sortOrder: sortOrder
        )
    }

    private nonisolated static func fetchPulsePackages(
        offset: Int,
        limit: Int
    ) throws -> PackageSearchPage {
        try NucleusBridge(
            compatibilityPolicy: .protocolOnly,
            daemonOwnership: .owner
        )
        .fetchPulsePackages(offset: offset, limit: limit)
    }

    private nonisolated static func fetchGeigerPackages(
        offset: Int,
        limit: Int
    ) throws -> PackageSearchPage {
        try NucleusBridge(
            compatibilityPolicy: .protocolOnly,
            daemonOwnership: .owner
        )
        .fetchGeigerPackages(offset: offset, limit: limit)
    }

    private nonisolated static func fetchSecurityRecommendationPackages(
        offset: Int,
        limit: Int
    ) throws -> PackageSearchPage {
        try NucleusBridge(
            compatibilityPolicy: .protocolOnly,
            daemonOwnership: .owner
        )
        .fetchSecurityRecommendationPackages(offset: offset, limit: limit)
    }

    private nonisolated static func fetchDetail(packageName: String) throws -> PackageDetail {
        try NucleusBridge(
            compatibilityPolicy: .protocolOnly,
            daemonOwnership: .owner
        )
            .fetchDetail(packageName: packageName)
    }

    private nonisolated static func searchPackages(
        query: String,
        offset: Int,
        limit: Int
    ) throws -> PackageSearchPage {
        try NucleusBridge(
            compatibilityPolicy: .protocolOnly,
            daemonOwnership: .owner
        )
            .fetchSearchResults(query: query, offset: offset, limit: limit)
    }

    private nonisolated static func freshness(for packageName: String) -> CGFloat {
        let hash = CGFloat(abs(packageName.hashValue % 1000)) / 1000
        return 0.28 + hash * 0.72
    }

    private static let iso8601Formatter = ISO8601DateFormatter()

    private static let fractionalISO8601Formatter: ISO8601DateFormatter = {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return formatter
    }()

    private static let relativeFormatter: RelativeDateTimeFormatter = {
        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .full
        return formatter
    }()

    private static func parseISO8601Date(_ raw: String) -> Date? {
        iso8601Formatter.date(from: raw)
            ?? fractionalISO8601Formatter.date(from: raw)
    }

    private static func relativeAgeText(
        for date: Date,
        relativeTo referenceDate: Date
    ) -> String {
        let elapsed = referenceDate.timeIntervalSince(date)
        if elapsed >= 0, elapsed < 60 * 60 * 60 {
            let hours = Int(elapsed / 3600)
            if hours < 1 {
                return L10n.string("less than 1 hour ago")
            }
            return hours == 1
                ? L10n.string("1 hour ago")
                : L10n.format("%d hours ago", hours)
        }
        return relativeFormatter.localizedString(for: date, relativeTo: referenceDate)
    }

    private static func packageCountText(_ count: Int) -> String {
        count == 1
            ? L10n.string("1 outdated package")
            : L10n.format("%d outdated packages", count)
    }
}

private extension Array where Element == PackagePresentation {
    func appendingUniquePackages(
        _ packages: [PackagePresentation]
    ) -> [PackagePresentation] {
        var seen = Set(map(\.selectionID))
        var result = self
        for package in packages where seen.insert(package.selectionID).inserted {
            result.append(package)
        }
        return result
    }

    func updatingDetail(
        _ detail: PackageDetail,
        for selectionID: String
    ) -> [PackagePresentation] {
        map { package in
            guard package.selectionID == selectionID else {
                return package
            }
            return PackagePresentation(
                item: package.item,
                detail: detail,
                freshness: package.freshness,
                presentationID: package.presentationID
            )
        }
    }

    func retiringSecurityReview(
        matching context: PackageHardeningContext
    ) -> [PackagePresentation] {
        map { package in
            guard context.matches(package) else {
                return package
            }

            let item: PackageListItem
            switch package.item {
            case .installed(let record):
                item = .installed(record.clearingSecurityState())
            case .available(let result):
                item = .available(result.clearingSecurityState())
            case .recommendation(let recommendation):
                item = .recommendation(recommendation.clearingSecurityState())
            case .command:
                item = package.item
            }

            return PackagePresentation(
                item: item,
                detail: package.detail?.clearingSecurityState(),
                freshness: package.freshness,
                presentationID: package.presentationID
            )
        }
    }
}

private struct PackageHardeningContext {
    private let identifiers: Set<String>

    init?(request: PackageOperationRequest) {
        guard request.kind == .harden else {
            return nil
        }

        var identifiers = Set<String>()
        for packageName in request.packageNames {
            Self.insert(packageName, into: &identifiers)
        }
        Self.insert(request.migrationIsotopeName, into: &identifiers)

        guard identifiers.isEmpty == false else {
            return nil
        }
        self.identifiers = identifiers
    }

    func matches(_ package: PackagePresentation) -> Bool {
        if let detail = package.detail,
           matches(detail) {
            return true
        }

        switch package.item {
        case .installed(let record):
            return matches(record)
        case .available(let result):
            return matches(result)
        case .recommendation(let recommendation):
            return matches(recommendation.detail)
                || matches(key: recommendation.packageName)
                || recommendation.missingPackageNames.contains(where: matches(key:))
        case .command:
            return false
        }
    }

    func matches(_ detail: PackageDetail) -> Bool {
        matches(detail.securityState)
            || matches(key: detail.packageName)
            || matches(key: detail.qualifiedName)
            || matches(detail.source)
            || detail.installPackageNames?.contains(where: matches(key:)) == true
    }

    func matches(key: String?) -> Bool {
        guard let identifier = Self.normalizedIdentifier(key) else {
            return false
        }
        return identifiers.contains(identifier)
    }

    private func matches(_ record: PackageRecord) -> Bool {
        matches(record.securityState)
            || matches(key: record.name)
            || matches(record.source)
            || record.installPackageNames?.contains(where: matches(key:)) == true
    }

    private func matches(_ result: PackageSearchResult) -> Bool {
        matches(result.securityState)
            || matches(key: result.name)
            || matches(result.source)
    }

    private func matches(_ securityState: PackageSecurityState?) -> Bool {
        guard let securityState else {
            return false
        }
        return matches(key: securityState.isotopeName)
    }

    private func matches(_ source: PackageSource?) -> Bool {
        switch source {
        case .formula(let rootFormula):
            return matches(key: rootFormula)
        case .cask(let caskName):
            return matches(key: caskName)
        case .isotope(let isotopeName):
            return matches(key: isotopeName)
        case .vendor(let vendorName):
            return matches(key: vendorName)
        case .npm(let packageName):
            return matches(key: packageName)
        case .pip(let packageName):
            return matches(key: packageName)
        case .none:
            return false
        }
    }

    private static func insert(_ value: String?, into identifiers: inout Set<String>) {
        guard let identifier = normalizedIdentifier(value) else {
            return
        }
        identifiers.insert(identifier)
    }

    private static func normalizedIdentifier(_ value: String?) -> String? {
        guard let value else {
            return nil
        }
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.isEmpty == false else {
            return nil
        }

        let orderedName = trimmed.packageSearchOrderName
        let leafName = orderedName
            .split(separator: "/", omittingEmptySubsequences: true)
            .last
            .map(String.init)
            ?? orderedName
        let normalized = leafName.lowercased()
        return normalized.isEmpty ? nil : normalized
    }
}
