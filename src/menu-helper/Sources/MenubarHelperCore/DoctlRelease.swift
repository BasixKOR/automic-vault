import CryptoKit
import Darwin
import Foundation
import Security

public let doctlSignedTarget = "/opt/av/doctl/1.175.0-av.1/doctl"
public let doctlLauncher = "/usr/local/bin/doctl"
public let doctlLauncherStub = "#!/usr/local/bin/av __doctl\n"

// Both reviewed Automic Vault-signed executables; application and CLI may run under different architectures.
private let doctlReleaseDigests: Set<String> = [
    "9c762b2b1167dab0d799dffdd82c6c47c946b6451171c7d63e0de70e370c6fbd",
    "6e6cf4e285b88ac9f679749e99e23319d305f02018840555b01387291f236dc5",
]

func doctlProtectedEntry(_ path: String, directory: Bool, uid: uid_t = 0) -> Bool {
    var info = stat()
    guard lstat(path, &info) == 0, info.st_uid == uid,
          info.st_mode & 0o022 == 0,
          info.st_mode & S_IFMT == (directory ? S_IFDIR : S_IFREG),
          directory || (info.st_gid == 0 && info.st_nlink == 1 && info.st_mode & 0o7777 == 0o755)
    else { return false }
    return gitTransportPathHasNoACL(path)
}

public func doctlReleaseBinaryValid(path: String) -> Bool {
    guard let attributes = try? FileManager.default.attributesOfItem(atPath: path),
          let size = attributes[.size] as? NSNumber, size.uint64Value <= 64 * 1024 * 1024,
          let data = try? Data(contentsOf: URL(fileURLWithPath: path)),
          doctlReleaseDigests.contains(SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined())
    else { return false }
    let requirementText = "identifier \"doctl\" and anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] exists and certificate leaf[field.1.2.840.113635.100.6.1.13] exists and certificate leaf[subject.OU] = \"ZU76A67LGU\""
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

public func doctlInstalledReleaseValid() -> Bool {
    let directories = ["/", "/opt", "/opt/av", "/opt/av/doctl", "/opt/av/doctl/1.175.0-av.1", "/usr", "/usr/local", "/usr/local/bin"]
    guard directories.allSatisfy({ doctlProtectedEntry($0, directory: true) }),
          doctlProtectedEntry(doctlSignedTarget, directory: false),
          doctlProtectedEntry(doctlLauncher, directory: false),
          readProtectedAWSStub(path: doctlLauncher) == doctlLauncherStub
    else { return false }
    return doctlReleaseBinaryValid(path: doctlSignedTarget)
}

public let doctlRequiredArguments = ["--api-url=https://api.digitalocean.com/", "--trace=false"]

public func doctlArgumentsBound(_ args: [String]) -> Bool {
    args.starts(with: doctlRequiredArguments) && !args.dropFirst(2).prefix(while: { $0 != "--" }).contains {
        $0 == "--api-url" || $0.hasPrefix("--api-url=") || $0.hasPrefix("-u")
            || $0 == "--trace" || $0.hasPrefix("--trace=")
    }
}
