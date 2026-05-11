import Foundation

struct VaultExecutionIntent: Codable, Equatable {
    let tool: String
    let args: [String]
    let cwd: String
    let env: [String: String]
    let agentID: String?

    enum CodingKeys: String, CodingKey {
        case tool
        case args
        case cwd
        case env
        case agentID = "agent_id"
    }
}

struct VaultApprovalRequestSnapshot: Codable, Equatable {
    let id: String
    let intent: VaultExecutionIntent
}

struct VaultApprovalDecision: Codable, Equatable {
    let id: String
    let approved: Bool
    let reason: String?
}

enum VaultNotification {
    static let pendingApprovalChanged = Notification.Name(
        "com.automicvault.vault-approval.pending-changed"
    )
}

struct IsotopeApprovalRequestSnapshot: Codable, Equatable {
    let id: String
    let keys: [String]
    let executablePath: String
    let executableRootControlled: Bool?
    let scriptPath: String?
    let requestedScriptPath: String?
    let scriptRootControlled: Bool?
    let requestedExecutablePath: String?
    let argv: [String]
    let cwd: String
    let parentProcess: IsotopeParentProcessSnapshot
    let canAlwaysAllow: Bool

    enum CodingKeys: String, CodingKey {
        case id
        case keys
        case executablePath = "executable_path"
        case executableRootControlled = "executable_root_controlled"
        case scriptPath = "script_path"
        case requestedScriptPath = "requested_script_path"
        case scriptRootControlled = "script_root_controlled"
        case requestedExecutablePath = "requested_executable_path"
        case argv
        case cwd
        case parentProcess = "parent_process"
        case canAlwaysAllow = "can_always_allow"
    }
}

struct IsotopeParentProcessSnapshot: Codable, Equatable {
    let pid: Int32
    let executablePath: String?
    let displayName: String?

    enum CodingKeys: String, CodingKey {
        case pid
        case executablePath = "executable_path"
        case displayName = "display_name"
    }
}

struct IsotopeApprovalDecision: Codable, Equatable {
    let id: String
    let approved: Bool
    let alwaysAllow: Bool
    let reason: String?

    enum CodingKeys: String, CodingKey {
        case id
        case approved
        case alwaysAllow = "always_allow"
        case reason
    }
}

enum IsotopeNotification {
    static let pendingApprovalChanged = Notification.Name(
        "com.automicvault.isotope-approval.pending-changed"
    )
}

struct GateApprovalRequestSnapshot: Codable, Equatable {
    let id: String
    let message: String
    let cwd: String
    let parentProcess: IsotopeParentProcessSnapshot

    enum CodingKeys: String, CodingKey {
        case id
        case message
        case cwd
        case parentProcess = "parent_process"
    }
}

struct GateApprovalDecision: Codable, Equatable {
    let id: String
    let approved: Bool
    let reason: String?
}

enum GateNotification {
    static let pendingApprovalChanged = Notification.Name(
        "com.automicvault.gate-approval.pending-changed"
    )
}

final class VaultApprovalStore {
    private let encoder = JSONEncoder()
    private let decoder = JSONDecoder()
    private let fileManager = FileManager.default
    private let distributedCenter = DistributedNotificationCenter.default()

    init() {
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
    }

    func loadPendingApproval() -> VaultApprovalRequestSnapshot? {
        guard let approval = load(
            VaultApprovalRequestSnapshot.self,
            from: pendingApprovalURL()
        ) else {
            return nil
        }
        if fileManager.fileExists(atPath: decisionURL(for: approval.id).path) {
            removePendingApproval(id: approval.id)
            return nil
        }
        return approval
    }

    func savePendingApproval(_ approval: VaultApprovalRequestSnapshot) throws {
        try write(approval, to: pendingApprovalURL())
        postPendingApprovalChanged()
    }

    func clearPendingApproval(id: String) {
        removePendingApproval(id: id)
        try? fileManager.removeItem(at: decisionURL(for: id))
    }

    func saveDecision(_ decision: VaultApprovalDecision) throws {
        try write(decision, to: decisionURL(for: decision.id))
        removePendingApproval(id: decision.id)
        postPendingApprovalChanged()
    }

    private func removePendingApproval(id: String) {
        let pendingURL = pendingApprovalURL()
        if let current = load(VaultApprovalRequestSnapshot.self, from: pendingURL), current.id == id {
            try? fileManager.removeItem(at: pendingURL)
            postPendingApprovalChanged()
        }
    }

    func loadDecision(id: String) -> VaultApprovalDecision? {
        load(VaultApprovalDecision.self, from: decisionURL(for: id))
    }

    func observePendingApprovalChanges(
        using block: @escaping (Notification) -> Void
    ) -> NSObjectProtocol {
        distributedCenter.addObserver(
            forName: VaultNotification.pendingApprovalChanged,
            object: nil,
            queue: .main,
            using: block
        )
    }

    func postPendingApprovalChanged() {
        distributedCenter.postNotificationName(
            VaultNotification.pendingApprovalChanged,
            object: nil,
            userInfo: nil,
            deliverImmediately: true
        )
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

    private func rootURL() -> URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(
                "Library/Application Support/Automic Vault",
                isDirectory: true
            )
    }

    private func pendingApprovalURL() -> URL {
        rootURL().appendingPathComponent("vault/pending-approval.json", isDirectory: false)
    }

    private func decisionURL(for id: String) -> URL {
        rootURL()
            .appendingPathComponent("vault/decisions", isDirectory: true)
            .appendingPathComponent("\(id).json", isDirectory: false)
    }
}

final class IsotopeApprovalStore {
    private let encoder = JSONEncoder()
    private let decoder = JSONDecoder()
    private let fileManager = FileManager.default
    private let distributedCenter = DistributedNotificationCenter.default()

    init() {
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
    }

    func loadPendingApproval() -> IsotopeApprovalRequestSnapshot? {
        guard let approval = load(
            IsotopeApprovalRequestSnapshot.self,
            from: pendingApprovalURL()
        ) else {
            return nil
        }
        if fileManager.fileExists(atPath: decisionURL(for: approval.id).path) {
            removePendingApproval(id: approval.id)
            return nil
        }
        return approval
    }

    func clearPendingApproval(id: String) {
        removePendingApproval(id: id)
        try? fileManager.removeItem(at: decisionURL(for: id))
    }

    func saveDecision(_ decision: IsotopeApprovalDecision) throws {
        try write(decision, to: decisionURL(for: decision.id))
        removePendingApproval(id: decision.id)
        postPendingApprovalChanged()
    }

    private func removePendingApproval(id: String) {
        let pendingURL = pendingApprovalURL()
        if let current = load(IsotopeApprovalRequestSnapshot.self, from: pendingURL),
           current.id == id {
            try? fileManager.removeItem(at: pendingURL)
            postPendingApprovalChanged()
        }
    }

    func observePendingApprovalChanges(
        using block: @escaping (Notification) -> Void
    ) -> NSObjectProtocol {
        distributedCenter.addObserver(
            forName: IsotopeNotification.pendingApprovalChanged,
            object: nil,
            queue: .main,
            using: block
        )
    }

    func postPendingApprovalChanged() {
        distributedCenter.postNotificationName(
            IsotopeNotification.pendingApprovalChanged,
            object: nil,
            userInfo: nil,
            deliverImmediately: true
        )
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

    private func rootURL() -> URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(
                "Library/Application Support/Automic Vault",
                isDirectory: true
            )
    }

    private func pendingApprovalURL() -> URL {
        rootURL().appendingPathComponent("isotope/pending-approval.json", isDirectory: false)
    }

    private func decisionURL(for id: String) -> URL {
        rootURL()
            .appendingPathComponent("isotope/decisions", isDirectory: true)
            .appendingPathComponent("\(id).json", isDirectory: false)
    }
}

final class GateApprovalStore {
    private let encoder = JSONEncoder()
    private let decoder = JSONDecoder()
    private let fileManager = FileManager.default
    private let distributedCenter = DistributedNotificationCenter.default()

    init() {
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
    }

    func loadPendingApproval() -> GateApprovalRequestSnapshot? {
        guard let approval = load(
            GateApprovalRequestSnapshot.self,
            from: pendingApprovalURL()
        ) else {
            return nil
        }
        if fileManager.fileExists(atPath: decisionURL(for: approval.id).path) {
            removePendingApproval(id: approval.id)
            return nil
        }
        return approval
    }

    func clearPendingApproval(id: String) {
        removePendingApproval(id: id)
        try? fileManager.removeItem(at: decisionURL(for: id))
    }

    func saveDecision(_ decision: GateApprovalDecision) throws {
        try write(decision, to: decisionURL(for: decision.id))
        removePendingApproval(id: decision.id)
        postPendingApprovalChanged()
    }

    private func removePendingApproval(id: String) {
        let pendingURL = pendingApprovalURL()
        if let current = load(GateApprovalRequestSnapshot.self, from: pendingURL),
           current.id == id {
            try? fileManager.removeItem(at: pendingURL)
            postPendingApprovalChanged()
        }
    }

    func observePendingApprovalChanges(
        using block: @escaping (Notification) -> Void
    ) -> NSObjectProtocol {
        distributedCenter.addObserver(
            forName: GateNotification.pendingApprovalChanged,
            object: nil,
            queue: .main,
            using: block
        )
    }

    func postPendingApprovalChanged() {
        distributedCenter.postNotificationName(
            GateNotification.pendingApprovalChanged,
            object: nil,
            userInfo: nil,
            deliverImmediately: true
        )
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

    private func rootURL() -> URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(
                "Library/Application Support/Automic Vault",
                isDirectory: true
            )
    }

    private func pendingApprovalURL() -> URL {
        rootURL().appendingPathComponent("gate/pending-approval.json", isDirectory: false)
    }

    private func decisionURL(for id: String) -> URL {
        rootURL()
            .appendingPathComponent("gate/decisions", isDirectory: true)
            .appendingPathComponent("\(id).json", isDirectory: false)
    }
}
