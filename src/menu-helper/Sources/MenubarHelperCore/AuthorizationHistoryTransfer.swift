import Foundation

/// One authorized snapshot, owned by one XPC connection. Continuations never read the store.
public final class AuthorizationHistoryTransfer: @unchecked Sendable {
    public static let chunkBytes = 256 * 1024

    public struct Chunk: Sendable {
        public let bytes: Data
        public let offset: Int
        public let total: Int
    }

    private let lock = NSLock()
    private var started = false
    private var canceled = false
    private var snapshot: Data?
    private var offset = 0

    public init() {}

    public func begin() -> Bool {
        lock.withLock {
            guard !started, !canceled else { return false }
            started = true
            return true
        }
    }

    public func prepare(_ data: Data) -> Bool {
        lock.withLock {
            guard started, !canceled, snapshot == nil, offset == 0, !data.isEmpty else { return false }
            snapshot = data
            return true
        }
    }

    public func next(offset requestedOffset: Int) -> Chunk? {
        lock.withLock {
            guard !canceled, let snapshot, requestedOffset == offset else { return nil }
            let end = offset + min(Self.chunkBytes, snapshot.count - offset)
            let chunk = Chunk(bytes: snapshot.subdata(in: offset..<end), offset: offset, total: snapshot.count)
            offset = end
            if end == snapshot.count { self.snapshot = nil }
            return chunk
        }
    }

    public func cancel() {
        lock.withLock {
            canceled = true
            snapshot = nil
        }
    }
}
