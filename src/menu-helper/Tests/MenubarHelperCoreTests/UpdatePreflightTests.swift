import Foundation
import Testing
@testable import MenubarHelperCore

@Test func updatePreflightUsesExactDraftResponseAndAsset() throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: false)
    defer { try? FileManager.default.removeItem(at: root) }
    let releases = root.appendingPathComponent("releases.json")
    let asset = root.appendingPathComponent("Automic-Vault-3.0.0.dmg")
    try Data("dmg".utf8).write(to: asset)
    let assetURL = "https://github.com/automic-vault/automic-vault/releases/download/3.0.0/Automic-Vault-3.0.0.dmg"
    let json = """
    [{
      "tag_name": "3.0.0",
      "target_commitish": "0123456789abcdef0123456789abcdef01234567",
      "draft": true,
      "assets": [{
        "name": "Automic-Vault-3.0.0.dmg",
        "browser_download_url": "\(assetURL)",
        "size": 3,
        "digest": "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
      }]
    }]
    """
    try Data(json.utf8).write(to: releases)

    let input = try UpdatePreflightInput(
        arguments: ["app", "--verify-update", "3.0.0", releases.path, asset.path]
    )
    #expect(input.fixture(for: UpdatePreflightInput.releasesURL)?.data == Data(json.utf8))
    #expect(input.fixture(for: URL(string: assetURL)!)?.file == asset)
    #expect(
        input.fixture(
            for: URL(string: "https://api.github.com/repos/automic-vault/automic-vault/attestations/x")!
        ) == nil
    )
}

@Test func updatePreflightRejectsNonDraftMetadata() throws {
    #expect(throws: UpdatePreflightError.self) {
        _ = try UpdatePreflightInput(arguments: ["app", "--verify-update", "nope"])
    }
}
