import Darwin
import Foundation
import Testing
@testable import MenubarHelperCore

@Test func doctlRejectsUntrustedPathsAndBinary() throws {
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: false)
    defer { try? FileManager.default.removeItem(at: directory) }
    try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: directory.path)
    #expect(doctlProtectedEntry(directory.path, directory: true, uid: getuid()))
    try FileManager.default.setAttributes([.posixPermissions: 0o777], ofItemAtPath: directory.path)
    #expect(!doctlProtectedEntry(directory.path, directory: true, uid: getuid()))
    let binary = directory.appendingPathComponent("doctl")
    try Data("fake binary".utf8).write(to: binary)
    #expect(!doctlReleaseBinaryValid(path: binary.path))
    #expect(!doctlProtectedEntry(binary.path, directory: false))
    let link = directory.appendingPathComponent("link")
    try FileManager.default.createSymbolicLink(atPath: link.path, withDestinationPath: binary.path)
    #expect(!doctlProtectedEntry(link.path, directory: false))
}

@Test func doctlReviewedFixtureVerifiesAndRejectsTampering() throws {
    guard let path = ProcessInfo.processInfo.environment["AV_DOCTL_BINARY_FIXTURE"] else { return }
    #expect(doctlReleaseBinaryValid(path: path))
    let copy = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: copy) }
    var data = try Data(contentsOf: URL(fileURLWithPath: path))
    data[1024] ^= 1
    try data.write(to: copy)
    #expect(!doctlReleaseBinaryValid(path: copy.path))
}
