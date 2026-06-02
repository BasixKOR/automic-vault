import Foundation
import XCTest
@testable import AutomicVaultApp

final class DeepLinkTests: XCTestCase {
    func testInstallDeepLinkParsesSinglePackage() throws {
        let link = try XCTUnwrap(
            AutomicVaultDeepLink(url: URL(string: "automicvault://install?package=brew%3Auv")!)
        )

        XCTAssertEqual(link.action, .install(packageNames: ["brew:uv"]))
    }

    func testInstallDeepLinkParsesPackageList() throws {
        let link = try XCTUnwrap(
            AutomicVaultDeepLink(
                url: URL(
                    string: "automicvault://install?packages=brew%3Affmpeg-full,brew%3Apython%403.13&package=brew%3Agh"
                )!
            )
        )

        XCTAssertEqual(
            link.action,
            .install(packageNames: ["brew:ffmpeg-full", "brew:python@3.13", "brew:gh"])
        )
    }

    func testInstallDeepLinkDeduplicatesAndSkipsInvalidPackages() throws {
        let link = try XCTUnwrap(
            AutomicVaultDeepLink(
                url: URL(
                    string: "automicvault://install?package=brew%3Auv&package=brew%3Auv&package=brew%3A%3Bbad"
                )!
            )
        )

        XCTAssertEqual(link.action, .install(packageNames: ["brew:uv"]))
    }

    func testUnknownDeepLinkIsRejected() {
        XCTAssertNil(AutomicVaultDeepLink(url: URL(string: "automicvault://open?package=brew%3Auv")!))
        XCTAssertNil(AutomicVaultDeepLink(url: URL(string: "https://www.automicvault.com/")!))
    }
}
