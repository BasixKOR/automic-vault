import XCTest
@testable import AutomicVaultApp

final class UpdateProgressViewModelTests: XCTestCase {
    @MainActor
    func testProgressEventsUsePlannedPackageIdentity() {
        let model = UpdateProgressViewModel()
        model.begin(
            packages: ["isotope:node"],
            activationLog: "Authorized",
            initialOperation: "Awaiting helper authorization"
        )

        model.handle(event: .downloading(
            package: "node",
            bytesPerSecond: 1_200_000,
            progress: 0.40
        ))

        XCTAssertEqual(model.rows.map(\.id), ["isotope:node"])
        XCTAssertEqual(model.rows.first?.stage, .downloading)
        XCTAssertEqual(model.operation, "Updating node")

        model.handle(event: .downloading(
            package: "icu4c@78",
            bytesPerSecond: 900_000,
            progress: 0.60
        ))
        model.handle(event: .installing(package: "icu4c@78"))
        model.handle(event: .completed(package: "icu4c@78"))

        XCTAssertEqual(model.rows.map(\.id), ["isotope:node"])
        XCTAssertEqual(model.rows.first?.stage, .downloading)
        XCTAssertEqual(model.operation, "Updating node")

        model.handle(event: .installing(package: "node"))

        XCTAssertEqual(model.rows.first?.stage, .extracting)
        XCTAssertEqual(model.operation, "Extracting node")
    }

    @MainActor
    func testDiscoveredPackagesKeepInitialAdditionOrderWhenNoPlanExists() {
        let model = UpdateProgressViewModel()
        model.begin(
            packages: [],
            activationLog: "Authorized",
            initialOperation: "Waiting for package plan"
        )

        model.handle(event: .downloading(
            package: "zstd",
            bytesPerSecond: 800_000,
            progress: 0.25
        ))
        model.handle(event: .log(
            package: "node",
            message: "dependency already current"
        ))
        model.handle(event: .downloading(
            package: "curl",
            bytesPerSecond: 700_000,
            progress: 0.15
        ))

        XCTAssertEqual(model.rows.map(\.id), ["zstd", "node", "curl"])
    }

    @MainActor
    func testHiddenDependencyProgressDoesNotReduceTerminalCompletion() {
        let model = UpdateProgressViewModel()
        model.begin(
            packages: ["isotope:node"],
            activationLog: "Authorized",
            initialOperation: "Awaiting helper authorization"
        )

        model.handle(event: .downloading(
            package: "node",
            bytesPerSecond: 1_200_000,
            progress: 0.40
        ))
        model.handle(event: .downloading(
            package: "icu4c@78",
            bytesPerSecond: 900_000,
            progress: 0.60
        ))

        model.succeed(message: "Hardening complete", packages: ["isotope:node"])

        XCTAssertEqual(model.rows.map(\.id), ["isotope:node"])
        XCTAssertEqual(model.rows.first?.stage, .completed)
        XCTAssertEqual(model.totalCount, 1)
        XCTAssertEqual(model.overallProgress, 1)
    }
}
