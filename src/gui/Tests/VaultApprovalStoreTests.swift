import XCTest
@testable import AutomicVaultApp

final class VaultApprovalStoreTests: XCTestCase {
    func testDotenvApprovalRequestDefaultsMissingCommandToEmpty() throws {
        let data = Data("""
        {
          "id": "request-1",
          "mode": "export",
          "env_file_path": "/tmp/project/.env",
          "project_root": "/tmp/project",
          "env_sha256": "abc",
          "public_key_fingerprint": "def",
          "keys": ["FOO"],
          "cwd": "/tmp/project",
          "parent_process": {
            "pid": 123,
            "executable_path": "/bin/zsh",
            "display_name": "zsh"
          }
        }
        """.utf8)

        let approval = try JSONDecoder().decode(DotenvApprovalRequestSnapshot.self, from: data)

        XCTAssertEqual(approval.id, "request-1")
        XCTAssertEqual(approval.mode, .export)
        XCTAssertEqual(approval.command, [])
    }

    func testDotenvApprovalRequestDecodesCommandWhenPresent() throws {
        let data = Data("""
        {
          "id": "request-2",
          "mode": "run",
          "env_file_path": "/tmp/project/.env",
          "project_root": "/tmp/project",
          "env_sha256": "abc",
          "public_key_fingerprint": "def",
          "keys": ["FOO"],
          "cwd": "/tmp/project",
          "parent_process": {
            "pid": 123,
            "executable_path": "/bin/zsh",
            "display_name": "zsh"
          },
          "command": ["/usr/bin/env"]
        }
        """.utf8)

        let approval = try JSONDecoder().decode(DotenvApprovalRequestSnapshot.self, from: data)

        XCTAssertEqual(approval.mode, .run)
        XCTAssertEqual(approval.command, ["/usr/bin/env"])
    }

    func testDotenvApprovalViewWrapsOverflowingKeyPills() throws {
        let compactView = DotenvApprovalView(approval: dotenvApproval(keys: ["FOO", "BAR"]))
        let wrappedView = DotenvApprovalView(approval: dotenvApproval(keys: [
            "APPLE_USERNAME",
            "AWS_ACCOUNT_ID",
            "AWS_REGION",
            "MIN_MACOS_VER",
            "HOMEBREW_GITHUB_API_TOKEN",
            "POSTHOG_PROJECT_API_KEY",
            "SENTRY_AUTH_TOKEN",
        ]))

        wrappedView.frame = NSRect(origin: .zero, size: wrappedView.intrinsicContentSize)
        wrappedView.layoutSubtreeIfNeeded()

        let secretsPanel = try XCTUnwrap(wrappedView.subviews.first)
        let keyFlow = try XCTUnwrap(secretsPanel.subviews.last)
        keyFlow.layoutSubtreeIfNeeded()
        let pillRows = Set(keyFlow.subviews.map { round($0.frame.minY) })
        let maxPillX = try XCTUnwrap(keyFlow.subviews.map(\.frame.maxX).max())

        XCTAssertGreaterThan(wrappedView.intrinsicContentSize.height, compactView.intrinsicContentSize.height)
        XCTAssertGreaterThan(pillRows.count, 1)
        XCTAssertLessThanOrEqual(maxPillX, keyFlow.bounds.width + 0.5)
    }

    private func dotenvApproval(keys: [String]) -> DotenvApprovalRequestSnapshot {
        DotenvApprovalRequestSnapshot(
            id: "request-1",
            mode: .export,
            envFilePath: "/tmp/project/.env",
            projectRoot: "/tmp/project",
            envSha256: "abc",
            publicKeyFingerprint: "def",
            keys: keys,
            cwd: "/tmp/project",
            parentProcess: IsotopeParentProcessSnapshot(
                pid: 123,
                executablePath: "/bin/zsh",
                displayName: "zsh"
            )
        )
    }
}
