import Foundation

struct NucleusStatusSnapshot: Codable, Equatable {
    struct ErrorSnapshot: Codable, Equatable {
        let message: String
        let refreshedAt: Date
    }

    enum RemoteDatabaseRefreshState: String, Codable {
        case normal
        case pendingHelperInstallation
    }

    let installedCount: Int
    let hazardousPackageCount: Int
    let outdatedPackages: [OutdatedPackageRecord]
    let refreshedAt: Date
    let lastError: ErrorSnapshot?
    let remoteDatabaseRefreshState: RemoteDatabaseRefreshState

    static let empty = NucleusStatusSnapshot(
        installedCount: 0,
        hazardousPackageCount: 0,
        outdatedPackages: [],
        refreshedAt: .distantPast,
        lastError: nil,
        remoteDatabaseRefreshState: .normal
    )

    var outdatedPackagesByName: [String: OutdatedPackageRecord] {
        Dictionary(uniqueKeysWithValues: outdatedPackages.map { ($0.name, $0) })
    }

    enum CodingKeys: String, CodingKey {
        case installedCount
        case hazardousPackageCount
        case outdatedPackages
        case refreshedAt
        case lastError
        case remoteDatabaseRefreshState
    }

    init(
        installedCount: Int,
        hazardousPackageCount: Int,
        outdatedPackages: [OutdatedPackageRecord],
        refreshedAt: Date,
        lastError: ErrorSnapshot?,
        remoteDatabaseRefreshState: RemoteDatabaseRefreshState = .normal
    ) {
        self.installedCount = installedCount
        self.hazardousPackageCount = hazardousPackageCount
        self.outdatedPackages = outdatedPackages
        self.refreshedAt = refreshedAt
        self.lastError = lastError
        self.remoteDatabaseRefreshState = remoteDatabaseRefreshState
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        installedCount = try container.decode(Int.self, forKey: .installedCount)
        hazardousPackageCount = try container.decodeIfPresent(
            Int.self,
            forKey: .hazardousPackageCount
        ) ?? 0
        outdatedPackages = try container.decode(
            [OutdatedPackageRecord].self,
            forKey: .outdatedPackages
        )
        refreshedAt = try container.decode(Date.self, forKey: .refreshedAt)
        lastError = try container.decodeIfPresent(
            ErrorSnapshot.self,
            forKey: .lastError
        )
        remoteDatabaseRefreshState = try container.decodeIfPresent(
            RemoteDatabaseRefreshState.self,
            forKey: .remoteDatabaseRefreshState
        ) ?? .normal
    }

    func withRemoteDatabaseRefreshState(
        _ state: RemoteDatabaseRefreshState
    ) -> NucleusStatusSnapshot {
        NucleusStatusSnapshot(
            installedCount: installedCount,
            hazardousPackageCount: hazardousPackageCount,
            outdatedPackages: outdatedPackages,
            refreshedAt: refreshedAt,
            lastError: lastError,
            remoteDatabaseRefreshState: state
        )
    }
}

struct StartAtLoginSnapshot: Codable, Equatable {
    enum Status: String, Codable {
        case disabled
        case enabled
        case notFound
        case requiresApproval
        case unavailable
    }

    let status: Status
    let updatedAt: Date
    let lastError: String?

    static let unavailable = StartAtLoginSnapshot(
        status: .unavailable,
        updatedAt: .distantPast,
        lastError: nil
    )
}

struct AppUpdateSnapshot: Codable, Equatable {
    let updateAvailable: Bool
    let updatedAt: Date
    let lastError: String?

    static let empty = AppUpdateSnapshot(
        updateAvailable: false,
        updatedAt: .distantPast,
        lastError: nil
    )
}

enum NucleusStatusNotification {
    static let snapshotDidChange = Notification.Name(
        "com.automicvault.nucleus-status.snapshot-did-change"
    )
    static let refreshRequested = Notification.Name(
        "com.automicvault.nucleus-status.refresh-requested"
    )
    static let openMainWindowRequested = Notification.Name(
        "com.automicvault.nucleus-status.open-main-window-requested"
    )
    static let startAtLoginToggleRequested = Notification.Name(
        "com.automicvault.nucleus-status.start-at-login-toggle-requested"
    )
    static let startAtLoginDidChange = Notification.Name(
        "com.automicvault.nucleus-status.start-at-login-did-change"
    )
    static let appUpdateDidChange = Notification.Name(
        "com.automicvault.nucleus-status.app-update-did-change"
    )
}

final class NucleusStatusStore {
    private let encoder = JSONEncoder()
    private let decoder = JSONDecoder()
    private let fileManager = FileManager.default
    private let distributedCenter = DistributedNotificationCenter.default()

    init() {
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        encoder.dateEncodingStrategy = .iso8601
        decoder.dateDecodingStrategy = .iso8601
    }

    func loadSnapshot() -> NucleusStatusSnapshot {
        guard let data = try? Data(contentsOf: snapshotURL()),
              let snapshot = try? decoder.decode(NucleusStatusSnapshot.self, from: data) else {
            return .empty
        }
        return snapshot
    }

    func saveSnapshot(_ snapshot: NucleusStatusSnapshot) throws {
        let url = snapshotURL()
        try fileManager.createDirectory(
            at: url.deletingLastPathComponent(),
            withIntermediateDirectories: true,
            attributes: nil
        )
        let data = try encoder.encode(snapshot)
        try data.write(to: url, options: .atomic)
        post(NucleusStatusNotification.snapshotDidChange)
    }

    func saveRemoteDatabaseRefreshState(
        _ state: NucleusStatusSnapshot.RemoteDatabaseRefreshState
    ) throws {
        let snapshot = loadSnapshot()
        guard snapshot.remoteDatabaseRefreshState != state else {
            return
        }
        try saveSnapshot(snapshot.withRemoteDatabaseRefreshState(state))
    }

    func loadStartAtLoginSnapshot() -> StartAtLoginSnapshot {
        guard let data = try? Data(contentsOf: startAtLoginURL()),
              let snapshot = try? decoder.decode(StartAtLoginSnapshot.self, from: data) else {
            return .unavailable
        }
        return snapshot
    }

    func saveStartAtLoginSnapshot(_ snapshot: StartAtLoginSnapshot) throws {
        let url = startAtLoginURL()
        try fileManager.createDirectory(
            at: url.deletingLastPathComponent(),
            withIntermediateDirectories: true,
            attributes: nil
        )
        let data = try encoder.encode(snapshot)
        try data.write(to: url, options: .atomic)
        post(NucleusStatusNotification.startAtLoginDidChange)
    }

    func loadAppUpdateSnapshot() -> AppUpdateSnapshot {
        guard let data = try? Data(contentsOf: appUpdateURL()),
              let snapshot = try? decoder.decode(AppUpdateSnapshot.self, from: data) else {
            return .empty
        }
        return snapshot
    }

    func saveAppUpdateSnapshot(_ snapshot: AppUpdateSnapshot) throws {
        let url = appUpdateURL()
        try fileManager.createDirectory(
            at: url.deletingLastPathComponent(),
            withIntermediateDirectories: true,
            attributes: nil
        )
        let data = try encoder.encode(snapshot)
        try data.write(to: url, options: .atomic)
        post(NucleusStatusNotification.appUpdateDidChange)
    }

    func observeSnapshotChanges(
        using block: @escaping (Notification) -> Void
    ) -> NSObjectProtocol {
        distributedCenter.addObserver(
            forName: NucleusStatusNotification.snapshotDidChange,
            object: nil,
            queue: .main,
            using: block
        )
    }

    func observeRefreshRequests(
        using block: @escaping (Notification) -> Void
    ) -> NSObjectProtocol {
        distributedCenter.addObserver(
            forName: NucleusStatusNotification.refreshRequested,
            object: nil,
            queue: .main,
            using: block
        )
    }

    func observeOpenMainWindowRequests(
        using block: @escaping (Notification) -> Void
    ) -> NSObjectProtocol {
        distributedCenter.addObserver(
            forName: NucleusStatusNotification.openMainWindowRequested,
            object: nil,
            queue: .main,
            using: block
        )
    }

    func observeStartAtLoginToggleRequests(
        using block: @escaping (Notification) -> Void
    ) -> NSObjectProtocol {
        distributedCenter.addObserver(
            forName: NucleusStatusNotification.startAtLoginToggleRequested,
            object: nil,
            queue: .main,
            using: block
        )
    }

    func observeStartAtLoginChanges(
        using block: @escaping (Notification) -> Void
    ) -> NSObjectProtocol {
        distributedCenter.addObserver(
            forName: NucleusStatusNotification.startAtLoginDidChange,
            object: nil,
            queue: .main,
            using: block
        )
    }

    func observeAppUpdateChanges(
        using block: @escaping (Notification) -> Void
    ) -> NSObjectProtocol {
        distributedCenter.addObserver(
            forName: NucleusStatusNotification.appUpdateDidChange,
            object: nil,
            queue: .main,
            using: block
        )
    }

    func requestRefresh() {
        post(NucleusStatusNotification.refreshRequested)
    }

    func requestOpenMainWindow() {
        post(NucleusStatusNotification.openMainWindowRequested)
    }

    func requestStartAtLoginToggle() {
        post(NucleusStatusNotification.startAtLoginToggleRequested)
    }

    func notifyStartAtLoginChanged() {
        post(NucleusStatusNotification.startAtLoginDidChange)
    }

    private func post(_ name: Notification.Name) {
        distributedCenter.postNotificationName(
            name,
            object: nil,
            userInfo: nil,
            deliverImmediately: true
        )
    }

    private func snapshotURL() -> URL {
        applicationSupportURL()
            .appendingPathComponent(
                "menu-helper-status.json",
                isDirectory: false
            )
    }

    private func startAtLoginURL() -> URL {
        applicationSupportURL()
            .appendingPathComponent("start-at-login-status.json", isDirectory: false)
    }

    private func appUpdateURL() -> URL {
        applicationSupportURL()
            .appendingPathComponent("app-update-status.json", isDirectory: false)
    }

    private func applicationSupportURL() -> URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(
                "Library/Application Support/Automic Vault",
                isDirectory: true
            )
    }
}
