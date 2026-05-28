import Foundation
import XCTest
@testable import AutomicVaultApp

final class MainWindowModelTests: XCTestCase {
    @MainActor
    func testPulseTimestampUsesHoursUntilSixtyHours() throws {
        let referenceDate = try XCTUnwrap(Self.date("2026-05-28T12:00:00Z"))
        let result = pulseResult(
            lastUpdatedAt: "2026-05-26T01:00:00Z",
            pulseKind: "updated"
        )

        XCTAssertEqual(
            MainWindowModel.pulseListTimestampText(
                for: result,
                relativeTo: referenceDate
            ),
            "Updated 59 hours ago"
        )
    }

    @MainActor
    func testPulseTimestampFallsBackAfterSixtyHours() throws {
        let referenceDate = try XCTUnwrap(Self.date("2026-05-28T12:00:00Z"))
        let result = pulseResult(
            lastUpdatedAt: "2026-05-25T23:00:00Z",
            pulseKind: "updated"
        )

        let text = MainWindowModel.pulseListTimestampText(
            for: result,
            relativeTo: referenceDate
        )

        XCTAssertTrue(text.hasPrefix("Updated "))
        XCTAssertFalse(text.contains("61 hours ago"))
    }

    @MainActor
    func testNewPulseTimestampShowsAgeWithoutUpdatedPrefix() throws {
        let referenceDate = try XCTUnwrap(Self.date("2026-05-28T12:00:00Z"))
        let result = pulseResult(
            lastUpdatedAt: "2026-05-28T00:00:00.000Z",
            pulseKind: "new"
        )

        XCTAssertEqual(
            MainWindowModel.pulseListTimestampText(
                for: result,
                relativeTo: referenceDate
            ),
            "12 hours ago"
        )
    }

    private func pulseResult(
        lastUpdatedAt: String?,
        pulseKind: String
    ) -> PackageSearchResult {
        PackageSearchResult(
            name: "brew:example",
            source: .formula(rootFormula: "example"),
            version: "1.0",
            description: "Example package",
            homepage: nil,
            dependencies: [],
            lastUpdatedAt: lastUpdatedAt,
            securityState: nil,
            pulseKind: pulseKind
        )
    }

    private static func date(_ raw: String) -> Date? {
        ISO8601DateFormatter().date(from: raw)
    }
}
