import AppKit
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
        XCTAssertEqual(approval.processAncestry, [])
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

    func testDotenvApprovalRequestDecodesProcessAncestryWhenPresent() throws {
        let data = Data("""
        {
          "id": "request-3",
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
          },
          "process_ancestry": [
            {
              "pid": 123,
              "parent_pid": 456,
              "executable_path": "/bin/zsh",
              "display_name": "zsh"
            },
            {
              "pid": 456,
              "parent_pid": 1,
              "executable_path": "/Applications/Codex.app/Contents/MacOS/Codex",
              "display_name": "Codex"
            }
          ]
        }
        """.utf8)

        let approval = try JSONDecoder().decode(DotenvApprovalRequestSnapshot.self, from: data)

        XCTAssertEqual(approval.processAncestry.count, 2)
        XCTAssertEqual(approval.processAncestry[0].parentPid, 456)
        XCTAssertEqual(
            approval.processAncestry[1].executablePath,
            "/Applications/Codex.app/Contents/MacOS/Codex"
        )
    }

    func testDotenvApprovalViewShowsApplicationAncestor() throws {
        let approval = dotenvApproval(
            keys: ["FOO"],
            processAncestry: [
                DotenvProcessSnapshot(
                    pid: 123,
                    parentPid: 456,
                    executablePath: "/bin/zsh",
                    displayName: "zsh"
                ),
                DotenvProcessSnapshot(
                    pid: 456,
                    parentPid: 1,
                    executablePath: "/Applications/Codex.app/Contents/MacOS/Codex",
                    displayName: "Codex"
                ),
            ]
        )
        let view = DotenvApprovalView(approval: approval)
        let text = textFields(in: view).joined(separator: "\n")

        XCTAssertTrue(text.contains("Codex.app"), text)
        XCTAssertTrue(text.contains("via zsh"), text)
    }

    func testDotenvApprovalViewShowsOutermostApplicationForNestedHelpers() throws {
        let approval = dotenvApproval(
            keys: ["FOO"],
            processAncestry: [
                DotenvProcessSnapshot(
                    pid: 123,
                    parentPid: 456,
                    executablePath: "/bin/zsh",
                    displayName: "zsh"
                ),
                DotenvProcessSnapshot(
                    pid: 456,
                    parentPid: 1,
                    executablePath: "/Applications/Visual Studio Code.app/Contents/Frameworks/Code Helper (Plugin).app/Contents/MacOS/Code Helper (Plugin)",
                    displayName: "Code Helper (Plugin)"
                ),
            ]
        )
        let view = DotenvApprovalView(approval: approval)
        let text = textFields(in: view).joined(separator: "\n")

        XCTAssertTrue(text.contains("Visual Studio Code.app"), text)
        XCTAssertFalse(text.contains("Code Helper (Plugin).app"), text)
    }

    func testDotenvApprovalAutoRejectsCodexExportProcessTree() throws {
        let approval = dotenvApproval(
            keys: ["FOO"],
            processAncestry: [
                DotenvProcessSnapshot(
                    pid: 123,
                    parentPid: 456,
                    executablePath: "/bin/zsh",
                    displayName: "zsh"
                ),
                DotenvProcessSnapshot(
                    pid: 456,
                    parentPid: 1,
                    executablePath: "/Applications/Codex.app/Contents/MacOS/Codex",
                    displayName: "Codex"
                ),
            ]
        )

        XCTAssertEqual(approval.automaticExportRejectionSourceName, "Codex.app")
    }

    func testDotenvApprovalAutoRejectsCodexCLIParent() throws {
        let approval = dotenvApproval(
            keys: ["FOO"],
            parentProcess: IsotopeParentProcessSnapshot(
                pid: 123,
                executablePath: "/usr/local/bin/codex",
                displayName: "codex"
            )
        )

        XCTAssertEqual(approval.automaticExportRejectionSourceName, "codex")
    }

    func testDotenvApprovalDoesNotAutoRejectRunModeOrOtherApplications() throws {
        let runApproval = dotenvApproval(
            mode: .run,
            keys: ["FOO"],
            processAncestry: [
                DotenvProcessSnapshot(
                    pid: 456,
                    parentPid: 1,
                    executablePath: "/Applications/Codex.app/Contents/MacOS/Codex",
                    displayName: "Codex"
                ),
            ]
        )
        let vscodeApproval = dotenvApproval(
            keys: ["FOO"],
            processAncestry: [
                DotenvProcessSnapshot(
                    pid: 456,
                    parentPid: 1,
                    executablePath: "/Applications/Visual Studio Code.app/Contents/Frameworks/Code Helper (Plugin).app/Contents/MacOS/Code Helper (Plugin)",
                    displayName: "Code Helper (Plugin)"
                ),
            ]
        )

        XCTAssertNil(runApproval.automaticExportRejectionSourceName)
        XCTAssertNil(vscodeApproval.automaticExportRejectionSourceName)
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

    private func dotenvApproval(
        mode: DotenvApprovalMode = .export,
        keys: [String],
        parentProcess: IsotopeParentProcessSnapshot = IsotopeParentProcessSnapshot(
            pid: 123,
            executablePath: "/bin/zsh",
            displayName: "zsh"
        ),
        processAncestry: [DotenvProcessSnapshot] = []
    ) -> DotenvApprovalRequestSnapshot {
        DotenvApprovalRequestSnapshot(
            id: "request-1",
            mode: mode,
            envFilePath: "/tmp/project/.env",
            projectRoot: "/tmp/project",
            envSha256: "abc",
            publicKeyFingerprint: "def",
            keys: keys,
            cwd: "/tmp/project",
            parentProcess: parentProcess,
            processAncestry: processAncestry
        )
    }

    private func textFields(in view: NSView) -> [String] {
        let current = (view as? NSTextField).map {
            $0.attributedStringValue.string.isEmpty
                ? $0.stringValue
                : $0.attributedStringValue.string
        }
        return (current.map { [$0] } ?? []) + view.subviews.flatMap(textFields)
    }
}
