import CryptoKit
import Darwin
import Foundation
import Security

public let hcloudOfficialTarget = "/opt/av/hcloud/1.69.0/hcloud"
public let hcloudLauncher = "/usr/local/bin/hcloud"
public let hcloudLauncherStub = "#!/usr/local/bin/av __hcloud\n"

// Both reviewed vendor executables; application and CLI may run under different architectures.
private let hcloudReleaseDigests: Set<String> = [
    "fd862d059a17175b491a15a531984ef76b516fe1ae422ab1c5201beb92ba76b2",
    "67889b60fd6b99f666111d328f862722a8b5cd9a04d5c2260ca8143d5aa61007",
]

func hcloudProtectedEntry(_ path: String, directory: Bool, uid: uid_t = 0) -> Bool {
    var info = stat()
    guard lstat(path, &info) == 0, info.st_uid == uid,
          info.st_mode & 0o022 == 0,
          info.st_mode & S_IFMT == (directory ? S_IFDIR : S_IFREG),
          directory || (info.st_gid == 0 && info.st_nlink == 1 && info.st_mode & 0o7777 == 0o755)
    else { return false }
    return gitTransportPathHasNoACL(path)
}

public func hcloudReleaseBinaryValid(path: String) -> Bool {
    guard let attributes = try? FileManager.default.attributesOfItem(atPath: path),
          let size = attributes[.size] as? NSNumber, size.uint64Value <= 64 * 1024 * 1024,
          let data = try? Data(contentsOf: URL(fileURLWithPath: path)),
          hcloudReleaseDigests.contains(SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined())
    else { return false }
    let requirementText = "identifier \"hcloud\" and anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] exists and certificate leaf[field.1.2.840.113635.100.6.1.13] exists and certificate leaf[subject.OU] = \"4PM38G6W5R\""
    var code: SecStaticCode?
    var requirement: SecRequirement?
    guard SecStaticCodeCreateWithPath(URL(fileURLWithPath: path) as CFURL, [], &code) == errSecSuccess,
          let code,
          SecRequirementCreateWithString(requirementText as CFString, [], &requirement) == errSecSuccess,
          SecStaticCodeCheckValidity(code, SecCSFlags(rawValue: kSecCSStrictValidate), requirement) == errSecSuccess
    else { return false }
    // The complete digest pins Hardened Runtime, empty entitlements and system-only dependencies.
    return true
}

public func hcloudInstalledReleaseValid() -> Bool {
    let directories = ["/", "/opt", "/opt/av", "/opt/av/hcloud", "/opt/av/hcloud/1.69.0", "/usr", "/usr/local", "/usr/local/bin"]
    guard directories.allSatisfy({ hcloudProtectedEntry($0, directory: true) }),
          hcloudProtectedEntry(hcloudOfficialTarget, directory: false),
          hcloudProtectedEntry(hcloudLauncher, directory: false),
          readProtectedAWSStub(path: hcloudLauncher) == hcloudLauncherStub
    else { return false }
    return hcloudReleaseBinaryValid(path: hcloudOfficialTarget)
}
