import AppKit
import Combine
import Foundation

enum MainWindowSection: String, CaseIterable, Identifiable {
    case installed
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

enum MainWindowRiskLevel {
    case low
    case medium
    case high

    var title: String {
        switch self {
        case .low:
            return "Low"
        case .medium:
            return "Medium"
        case .high:
            return "High"
        }
    }
}

@MainActor
final class MainWindowModel: ObservableObject {
    @Published var selectedSection: MainWindowSection = .installed
    @Published var searchText = ""
    @Published private(set) var packages: [PackagePresentation] = []
    @Published private(set) var snapshot = NucleusStatusSnapshot.empty
    @Published private(set) var selectedItemID: String?
    @Published private(set) var isReloading = false
    @Published private(set) var isLoadingDetail = false
    @Published private(set) var statusMessage: String?
    @Published private(set) var lastErrorMessage: String?

    private let statusStore = NucleusStatusStore()
    private var snapshotObserver: NSObjectProtocol?
    private var reloadRequestID = 0
    private var detailRequestID = 0
    private var detailsByPackageName: [String: PackageDetail] = [:]
    private var transientStatusTask: Task<Void, Never>?

    var installedCount: Int {
        snapshot.installedCount > 0 ? snapshot.installedCount : packages.count
    }

    var selectedPackage: PackagePresentation? {
        guard let selectedItemID else {
            return displayedPackages.first
        }
        return packages.first { $0.selectionID == selectedItemID }
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
    }

    func reloadPackages() {
        reloadRequestID += 1
        let requestID = reloadRequestID
        isReloading = true
        lastErrorMessage = nil
        statusMessage = "Refreshing packages"

        Task.detached(priority: .userInitiated) {
            let result = Result { try Self.fetchInstalledRecords() }
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
        case .newUpdated:
            return max(outdatedPackageNames.count, snapshot.flaggedOutdatedPackageCount)
        case .outdated:
            return max(outdatedPackageNames.count, snapshot.flaggedOutdatedPackageCount)
        case .allPackages:
            return packages.count
        case .settings, .about:
            return nil
        case .shell, .cliTools, .development, .system, .networking, .security, .other:
            return packages.filter { package in
                sectionMatches(section, package: package)
            }.count
        }
    }

    func riskLevel(for package: PackagePresentation) -> MainWindowRiskLevel {
        let detail = detailsByPackageName[package.selectionID] ?? package.detail
        if detail?.securityState?.installIsInsecure == true
            || detail?.securityNotice != nil
            || package.hasActivePlainTextSecretAlert {
            return .high
        }
        if isOutdated(package) {
            return .medium
        }
        return .low
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

    private func packages(for section: MainWindowSection) -> [PackagePresentation] {
        guard section != .settings, section != .about else {
            return []
        }
        let query = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        return packages.filter { package in
            sectionMatches(section, package: package)
                && (query.isEmpty || packageMatchesQuery(package, query: query))
        }
    }

    private func sectionMatches(
        _ section: MainWindowSection,
        package: PackagePresentation
    ) -> Bool {
        switch section {
        case .installed, .allPackages:
            return true
        case .newUpdated, .outdated:
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
        self.snapshot = snapshot
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
        _ result: Result<[PackageRecord], Error>,
        requestID: Int
    ) {
        guard requestID == reloadRequestID else {
            return
        }

        isReloading = false
        switch result {
        case .success(let records):
            packages = records.map { record in
                let merged = mergeOutdatedState(into: record)
                let detail = detailsByPackageName[merged.name] ?? merged.fallbackDetail
                return PackagePresentation(
                    item: .installed(merged),
                    detail: detail,
                    freshness: Self.freshness(for: merged.name)
                )
            }
            if let selectedItemID,
               packages.contains(where: { $0.selectionID == selectedItemID }) {
                loadSelectedDetailIfPossible()
            } else if let first = displayedPackages.first ?? packages.first {
                select(first)
            } else {
                selectedItemID = nil
            }
            statusMessage = nil
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
        guard let packageName = package.packageName else {
            return
        }
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
            let normalized = detail.applying(
                outdated: snapshot.outdatedPackagesByName[detail.packageName]
            )
            detailsByPackageName[package.selectionID] = normalized
            detailsByPackageName[detail.packageName] = normalized
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
        case .failure:
            if let fallback = package.detail {
                detailsByPackageName[package.selectionID] = fallback
            }
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

    private nonisolated static func fetchInstalledRecords() throws -> [PackageRecord] {
        try NucleusBridge().fetchPackages().sorted {
            let left = $0.name.packageSearchOrderName
            let right = $1.name.packageSearchOrderName
            if left == right {
                return $0.name < $1.name
            }
            return left < right
        }
    }

    private nonisolated static func fetchDetail(packageName: String) throws -> PackageDetail {
        try NucleusBridge().fetchDetail(packageName: packageName)
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
