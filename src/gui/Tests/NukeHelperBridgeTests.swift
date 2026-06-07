import XCTest
@testable import AutomicVaultApp

final class NukeHelperBridgeTests: XCTestCase {
    func testStartupReplyGuardTimesOutBeforeProgressOrReply() {
        let completion = expectation(description: "timeout completes")
        var invalidatedConnection = false

        let guard = NukeHelperStartupReplyGuard<NukeHelperResult>(
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

        guard.startWatchdog()

        wait(for: [completion], timeout: 1)
        XCTAssertTrue(invalidatedConnection)
    }

    func testStartupReplyGuardCompletesOnlyOnce() {
        let completion = expectation(description: "single completion")
        completion.expectedFulfillmentCount = 1

        let guard = NukeHelperStartupReplyGuard<NukeHelperResult>(
            operationName: "Update Package",
            startupTimeout: 1,
            activityTimeout: nil,
            completion: { _ in
                completion.fulfill()
            },
            onFailure: {}
        )

        guard.fail(NukeHelperBridgeError.connectionFailed("first failure"))
        guard.fail(NukeHelperBridgeError.connectionFailed("second failure"))
        guard.complete(.success(NukeHelperResult(
            message: "late success",
            processedPackages: [],
            value: nil
        )))

        wait(for: [completion], timeout: 1)
    }

    func testStartupReplyGuardTimesOutAfterProgressStops() {
        let completion = expectation(description: "activity timeout completes")

        let guard = NukeHelperStartupReplyGuard<NukeHelperResult>(
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

        guard.startWatchdog()
        guard.markStarted()

        wait(for: [completion], timeout: 1)
    }
}
