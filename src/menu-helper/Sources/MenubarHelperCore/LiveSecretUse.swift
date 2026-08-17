import Foundation

public struct LiveSecretUseSnapshot: Identifiable, Equatable, Sendable {
    public let id: UUID
    public let launcherName: String?
    public let targetPath: String
    public let processID: Int32
    public let secretNames: [String]
    public let startedAt: Date
}

public final class LiveSecretUseController<Process: Hashable & Sendable>: @unchecked Sendable {
    private struct Key: Hashable {
        let process: Process
        let launcherDesignatedRequirement: String?
        let targetPath: String
    }

    private struct Use {
        let id: UUID
        let launcherName: String?
        let targetPath: String
        let processID: Int32
        var secretNames: Set<String>
        let startedAt: Date

        var snapshot: LiveSecretUseSnapshot {
            LiveSecretUseSnapshot(
                id: id,
                launcherName: launcherName,
                targetPath: targetPath,
                processID: processID,
                secretNames: secretNames.sorted(),
                startedAt: startedAt
            )
        }
    }

    private let lock = NSLock()
    private var uses: [Key: Use] = [:]

    public init() {}

    public func record(
        process: Process,
        launcherDesignatedRequirement: String?,
        launcherName: String?,
        targetPath: String,
        processID: Int32,
        secretNames: Set<String>,
        startedAt: Date = Date()
    ) {
        guard !secretNames.isEmpty else { return }
        lock.lock()
        defer { lock.unlock() }
        let key = Key(
            process: process,
            launcherDesignatedRequirement: launcherDesignatedRequirement,
            targetPath: targetPath
        )
        if uses[key] != nil {
            uses[key]?.secretNames.formUnion(secretNames)
        } else {
            uses[key] = Use(
                id: UUID(),
                launcherName: launcherName,
                targetPath: targetPath,
                processID: processID,
                secretNames: secretNames,
                startedAt: startedAt
            )
        }
    }

    public func snapshots(isLive: (Process) -> Bool) -> [LiveSecretUseSnapshot] {
        lock.lock()
        defer { lock.unlock() }
        uses = uses.filter { isLive($0.key.process) }
        return uses.values.map(\.snapshot).sorted {
            $0.startedAt == $1.startedAt
                ? $0.id.uuidString < $1.id.uuidString
                : $0.startedAt < $1.startedAt
        }
    }

    public func cancelAll() {
        lock.lock()
        defer { lock.unlock() }
        uses.removeAll()
    }
}
