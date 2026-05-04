import Foundation

struct VaultContainmentSessionSnapshot: Codable, Equatable {
    let id: String
    let pid: UInt32
    let agentID: String
    let command: String
    let args: [String]
    let cwd: String
    let initialExecutablePath: String
    let toolchainRoot: String
    let binDir: String
    let sandboxProfilePath: String
    let socketPath: String
    let startedAt: Date

    enum CodingKeys: String, CodingKey {
        case id
        case pid
        case agentID = "agent_id"
        case command
        case args
        case cwd
        case initialExecutablePath = "initial_executable_path"
        case toolchainRoot = "toolchain_root"
        case binDir = "bin_dir"
        case sandboxProfilePath = "sandbox_profile_path"
        case socketPath = "socket_path"
        case startedAt = "started_at"
    }
}

struct VaultContainmentLogEntry: Codable, Equatable, Identifiable {
    enum Kind: String, Codable {
        case sessionStarted = "session_started"
        case command
        case approval
        case completion
        case error
    }

    let id: String
    let kind: Kind
    let createdAt: Date
    let title: String
    let detail: String

    enum CodingKeys: String, CodingKey {
        case id
        case kind
        case createdAt = "created_at"
        case title
        case detail
    }
}

struct VaultContainmentLogSnapshot: Codable, Equatable {
    let session: VaultContainmentSessionSnapshot
    var entries: [VaultContainmentLogEntry]
    var updatedAt: Date

    enum CodingKeys: String, CodingKey {
        case session
        case entries
        case updatedAt = "updated_at"
    }
}

enum ContainmentLogNotification {
    static let changed = Notification.Name(
        "com.automicvault.containment-log.changed"
    )
    static let sessionIDKey = "session_id"
}

final class ContainmentLogStore {
    private let encoder = JSONEncoder()
    private let decoder = JSONDecoder()
    private let fileManager = FileManager.default
    private let distributedCenter = DistributedNotificationCenter.default()
    private let lock = NSLock()

    init() {
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        encoder.dateEncodingStrategy = .iso8601
        decoder.dateDecodingStrategy = .iso8601
    }

    func startSession(_ session: VaultContainmentSessionSnapshot) throws {
        lock.lock()
        defer { lock.unlock() }

        var snapshot = VaultContainmentLogSnapshot(
            session: session,
            entries: [],
            updatedAt: session.startedAt
        )
        snapshot.entries.append(
            entry(
                kind: .sessionStarted,
                title: "Contained entity started",
                detail: ([session.command] + session.args).joined(separator: " "),
                date: session.startedAt
            )
        )
        try write(snapshot, to: logURL(for: session.id))
        postChanged(sessionID: session.id)
    }

    func append(
        sessionID: String,
        kind: VaultContainmentLogEntry.Kind,
        title: String,
        detail: String
    ) throws {
        lock.lock()
        defer { lock.unlock() }

        guard var snapshot = loadUnlocked(sessionID: sessionID) else {
            return
        }
        snapshot.entries.append(
            entry(kind: kind, title: title, detail: detail, date: Date())
        )
        snapshot.updatedAt = Date()
        try write(snapshot, to: logURL(for: sessionID))
        postChanged(sessionID: sessionID)
    }

    func load(sessionID: String) -> VaultContainmentLogSnapshot? {
        lock.lock()
        defer { lock.unlock() }
        return loadUnlocked(sessionID: sessionID)
    }

    func loadRecentSessions(limit: Int = 8) -> [VaultContainmentLogSnapshot] {
        lock.lock()
        defer { lock.unlock() }

        guard let urls = try? fileManager.contentsOfDirectory(
            at: logDirectoryURL(),
            includingPropertiesForKeys: nil
        ) else {
            return []
        }
        return urls
            .compactMap { load(VaultContainmentLogSnapshot.self, from: $0) }
            .sorted { $0.updatedAt > $1.updatedAt }
            .prefix(limit)
            .map { $0 }
    }

    func observeChanges(
        using block: @escaping (Notification) -> Void
    ) -> NSObjectProtocol {
        distributedCenter.addObserver(
            forName: ContainmentLogNotification.changed,
            object: nil,
            queue: .main,
            using: block
        )
    }

    private func entry(
        kind: VaultContainmentLogEntry.Kind,
        title: String,
        detail: String,
        date: Date
    ) -> VaultContainmentLogEntry {
        VaultContainmentLogEntry(
            id: UUID().uuidString,
            kind: kind,
            createdAt: date,
            title: title,
            detail: detail
        )
    }

    private func postChanged(sessionID: String) {
        distributedCenter.postNotificationName(
            ContainmentLogNotification.changed,
            object: nil,
            userInfo: [ContainmentLogNotification.sessionIDKey: sessionID],
            deliverImmediately: true
        )
    }

    private func loadUnlocked(sessionID: String) -> VaultContainmentLogSnapshot? {
        load(VaultContainmentLogSnapshot.self, from: logURL(for: sessionID))
    }

    private func write<T: Encodable>(_ value: T, to url: URL) throws {
        try fileManager.createDirectory(
            at: url.deletingLastPathComponent(),
            withIntermediateDirectories: true,
            attributes: nil
        )
        let data = try encoder.encode(value)
        try data.write(to: url, options: .atomic)
    }

    private func load<T: Decodable>(_ type: T.Type, from url: URL) -> T? {
        guard let data = try? Data(contentsOf: url) else {
            return nil
        }
        return try? decoder.decode(type, from: data)
    }

    private func logURL(for sessionID: String) -> URL {
        logDirectoryURL()
            .appendingPathComponent("\(safeFileName(sessionID)).json", isDirectory: false)
    }

    private func logDirectoryURL() -> URL {
        rootURL()
            .appendingPathComponent("vault/containments", isDirectory: true)
    }

    private func rootURL() -> URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(
                "Library/Application Support/Automic Vault",
                isDirectory: true
            )
    }

    private func safeFileName(_ value: String) -> String {
        let allowed = CharacterSet.alphanumerics.union(CharacterSet(charactersIn: "-_."))
        return String(
            value.unicodeScalars.map { allowed.contains($0) ? Character($0) : "-" }
        )
    }
}
