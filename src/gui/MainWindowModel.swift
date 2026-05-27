import AppKit
import Combine
import Foundation

enum MainWindowSection: String, CaseIterable, Identifiable {
    case installed
    case geigerCounter
    case newUpdated
    case outdated
    case allPackages
    case shell
    case cliTools
    case development
    case system
    case networking
    case security
    case other
    case settings
    case about

    var id: String { rawValue }

    static let librarySections: [MainWindowSection] = [
        .installed,
        .geigerCounter,
        .newUpdated,
        .outdated,
        .allPackages
    ]

    static let categorySections: [MainWindowSection] = [
        .shell,
        .cliTools,
        .development,
        .system,
        .networking,
        .security,
        .other
    ]

    static let utilitySections: [MainWindowSection] = [
        .settings,
        .about
    ]

    var title: String {
        switch self {
        case .installed:
            return "Installed"
        case .geigerCounter:
            return "Geiger Counter"
        case .newUpdated:
            return "New / Updated"
        case .outdated:
            return "Outdated"
        case .allPackages:
            return "All Packages"
        case .shell:
            return "Shell"
        case .cliTools:
            return "CLI Tools"
        case .development:
            return "Development"
        case .system:
            return "System"
        case .networking:
            return "Networking"
        case .security:
            return "Security"
        case .other:
            return "Other"
        case .settings:
            return "Settings"
        case .about:
            return "About"
        }
    }

    var systemImage: String {
        switch self {
        case .installed:
            return "shippingbox"
        case .geigerCounter:
            return "dot.radiowaves.left.and.right"
        case .newUpdated:
            return "sparkles"
        case .outdated:
            return "clock"
        case .allPackages:
            return "cube"
        case .shell:
            return "terminal"
        case .cliTools:
            return "chevron.left.forwardslash.chevron.right"
        case .development:
            return "hammer"
        case .system:
            return "gearshape"
        case .networking:
            return "network"
        case .security:
            return "shield"
        case .other:
            return "ellipsis"
        case .settings:
            return "gear"
        case .about:
            return "info.circle"
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
            return "Homepage"
        case .repository:
            return "Repository"
        case .documentation:
            return "Documentation"
        }
    }
}

enum MainWindowPackageBadge {
    case vulnerable
    case hardened
    case immutable
}

@MainActor
final class MainWindowModel: ObservableObject {
    @Published var selectedSection: MainWindowSection = .installed {
        didSet {
            ensureSelectedSectionLoaded()
        }
    }
    @Published var searchText = "" {
        didSet {
            scheduleSearch()
        }
    }
    @Published private(set) var packages: [PackagePresentation] = []
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

    private struct InitialDaemonData {
        let installed: [PackageRecord]
        let outdated: [OutdatedPackageRecord]
    }

    nonisolated private static let pageSize = 96
    private let statusStore = NucleusStatusStore()
    private var snapshotObserver: NSObjectProtocol?
    private var reloadRequestID = 0
    private var searchRequestID = 0
    private var sectionPageRequestID = 0
    private var detailRequestID = 0
    private var detailsByPackageName: [String: PackageDetail] = [:]
    private var geigerTotalCount: Int?
    private var catalogTotalCount: Int?
    private var pulseTotalCount: Int?
    private var searchTotalCount = 0
    private var loadingSectionKind: SectionPageKind?
    private var transientStatusTask: Task<Void, Never>?
    private var searchTask: Task<Void, Never>?
    private var sectionPageTask: Task<Void, Never>?

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
        sectionPageTask?.cancel()
    }

    func reloadPackages() {
        reloadRequestID += 1
        let requestID = reloadRequestID
        isReloading = true
        lastErrorMessage = nil
        statusMessage = "Loading packages from the protocol daemon"

        Task.detached(priority: .userInitiated) {
            let result = Result { try Self.fetchInitialDaemonData() }
            await MainActor.run {
                self.finishReload(result, requestID: requestID)
            }
        }
        statusStore.requestRefresh()
    }

    func select(_ package: PackagePresentation) {
        selectedItemID = package.selectionID
        loadDetail(for: package)
    }

    func requestSearchFocus() {
        searchFocusRequestID += 1
    }

    func selectedURL(for tab: MainWindowLinkTab) -> URL? {
        guard let detail = selectedDetail else {
            return nil
        }
        switch tab {
        case .homepage:
            return detail.homepageURL
        case .repository:
            return githubRepositoryURL(from: detail.homepageURL)
        case .documentation:
            return documentationURL(from: detail.homepageURL)
        }
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

    func count(for section: MainWindowSection) -> Int? {
        switch section {
        case .installed:
            return installedCount
        case .geigerCounter:
            return geigerCounterCount
        case .newUpdated:
            return pulseTotalCount ?? (pulsePackages.isEmpty ? nil : pulsePackages.count)
        case .outdated:
            return max(outdatedPackageNames.count, snapshot.flaggedOutdatedPackageCount)
        case .allPackages:
            return catalogTotalCount ?? (catalogPackages.isEmpty ? nil : catalogPackages.count)
        case .settings, .about:
            return nil
        case .shell, .cliTools, .development, .system, .networking, .security, .other:
            guard catalogPackages.isEmpty == false else {
                return nil
            }
            return catalogSourcePackages.filter { package in
                sectionMatches(section, package: package)
            }.count
        }
    }

    func packageBadge(for package: PackagePresentation) -> MainWindowPackageBadge? {
        if isInstalledAsIsotope(package) {
            return .hardened
        }
        if needsHardening(package) {
            return .vulnerable
        }
        if isInstalledAsRoot(package) {
            return .immutable
        }
        return nil
    }

    private func needsHardening(_ package: PackagePresentation) -> Bool {
        let detail = detailsByPackageName[package.selectionID] ?? package.detail
        if detail?.securityState?.installIsInsecure == true
            || detail?.securityState?.error?
                .trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
            || detail?.securityNotice != nil
            || package.hasActivePlainTextSecretAlert {
            return true
        }
        return false
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

    func isHardened(_ package: PackagePresentation) -> Bool {
        if package.isInstalledIsotope {
            return true
        }
        let detail = detailsByPackageName[package.selectionID] ?? package.detail
        if case .isotope = detail?.source {
            return true
        }
        return detail?.securityState != nil && detail?.securityState?.installIsInsecure != true
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
        return package.listSecondaryText
    }

    func versionText(for package: PackagePresentation) -> String {
        switch package.item {
        case .installed(let record):
            return record.version
        case .recommendation, .available, .command:
            return package.versionText
        }
    }

    func relativeLastUpdatedText(for detail: PackageDetail?) -> String {
        guard let raw = detail?.lastUpdatedAt,
              let date = Self.iso8601Formatter.date(from: raw) else {
            return relativeRefreshText
        }
        return Self.relativeFormatter.localizedString(for: date, relativeTo: Date())
    }

    var relativeRefreshText: String {
        guard snapshot.refreshedAt > .distantPast else {
            return "Not yet refreshed"
        }
        return Self.relativeFormatter.localizedString(
            for: snapshot.refreshedAt,
            relativeTo: Date()
        )
    }

    var outdatedPackageNames: Set<String> {
        Set(snapshot.outdatedPackages.map(\.name))
            .union(snapshot.homebrewOutdatedPackages.map(\.name))
    }

    private var allKnownPackages: [PackagePresentation] {
        var seen = Set<String>()
        var result: [PackagePresentation] = []
        for package in packages + geigerPackages + catalogPackages + pulsePackages + searchResults {
            if seen.insert(package.selectionID).inserted {
                result.append(package)
            }
        }
        return result
    }

    private var geigerCounterCount: Int? {
        let knownActionableCount = geigerActionPackages.count
        if let geigerTotalCount {
            return max(geigerTotalCount, knownActionableCount)
        }
        let fallbackCount = max(snapshot.hazardousPackageCount, knownActionableCount)
        return fallbackCount > 0 ? fallbackCount : nil
    }

    private var geigerActionPackages: [PackagePresentation] {
        uniquePackages(packages.filter(isGeigerActionPackage) + geigerPackages)
    }

    private var catalogSourcePackages: [PackagePresentation] {
        catalogPackages
    }

    private func uniquePackages(_ source: [PackagePresentation]) -> [PackagePresentation] {
        var seen = Set<String>()
        var result: [PackagePresentation] = []
        for package in source where seen.insert(package.selectionID).inserted {
            result.append(package)
        }
        return result
    }

    private func packages(for section: MainWindowSection) -> [PackagePresentation] {
        guard section != .settings, section != .about else {
            return []
        }
        let query = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        if query.isEmpty == false {
            return mergedSearchPackages(query: query)
        }

        let source: [PackagePresentation]
        switch section {
        case .installed:
            source = packages
        case .geigerCounter:
            source = geigerActionPackages
        case .newUpdated:
            source = pulsePackages
        case .outdated:
            source = packages.filter(isOutdated)
        case .allPackages:
            source = catalogSourcePackages
        case .shell, .cliTools, .development, .system, .networking, .security, .other:
            source = catalogSourcePackages
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
        var seen = Set(installedMatches.map(\.selectionID))
        let daemonMatches = searchResults.filter { package in
            seen.insert(package.selectionID).inserted
        }
        return installedMatches + daemonMatches
    }

    private func sectionMatches(
        _ section: MainWindowSection,
        package: PackagePresentation
    ) -> Bool {
        switch section {
        case .installed, .allPackages:
            return true
        case .geigerCounter:
            return isGeigerActionPackage(package)
        case .newUpdated:
            return true
        case .outdated:
            return isOutdated(package)
        case .shell:
            return packageName(package, containsAny: [
                "bash", "zsh", "fish", "shell", "shfmt", "shellcheck", "starship"
            ])
        case .cliTools:
            return !isDevelopment(package)
                && !isNetworking(package)
                && !isSecurity(package)
                && !isSystem(package)
        case .development:
            return isDevelopment(package)
        case .system:
            return isSystem(package)
        case .networking:
            return isNetworking(package)
        case .security:
            return isSecurity(package)
        case .other:
            return !isDevelopment(package)
                && !isNetworking(package)
                && !isSecurity(package)
                && !isSystem(package)
                && !packageName(package, containsAny: [
                    "bash", "zsh", "fish", "shell", "shfmt", "shellcheck", "starship"
                ])
        case .settings, .about:
            return false
        }
    }

    private func isGeigerActionPackage(_ package: PackagePresentation) -> Bool {
        packageBadge(for: package) == .vulnerable
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
        if let detail = detailsByPackageName[package.selectionID] ?? package.detail {
            return detail.isOutdated
        }
        guard let name = package.packageName else {
            return false
        }
        return outdatedPackageNames.contains(name)
    }

    private func isDevelopment(_ package: PackagePresentation) -> Bool {
        if case .npm = package.detail?.source {
            return true
        }
        if case .pip = package.detail?.source {
            return true
        }
        return packageName(package, containsAny: [
            "git", "node", "python", "ruby", "go", "rust", "swift", "cmake", "make",
            "llvm", "gcc", "cargo", "npm", "pnpm", "yarn", "deno", "bun"
        ])
    }

    private func isNetworking(_ package: PackagePresentation) -> Bool {
        packageName(package, containsAny: [
            "curl", "wget", "http", "ssh", "openssl", "nginx", "dns", "net", "proxy",
            "tailscale", "wireguard"
        ])
    }

    private func isSecurity(_ package: PackagePresentation) -> Bool {
        package.hasPlainTextSecretAlert
            || package.detail?.securityState != nil
            || packageName(package, containsAny: [
                "vault", "secret", "token", "key", "pass", "gpg", "age", "sops",
                "security", "cert"
            ])
    }

    private func isSystem(_ package: PackagePresentation) -> Bool {
        guard let name = package.packageName else {
            return false
        }
        return name.hasPrefix("sys:")
            || packageName(package, containsAny: [
                "coreutils", "findutils", "grep", "sed", "awk", "pkgconf", "system"
            ])
    }

    private func packageName(
        _ package: PackagePresentation,
        containsAny needles: [String]
    ) -> Bool {
        let haystack = [
            package.displayName,
            package.packageName ?? "",
            package.listSecondaryText
        ].joined(separator: " ").lowercased()
        return needles.contains { haystack.contains($0) }
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
            && snapshot.homebrewOutdatedPackages.isEmpty
            && packages.isEmpty == false
        self.snapshot = shouldKeepDaemonSnapshot
            ? self.snapshot.withRemoteDatabaseRefreshState(snapshot.remoteDatabaseRefreshState)
            : snapshot
        packages = packages.map { package in
            guard case .installed(let record) = package.item else {
                return package
            }
            let merged = mergeOutdatedState(into: record)
            let detail = detailsByPackageName[package.selectionID] ?? package.detail
            return PackagePresentation(
                item: .installed(merged),
                detail: detail,
                freshness: package.freshness,
                presentationID: package.presentationID
            )
        }
    }

    private func finishReload(
        _ result: Result<InitialDaemonData, Error>,
        requestID: Int
    ) {
        guard requestID == reloadRequestID else {
            return
        }

        isReloading = false
        switch result {
        case .success(let data):
            snapshot = NucleusStatusSnapshot(
                installedCount: data.installed.count,
                hazardousPackageCount: data.installed.filter {
                    $0.securityState?.installIsInsecure == true
                }.count,
                outdatedPackages: data.outdated,
                refreshedAt: Date(),
                lastError: nil,
                remoteDatabaseRefreshState: snapshot.remoteDatabaseRefreshState
            )

            packages = data.installed.map { record in
                let merged = mergeOutdatedState(into: record)
                let detail = detailsByPackageName[merged.name] ?? merged.fallbackDetail
                return PackagePresentation(
                    item: .installed(merged),
                    detail: detail,
                    freshness: Self.freshness(for: merged.name)
                )
            }
            if let selectedItemID,
               allKnownPackages.contains(where: { $0.selectionID == selectedItemID }) {
                loadSelectedDetailIfPossible()
            } else {
                selectedItemID = nil
            }
            statusMessage = nil
            ensureSelectedSectionLoaded()
        case .failure(let error):
            lastErrorMessage = error.localizedDescription
            statusMessage = "Package refresh failed"
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
        if let outdated = homebrewOutdatedPackage(named: detail.packageName) {
            return detail.applying(outdated: outdated)
        }
        return detail
    }

    private func detailLookupName(for package: PackagePresentation) -> String {
        switch package.item {
        case .available(let result):
            return result.detailLookupName
        case .installed, .recommendation, .command:
            return package.packageName ?? package.selectionID
        }
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
            ensureSelectedSectionLoaded()
            return
        }

        searchRequestID += 1
        let requestID = searchRequestID
        isSearching = true
        searchTask = Task { [weak self] in
            try? await Task.sleep(for: .milliseconds(180))
            guard !Task.isCancelled else { return }
            let result = await Task.detached(priority: .userInitiated) {
                Result {
                    try Self.searchPackages(query: query)
                }
            }.value
            await MainActor.run {
                self?.finishSearch(result, query: query, requestID: requestID)
            }
        }
    }

    private func finishSearch(
        _ result: Result<PackageSearchPage, Error>,
        query: String,
        requestID: Int
    ) {
        guard requestID == searchRequestID,
              query == searchText.trimmingCharacters(in: .whitespacesAndNewlines) else {
            return
        }
        isSearching = false
        switch result {
        case .success(let page):
            searchTotalCount = page.totalCount
            searchResults = page.packages.map {
                presentation(for: $0, prefix: "search")
            }
        case .failure(let error):
            searchTotalCount = 0
            searchResults = []
            lastErrorMessage = error.localizedDescription
        }
    }

    private func ensureSelectedSectionLoaded() {
        let query = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard query.isEmpty else {
            return
        }
        switch selectedSection {
        case .geigerCounter:
            loadGeigerPageIfNeeded()
        case .newUpdated:
            loadPulsePageIfNeeded()
        case .allPackages,
             .shell,
             .cliTools,
             .development,
             .system,
             .networking,
             .security,
             .other:
            loadCatalogPageIfNeeded()
        case .installed, .outdated, .settings, .about:
            break
        }
    }

    private func loadGeigerPageIfNeeded() {
        guard geigerPackages.isEmpty, geigerTotalCount == nil else {
            return
        }
        loadSectionPage(kind: .geiger)
    }

    private func loadCatalogPageIfNeeded() {
        guard catalogPackages.isEmpty, catalogTotalCount == nil else {
            return
        }
        loadSectionPage(kind: .catalog)
    }

    private func loadPulsePageIfNeeded() {
        guard pulsePackages.isEmpty, pulseTotalCount == nil else {
            return
        }
        loadSectionPage(kind: .pulse)
    }

    private enum SectionPageKind: Sendable, Equatable {
        case geiger
        case catalog
        case pulse
    }

    private func loadSectionPage(kind: SectionPageKind) {
        guard loadingSectionKind != kind else {
            return
        }
        sectionPageTask?.cancel()
        sectionPageRequestID += 1
        let requestID = sectionPageRequestID
        isLoadingSectionPage = true
        loadingSectionKind = kind
        lastErrorMessage = nil
        sectionPageTask = Task { [weak self] in
            let result = await Task.detached(priority: .userInitiated) {
                Result {
                    try Self.fetchSectionPage(kind: kind)
                }
            }.value
            await MainActor.run {
                self?.finishSectionPage(result, kind: kind, requestID: requestID)
            }
        }
    }

    private func finishSectionPage(
        _ result: Result<PackageSearchPage, Error>,
        kind: SectionPageKind,
        requestID: Int
    ) {
        guard requestID == sectionPageRequestID else {
            return
        }
        isLoadingSectionPage = false
        if loadingSectionKind == kind {
            loadingSectionKind = nil
        }
        switch result {
        case .success(let page):
            switch kind {
            case .geiger:
                geigerTotalCount = page.totalCount
                geigerPackages = page.packages.map { result in
                    result.detectedLocalHazardPresentation(
                        freshness: Self.freshness(for: result.detailLookupName)
                    )?.presentation ?? presentation(for: result, prefix: "geiger")
                }
            case .catalog:
                catalogTotalCount = page.totalCount
                catalogPackages = page.packages.map {
                    presentation(for: $0, prefix: nil)
                }
            case .pulse:
                pulseTotalCount = page.totalCount
                pulsePackages = page.packages.map {
                    presentation(for: $0, prefix: "pulse")
                }
            }
        case .failure(let error):
            lastErrorMessage = error.localizedDescription
        }
    }

    private func mergeOutdatedState(into record: PackageRecord) -> PackageRecord {
        if let outdated = snapshot.outdatedPackagesByName[record.name] {
            return record.applying(outdated: outdated)
        }
        if let outdated = homebrewOutdatedPackage(named: record.name) {
            return record.applying(outdated: outdated)
        }
        return record
    }

    private func homebrewOutdatedPackage(named packageName: String) -> OutdatedPackageRecord? {
        if let package = snapshot.homebrewOutdatedPackagesByName[packageName] {
            return package
        }
        guard let formula = packageName.strippingPrefix("brew:"),
              formula.contains("/") else {
            return nil
        }
        guard let leafName = formula.split(separator: "/").last.map(String.init) else {
            return nil
        }
        return snapshot.homebrewOutdatedPackagesByName["brew:\(leafName)"]
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

    private nonisolated static func fetchInitialDaemonData() throws -> InitialDaemonData {
        let bridge = NucleusBridge(compatibilityPolicy: .protocolOnly)
        let installed = try bridge.fetchPackages().sorted {
            let left = $0.name.packageSearchOrderName
            let right = $1.name.packageSearchOrderName
            if left == right {
                return $0.name < $1.name
            }
            return left < right
        }
        let outdated = (try? bridge.fetchOutdatedPackages()) ?? []
        return InitialDaemonData(
            installed: installed,
            outdated: outdated
        )
    }

    private nonisolated static func fetchSectionPage(
        kind: SectionPageKind
    ) throws -> PackageSearchPage {
        let bridge = NucleusBridge(compatibilityPolicy: .protocolOnly)
        switch kind {
        case .geiger:
            return try bridge.fetchGeigerPackages(offset: 0, limit: pageSize)
        case .catalog:
            return try bridge.fetchAvailablePackages(offset: 0, limit: pageSize)
        case .pulse:
            return try bridge.fetchPulsePackages(offset: 0, limit: pageSize)
        }
    }

    private nonisolated static func fetchDetail(packageName: String) throws -> PackageDetail {
        try NucleusBridge(compatibilityPolicy: .protocolOnly)
            .fetchDetail(packageName: packageName)
    }

    private nonisolated static func searchPackages(query: String) throws -> PackageSearchPage {
        try NucleusBridge(compatibilityPolicy: .protocolOnly)
            .fetchSearchResults(query: query, offset: 0, limit: pageSize)
    }

    private nonisolated static func freshness(for packageName: String) -> CGFloat {
        let hash = CGFloat(abs(packageName.hashValue % 1000)) / 1000
        return 0.28 + hash * 0.72
    }

    private static let iso8601Formatter = ISO8601DateFormatter()

    private static let relativeFormatter: RelativeDateTimeFormatter = {
        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .full
        return formatter
    }()
}

private extension Array where Element == PackagePresentation {
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
}
