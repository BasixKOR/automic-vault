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
}
