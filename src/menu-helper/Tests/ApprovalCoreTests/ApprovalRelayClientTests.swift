@testable import ApprovalCore
import Foundation
import Testing

@Test
func canceledPongWaitDoesNotHang() async {
    let pingStarted = AsyncStream<Void>.makeStream(bufferingPolicy: .bufferingNewest(1))
    let wait = Task {
        try await ApprovalRelayClient.waitForPong { _ in
            pingStarted.continuation.yield()
        }
    }
    for await _ in pingStarted.stream.prefix(1) { break }

    wait.cancel()
    await #expect(throws: CancellationError.self) {
        try await wait.value
    }
}
