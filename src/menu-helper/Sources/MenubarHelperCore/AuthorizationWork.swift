/// Runs synchronous security checks and record-before-release work away from UI isolation.
@concurrent
package func performAuthorizationWork<Result: Sendable>(
    _ operation: @Sendable () throws -> Result
) async rethrows -> Result {
    try operation()
}

/// Serial checks: a slow validation cannot create an overlapping backlog of work.
package func monitorAuthorizationValidity(
    interval: Duration = .seconds(1),
    validate: @Sendable () throws -> Void
) async throws {
    while true {
        try await Task.sleep(for: interval)
        try await performAuthorizationWork(validate)
        try Task.checkCancellation()
    }
}
