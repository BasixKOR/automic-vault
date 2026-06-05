import Foundation

struct VaultExecutionIntent: Codable, Equatable {
    let tool: String
    let args: [String]
    let cwd: String
    let env: [String: String]
    let agentID: String?
    let requestingProcess: IsotopeParentProcessSnapshot?

    enum CodingKeys: String, CodingKey {
        case tool
        case args
        case cwd
        case env
        case agentID = "agent_id"
        case requestingProcess = "requesting_process"
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
    let scriptSha256: String?
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
        case scriptSha256 = "script_sha256"
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

struct DotenvProcessSnapshot: Codable, Equatable {
    let pid: Int32
    let parentPid: Int32
    let executablePath: String?
    let displayName: String?

    enum CodingKeys: String, CodingKey {
        case pid
        case parentPid = "parent_pid"
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
    static let automaticApprovalGranted = Notification.Name(
        "com.automicvault.isotope-approval.automatic-granted"
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

enum DotenvApprovalMode: String, Codable, Equatable {
    case export
    case run
}

struct DotenvApprovalRequestSnapshot: Codable, Equatable {
    let id: String
    let mode: DotenvApprovalMode
    let envFilePath: String
    let projectRoot: String
    let envSha256: String
    let publicKeyFingerprint: String
    let keys: [String]
    let cwd: String
    let parentProcess: IsotopeParentProcessSnapshot
    let processAncestry: [DotenvProcessSnapshot]
    let command: [String]

    enum CodingKeys: String, CodingKey {
        case id
        case mode
        case envFilePath = "env_file_path"
        case projectRoot = "project_root"
        case envSha256 = "env_sha256"
        case publicKeyFingerprint = "public_key_fingerprint"
        case keys
        case cwd
        case parentProcess = "parent_process"
        case processAncestry = "process_ancestry"
        case command
    }

    init(
        id: String,
        mode: DotenvApprovalMode,
        envFilePath: String,
        projectRoot: String,
        envSha256: String,
        publicKeyFingerprint: String,
        keys: [String],
        cwd: String,
        parentProcess: IsotopeParentProcessSnapshot,
        processAncestry: [DotenvProcessSnapshot] = [],
        command: [String] = []
    ) {
        self.id = id
        self.mode = mode
        self.envFilePath = envFilePath
        self.projectRoot = projectRoot
        self.envSha256 = envSha256
        self.publicKeyFingerprint = publicKeyFingerprint
        self.keys = keys
        self.cwd = cwd
        self.parentProcess = parentProcess
        self.processAncestry = processAncestry
        self.command = command
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        mode = try container.decode(DotenvApprovalMode.self, forKey: .mode)
        envFilePath = try container.decode(String.self, forKey: .envFilePath)
        projectRoot = try container.decode(String.self, forKey: .projectRoot)
        envSha256 = try container.decode(String.self, forKey: .envSha256)
        publicKeyFingerprint = try container.decode(String.self, forKey: .publicKeyFingerprint)
        keys = try container.decode([String].self, forKey: .keys)
        cwd = try container.decode(String.self, forKey: .cwd)
        parentProcess = try container.decode(
            IsotopeParentProcessSnapshot.self,
            forKey: .parentProcess
        )
        processAncestry = try container.decodeIfPresent(
            [DotenvProcessSnapshot].self,
            forKey: .processAncestry
        ) ?? []
        command = try container.decodeIfPresent([String].self, forKey: .command) ?? []
    }
}

struct DotenvApprovalDecision: Codable, Equatable {
    let id: String
    let approved: Bool
    let reason: String?
}

enum DotenvNotification {
    static let pendingApprovalChanged = Notification.Name(
        "com.automicvault.dotenv-approval.pending-changed"
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

final class DotenvApprovalStore {
    private let encoder = JSONEncoder()
    private let decoder = JSONDecoder()
    private let fileManager = FileManager.default
    private let distributedCenter = DistributedNotificationCenter.default()

    init() {
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
    }

    func loadPendingApproval() -> DotenvApprovalRequestSnapshot? {
        guard let approval = load(
            DotenvApprovalRequestSnapshot.self,
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

    func saveDecision(_ decision: DotenvApprovalDecision) throws {
        try write(decision, to: decisionURL(for: decision.id))
        removePendingApproval(id: decision.id)
        postPendingApprovalChanged()
    }

    private func removePendingApproval(id: String) {
        let pendingURL = pendingApprovalURL()
        if let current = load(DotenvApprovalRequestSnapshot.self, from: pendingURL),
           current.id == id {
            try? fileManager.removeItem(at: pendingURL)
            postPendingApprovalChanged()
        }
    }

    func observePendingApprovalChanges(
        using block: @escaping (Notification) -> Void
    ) -> NSObjectProtocol {
        distributedCenter.addObserver(
            forName: DotenvNotification.pendingApprovalChanged,
            object: nil,
            queue: .main,
            using: block
        )
    }

    func postPendingApprovalChanged() {
        distributedCenter.postNotificationName(
            DotenvNotification.pendingApprovalChanged,
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
        rootURL().appendingPathComponent("dotenv/pending-approval.json", isDirectory: false)
    }

    private func decisionURL(for id: String) -> URL {
        rootURL()
            .appendingPathComponent("dotenv/decisions", isDirectory: true)
            .appendingPathComponent("\(id).json", isDirectory: false)
    }
}
