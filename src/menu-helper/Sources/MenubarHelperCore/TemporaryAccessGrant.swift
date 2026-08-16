import Foundation

public enum AgentProvider: String, Hashable, Sendable {
    case codex
    case claudeCode

    public var environmentVariable: String {
        switch self {
        case .codex: "CODEX_THREAD_ID"
        case .claudeCode: "CLAUDE_CODE_SESSION_ID"
        }
    }

    public var taskLabel: String {
        switch self {
        case .codex: "Codex task"
        case .claudeCode: "Claude session"
        }
    }
}

public struct AgentTaskContext: Hashable, Sendable {
    public let provider: AgentProvider
    public let id: UUID

    public init(provider: AgentProvider, id: UUID) {
        self.provider = provider
        self.id = id
    }

    public init?(environment: [String: String]) {
        let present = AgentProvider.allCases.compactMap { provider in
            environment[provider.environmentVariable].map { (provider, $0) }
        }
        guard present.count == 1,
              let id = UUID(uuidString: present[0].1),
              id.uuidString.caseInsensitiveCompare(present[0].1) == .orderedSame
        else { return nil }
        self = Self(provider: present[0].0, id: id)
    }

    public var abbreviatedID: String { String(id.uuidString.prefix(8)) }
}

extension AgentProvider: CaseIterable {}

public struct TemporaryAccessGrantScope: Hashable, Sendable {
    public let authorizationGateID: String
    public let launcherDesignatedRequirement: String
    public let launcherRuntimeRequirement: LauncherRuntimeRequirement
    public let agentTaskContext: AgentTaskContext
    public let protection: SecretGateProtection

    public init(
        authorizationGateID: String,
        launcherDesignatedRequirement: String,
        launcherRuntimeRequirement: LauncherRuntimeRequirement,
        agentTaskContext: AgentTaskContext,
        protection: SecretGateProtection = .fullExceptSecretDumps
    ) {
        self.authorizationGateID = authorizationGateID
        self.launcherDesignatedRequirement = launcherDesignatedRequirement
        self.launcherRuntimeRequirement = launcherRuntimeRequirement
        self.agentTaskContext = agentTaskContext
        self.protection = protection.normalized(forGateID: authorizationGateID)
    }

    public func matches(
        authorizationGateID: String,
        launcherDesignatedRequirement: String,
        launcherRuntimeProtection: LauncherRuntimeProtection,
        agentTaskContext: AgentTaskContext,
        classification: SecretGateRequestClassification
    ) -> Bool {
        self.authorizationGateID == authorizationGateID
            && self.launcherDesignatedRequirement == launcherDesignatedRequirement
            && self.agentTaskContext == agentTaskContext
            && launcherRuntimeProtection.secretGateAdmissionRequirement
                == launcherRuntimeRequirement
            && protection == .fullExceptSecretDumps
            && protection.allows(classification)
    }
}

public struct TemporaryAccessGrantSnapshot: Identifiable, Equatable, Sendable {
    public let id: UUID
    public let generation: UUID
    public let scope: TemporaryAccessGrantScope
    public let launcherName: String
    public let authorizationGateName: String
    public let grantedAt: Date
    public let expiresAt: Date
    public let monotonicDeadline: TimeInterval

    public func remaining(wallNow: Date, monotonicNow: TimeInterval) -> TimeInterval {
        max(0, min(expiresAt.timeIntervalSince(wallNow), monotonicDeadline - monotonicNow))
    }
}

public final class TemporaryAccessGrantController: @unchecked Sendable {
    public static let duration: TimeInterval = 10 * 60

    private struct Grant {
        let id: UUID
        let generation: UUID
        let scope: TemporaryAccessGrantScope
        let launcherName: String
        let authorizationGateName: String
        let grantedAt: Date
        let expiresAt: Date
        let monotonicDeadline: TimeInterval

        var snapshot: TemporaryAccessGrantSnapshot {
            TemporaryAccessGrantSnapshot(
                id: id,
                generation: generation,
                scope: scope,
                launcherName: launcherName,
                authorizationGateName: authorizationGateName,
                grantedAt: grantedAt,
                expiresAt: expiresAt,
                monotonicDeadline: monotonicDeadline
            )
        }

        func isActive(wallNow: Date, monotonicNow: TimeInterval) -> Bool {
            wallNow < expiresAt && monotonicNow < monotonicDeadline
        }
    }

    private let lock = NSLock()
    private var grants: [UUID: Grant] = [:]

    public init() {}

    @discardableResult
    public func start(
        scope: TemporaryAccessGrantScope,
        launcherName: String,
        authorizationGateName: String,
        wallNow: Date = Date(),
        monotonicNow: TimeInterval = ProcessInfo.processInfo.systemUptime
    ) -> TemporaryAccessGrantSnapshot {
        startWithLease(
            scope: scope,
            launcherName: launcherName,
            authorizationGateName: authorizationGateName,
            wallNow: wallNow,
            monotonicNow: monotonicNow
        ) { _ in }.0
    }

    @discardableResult
    public func startWithLease<Result>(
        scope: TemporaryAccessGrantScope,
        launcherName: String,
        authorizationGateName: String,
        wallNow: Date = Date(),
        monotonicNow: TimeInterval = ProcessInfo.processInfo.systemUptime,
        _ body: (TemporaryAccessGrantSnapshot) throws -> Result
    ) rethrows -> (TemporaryAccessGrantSnapshot, Result) {
        lock.lock()
        defer { lock.unlock() }
        removeExpired(wallNow: wallNow, monotonicNow: monotonicNow)
        let id = grants.values.first(where: { $0.scope == scope })?.id ?? UUID()
        let grant = Grant(
            id: id,
            generation: UUID(),
            scope: scope,
            launcherName: launcherName,
            authorizationGateName: authorizationGateName,
            grantedAt: wallNow,
            expiresAt: wallNow.addingTimeInterval(Self.duration),
            monotonicDeadline: monotonicNow + Self.duration
        )
        grants[id] = grant
        return try (grant.snapshot, body(grant.snapshot))
    }

    public func snapshots(
        wallNow: Date = Date(),
        monotonicNow: TimeInterval = ProcessInfo.processInfo.systemUptime
    ) -> [TemporaryAccessGrantSnapshot] {
        lock.lock()
        defer { lock.unlock() }
        removeExpired(wallNow: wallNow, monotonicNow: monotonicNow)
        return grants.values.map(\.snapshot).sorted {
            let left = $0.remaining(wallNow: wallNow, monotonicNow: monotonicNow)
            let right = $1.remaining(wallNow: wallNow, monotonicNow: monotonicNow)
            return left == right ? $0.id.uuidString < $1.id.uuidString : left < right
        }
    }

    @discardableResult
    public func cancel(id: UUID) -> Bool {
        lock.lock()
        defer { lock.unlock() }
        return grants.removeValue(forKey: id) != nil
    }

    public func cancelAll() {
        lock.lock()
        defer { lock.unlock() }
        grants.removeAll()
    }

    public func withActiveLease<Result>(
        authorizationGateID: String,
        launcherDesignatedRequirement: String,
        launcherRuntimeProtection: LauncherRuntimeProtection,
        agentTaskContext: AgentTaskContext,
        classification: SecretGateRequestClassification,
        wallNow: Date = Date(),
        monotonicNow: TimeInterval = ProcessInfo.processInfo.systemUptime,
        _ body: (TemporaryAccessGrantSnapshot) throws -> Result
    ) rethrows -> Result? {
        lock.lock()
        defer { lock.unlock() }
        removeExpired(wallNow: wallNow, monotonicNow: monotonicNow)
        guard let grant = grants.values.first(where: {
            $0.scope.matches(
                authorizationGateID: authorizationGateID,
                launcherDesignatedRequirement: launcherDesignatedRequirement,
                launcherRuntimeProtection: launcherRuntimeProtection,
                agentTaskContext: agentTaskContext,
                classification: classification
            )
        }) else { return nil }
        return try body(grant.snapshot)
    }

    private func removeExpired(wallNow: Date, monotonicNow: TimeInterval) {
        grants = grants.filter { $0.value.isActive(wallNow: wallNow, monotonicNow: monotonicNow) }
    }
}
