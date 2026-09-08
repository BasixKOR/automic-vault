import Foundation
import Security
import Darwin

// Copy selected v1.25.0 login-Keychain values through av 4.6.0+ import Approval.
// Usage: swift migrate-av-v1.swift /path/to/login.keychain-db NAME [NAME...]
let av = "/usr/local/bin/av"

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data((message + "\n").utf8))
    exit(1)
}

let args = Array(CommandLine.arguments.dropFirst())
guard args.count >= 2 else {
    fail("Usage: swift migrate-av-v1.swift KEYCHAIN_PATH NAME [NAME...]")
}
let names = Array(args.dropFirst())
guard Set(names).count == names.count,
      names.allSatisfy({ $0.range(of: "^[A-Za-z_][A-Za-z0-9_]*\\z", options: .regularExpression) != nil })
else { fail("Provide distinct valid Secret Names.") }

var keychain: SecKeychain?
let openStatus = SecKeychainOpen(args[0], &keychain)
guard openStatus == errSecSuccess, let keychain else {
    fail("Could not open the specified legacy Keychain (OSStatus \(openStatus)).")
}
signal(SIGPIPE, SIG_IGN)

for name in names {
    let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: "com.automicvault.isotope",
        kSecAttrAccount as String: name,
        kSecMatchSearchList as String: [keychain],
        kSecUseDataProtectionKeychain as String: false,
        kSecReturnData as String: true,
        kSecMatchLimit as String: kSecMatchLimitOne,
    ]
    var result: CFTypeRef?
    let status = SecItemCopyMatching(query as CFDictionary, &result)
    guard status == errSecSuccess, let data = result as? Data else {
        fail("Stopped at \(name): legacy Keychain read failed (OSStatus \(status)).")
    }
    guard !data.isEmpty, data.count <= 1_048_576, !data.contains(0),
          String(data: data, encoding: .utf8) != nil else {
        fail("Stopped at \(name): av import requires nonempty UTF-8 without NUL, at most 1 MiB.")
    }

    let pipe = Pipe()
    let child = Process()
    child.executableURL = URL(fileURLWithPath: av)
    child.arguments = ["save", "--stdin", name]
    child.standardInput = pipe
    do {
        try child.run()
        try pipe.fileHandleForReading.close()
        try pipe.fileHandleForWriting.write(contentsOf: data)
        try pipe.fileHandleForWriting.close()
        child.waitUntilExit()
    } catch {
        try? pipe.fileHandleForWriting.close()
        if child.isRunning { child.terminate(); child.waitUntilExit() }
        fail("Stopped at \(name): could not complete av import.")
    }
    guard child.terminationReason == .exit, child.terminationStatus == 0 else {
        fail("Stopped at \(name): av import failed or Approval was denied.")
    }
    print("Imported \(name); original legacy item retained.")
}
