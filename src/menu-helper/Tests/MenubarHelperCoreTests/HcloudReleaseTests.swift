import Darwin
import Foundation
import Testing
@testable import MenubarHelperCore

@Test func hcloudRejectsUntrustedPathsAndBinary() throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: false)
    defer { try? FileManager.default.removeItem(at: directory) }
    try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: directory.path)
    #expect(hcloudProtectedEntry(directory.path, directory: true, uid: getuid()))
    try FileManager.default.setAttributes([.posixPermissions: 0o777], ofItemAtPath: directory.path)
    #expect(!hcloudProtectedEntry(directory.path, directory: true, uid: getuid()))
    let binary = directory.appendingPathComponent("hcloud")
    try Data("fake binary".utf8).write(to: binary)
    #expect(!hcloudReleaseBinaryValid(path: binary.path))
    #expect(!hcloudProtectedEntry(binary.path, directory: false))
    let link = directory.appendingPathComponent("link")
    try FileManager.default.createSymbolicLink(atPath: link.path, withDestinationPath: binary.path)
    #expect(!hcloudProtectedEntry(link.path, directory: false))
}

@Test func hcloudReviewedFixtureVerifiesAndRejectsTampering() throws {
    guard let path = ProcessInfo.processInfo.environment["AV_HCLOUD_BINARY_FIXTURE"] else { return }
    #expect(hcloudReleaseBinaryValid(path: path))
    let copy = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: copy) }
    var data = try Data(contentsOf: URL(fileURLWithPath: path))
    data[1024] ^= 1
    try data.write(to: copy)
    #expect(!hcloudReleaseBinaryValid(path: copy.path))
}
