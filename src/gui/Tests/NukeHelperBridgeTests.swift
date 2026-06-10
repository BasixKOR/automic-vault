import XCTest
@testable import AutomicVaultApp

final class NukeHelperBridgeTests: XCTestCase {
    func testStartupReplyGuardTimesOutBeforeProgressOrReply() {
        let completion = expectation(description: "timeout completes")
        var invalidatedConnection = false

        let replyGuard = NukeHelperStartupReplyGuard<NukeHelperResult>(
            operationName: "Update Package",
            startupTimeout: 0.01,
            activityTimeout: nil,
            completion: { result in
                guard case .failure(let error) = result else {
                    XCTFail("Expected timeout failure")
                    return
                }
                XCTAssertTrue(error.localizedDescription.contains("did not receive a response"))
                completion.fulfill()
            },
            onFailure: {
                invalidatedConnection = true
            }
        )

        replyGuard.startWatchdog()

        wait(for: [completion], timeout: 1)
        XCTAssertTrue(invalidatedConnection)
    }

    func testStartupReplyGuardCompletesOnlyOnce() {
        let completion = expectation(description: "single completion")
        completion.expectedFulfillmentCount = 1

        let replyGuard = NukeHelperStartupReplyGuard<NukeHelperResult>(
            operationName: "Update Package",
            startupTimeout: 1,
            activityTimeout: nil,
            completion: { _ in
                completion.fulfill()
            },
            onFailure: {}
        )

        replyGuard.fail(NukeHelperBridgeError.connectionFailed("first failure"))
        replyGuard.fail(NukeHelperBridgeError.connectionFailed("second failure"))
        replyGuard.complete(.success(NukeHelperResult(
            message: "late success",
            processedPackages: [],
            value: nil
        )))

        wait(for: [completion], timeout: 1)
    }

    func testStartupReplyGuardTimesOutAfterProgressStops() {
        let completion = expectation(description: "activity timeout completes")

        let replyGuard = NukeHelperStartupReplyGuard<NukeHelperResult>(
            operationName: "Update Package",
            startupTimeout: 1,
            activityTimeout: 0.01,
            completion: { result in
                guard case .failure(let error) = result else {
                    XCTFail("Expected activity timeout failure")
                    return
                }
                XCTAssertTrue(error.localizedDescription.contains("stopped receiving responses"))
                completion.fulfill()
            },
            onFailure: {}
        )

        replyGuard.startWatchdog()
        replyGuard.markStarted()

        wait(for: [completion], timeout: 1)
    }
}
