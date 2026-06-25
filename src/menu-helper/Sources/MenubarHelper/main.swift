import AppKit
import CProcessInfo
import Darwin
import Foundation
import Security

private let socketPath = "/tmp/com.automicvault.av2.credential-helper.\(getuid()).sock"

final class AppDelegate: NSObject, NSApplicationDelegate {
    private let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
    private let broker = CredentialBroker(socketPath: socketPath)

    func applicationDidFinishLaunching(_ notification: Notification) {
        if let button = statusItem.button {
            button.image = menuImage()
        }

        let menu = NSMenu()
        let title = NSMenuItem(title: "Automic Vault", action: nil, keyEquivalent: "")
        title.isEnabled = false
        menu.addItem(title)
        menu.addItem(NSMenuItem(title: "Credential broker running", action: nil, keyEquivalent: ""))
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(title: "Quit", action: #selector(quit), keyEquivalent: "q"))
        statusItem.menu = menu

        do {
            try broker.start()
        } catch {
            NSAlert(error: error).runModal()
            NSApp.terminate(nil)
        }
    }

    func applicationWillTerminate(_ notification: Notification) {
        broker.stop()
    }

    @MainActor @objc private func quit() {
        NSApp.terminate(nil)
    }

    private func menuImage() -> NSImage? {
        let url = Bundle.main.url(forResource: "NSMenuItem", withExtension: "png")
        guard let url, let image = NSImage(contentsOf: url) else { return nil }
        image.isTemplate = true
        image.size = NSSize(width: 15, height: 18)
        return image
    }
}

private struct Nonce {
    let tool: String
    let pid: pid_t
    let startUsec: UInt64
    let expiresAt: Date
}

private final class CredentialBroker: @unchecked Sendable {
    private let socketPath: String
    private let queue = DispatchQueue(label: "com.automicvault.credential-broker")
    private var socketFD: Int32 = -1
    private var nonces: [String: Nonce] = [:]

    init(socketPath: String) {
        self.socketPath = socketPath
    }

    func start() throws {
        unlink(socketPath)
        socketFD = socket(AF_UNIX, SOCK_STREAM, 0)
        guard socketFD >= 0 else { throw BrokerError("socket failed") }

        var addr = sockaddr_un()
        addr.sun_family = sa_family_t(AF_UNIX)
        let pathBytes = Array(socketPath.utf8)
        guard pathBytes.count < MemoryLayout.size(ofValue: addr.sun_path) else {
            throw BrokerError("socket path too long")
        }
        withUnsafeMutableBytes(of: &addr.sun_path) { buffer in
            buffer.copyBytes(from: pathBytes)
            buffer[pathBytes.count] = 0
        }

        let bound = withUnsafePointer(to: &addr) { pointer in
            pointer.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                bind(socketFD, $0, socklen_t(MemoryLayout<sa_family_t>.size + pathBytes.count + 1))
            }
        }
        guard bound == 0 else { throw BrokerError("bind failed at \(socketPath)") }
        chmod(socketPath, 0o600)
        guard listen(socketFD, 16) == 0 else { throw BrokerError("listen failed") }

        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            self?.acceptLoop()
        }
    }

    func stop() {
        if socketFD >= 0 {
            close(socketFD)
            socketFD = -1
        }
        unlink(socketPath)
    }

    private func acceptLoop() {
        while socketFD >= 0 {
            let client = accept(socketFD, nil, nil)
            if client >= 0 {
                handle(client)
            }
        }
    }

    private func handle(_ fd: Int32) {
        var peer: pid_t = 0
        guard av_peer_pid(fd, &peer) else {
            respond(fd, "err missing peer pid\n")
            close(fd)
            return
        }
        var identity = AVProcessIdentity()
        guard av_process_identity(peer, &identity) else {
            respond(fd, "err missing peer identity\n")
            close(fd)
            return
        }

        let request = readRequest(fd)
        let response = queue.sync { process(request, from: identity) }
        respond(fd, response)
        close(fd)
    }

    private func process(_ request: String, from identity: AVProcessIdentity) -> String {
        cleanup()
        let parts = request
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .split(separator: " ", maxSplits: 2, omittingEmptySubsequences: true)
            .map(String.init)
        guard let command = parts.first else { return "err empty request\n" }

        switch command {
        case "mint":
            guard parts.count == 3 else { return "err invalid mint request\n" }
            return mint(tool: parts[1], target: parts[2], identity: identity)
        case "validate":
            guard parts.count == 3 else { return "err invalid validate request\n" }
            return validate(tool: parts[1], token: parts[2], helper: identity)
        default:
            return "err unknown request\n"
        }
    }

    private func mint(tool: String, target: String, identity: AVProcessIdentity) -> String {
        guard tool == "aws" else { return "err unsupported tool\n" }
        let path = pathString(identity)
        guard path == "/usr/local/bin/av" else {
            return "err nonce requester is not /usr/local/bin/av\n"
        }
        guard signedByAutomicVaultCLI(path) else {
            return "err /usr/local/bin/av is not signed as com.automicvault.av\n"
        }

        let stub = "/usr/local/bin/\(tool)"
        let argv = argvLines(identity)
        guard argv.dropFirst().prefix(3).elementsEqual(["stub-exec", tool, target])
        else {
            return "err nonce requester was not launched by hardened stub\n"
        }
        guard let script = try? String(contentsOfFile: stub, encoding: .utf8),
              script.contains("# Automic Vault hardened stub\n"),
              script.contains("exec /usr/local/bin/av stub-exec '\(tool)' '\(shellQuote(target))'")
        else {
            return "err hardened stub does not match requested target\n"
        }
        guard standardUserCannotWrite(path), standardUserCannotWrite(stub) else {
            return "err hardened stub path is writable by the standard user\n"
        }
        guard standardUserCannotWrite(target) else {
            return "err hardened target path is writable by the standard user\n"
        }

        guard let token = randomToken() else { return "err token generation failed\n" }
        nonces[token] = Nonce(
            tool: tool,
            pid: identity.pid,
            startUsec: identity.start_usec,
            expiresAt: Date().addingTimeInterval(300)
        )
        return "ok \(token)\n"
    }

    private func validate(tool: String, token: String, helper: AVProcessIdentity) -> String {
        guard let nonce = nonces[token] else { return "err unknown credential helper token\n" }
        guard nonce.tool == tool else { return "err credential helper token is for another tool\n" }
        guard nonce.expiresAt > Date() else {
            nonces.removeValue(forKey: token)
            return "err expired credential helper token\n"
        }

        var parent = AVProcessIdentity()
        guard av_process_identity(helper.ppid, &parent) else {
            return "err missing credential helper parent\n"
        }
        guard parent.pid == nonce.pid && parent.start_usec == nonce.startUsec else {
            return "err credential helper parent does not match token owner\n"
        }
        return "ok valid\n"
    }

    private func cleanup() {
        let now = Date()
        nonces = nonces.filter { _, nonce in nonce.expiresAt > now }
    }

    private func readRequest(_ fd: Int32) -> String {
        var bytes = [UInt8](repeating: 0, count: 4096)
        let count = read(fd, &bytes, bytes.count)
        guard count > 0 else { return "" }
        return String(decoding: bytes.prefix(count), as: UTF8.self)
    }

    private func respond(_ fd: Int32, _ response: String) {
        _ = response.withCString { write(fd, $0, strlen($0)) }
    }

    private func randomToken() -> String? {
        var bytes = [UInt8](repeating: 0, count: 32)
        guard SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes) == errSecSuccess else {
            return nil
        }
        return bytes.map { String(format: "%02x", $0) }.joined()
    }
}

private struct BrokerError: LocalizedError {
    let errorDescription: String?

    init(_ description: String) {
        errorDescription = description
    }
}

private func pathString(_ identity: AVProcessIdentity) -> String {
    var copy = identity
    return withUnsafePointer(to: &copy.path) { pointer in
        pointer.withMemoryRebound(to: CChar.self, capacity: 4096) {
            String(cString: $0)
        }
    }
}

private func argvLines(_ identity: AVProcessIdentity) -> [String] {
    var buffer = [CChar](repeating: 0, count: 8192)
    guard av_process_arguments(identity.pid, &buffer, buffer.count) else {
        return []
    }
    let end = buffer.firstIndex(of: 0) ?? buffer.count
    return String(decoding: buffer[..<end].map(UInt8.init(bitPattern:)), as: UTF8.self)
        .split(separator: "\n")
        .map(String.init)
}

private func shellQuote(_ value: String) -> String {
    value.replacingOccurrences(of: "'", with: "'\\''")
}

private func standardUserCannotWrite(_ path: String) -> Bool {
    let parent = URL(fileURLWithPath: path).deletingLastPathComponent().path
    return access(path, W_OK) != 0 && access(parent, W_OK) != 0
}

private func signedByAutomicVaultCLI(_ path: String) -> Bool {
    let requirement = """
    identifier "com.automicvault.av" and anchor apple generic and \
    certificate leaf[subject.OU] = ZU76A67LGU
    """
    return satisfiesRequirement(path, requirement)
}

private func satisfiesRequirement(_ path: String, _ requirement: String) -> Bool {
    var staticCode: SecStaticCode?
    let url = URL(fileURLWithPath: path) as CFURL
    guard SecStaticCodeCreateWithPath(url, [], &staticCode) == errSecSuccess,
          let staticCode
    else {
        return false
    }

    var secRequirement: SecRequirement?
    guard SecRequirementCreateWithString(requirement as CFString, [], &secRequirement) == errSecSuccess,
          let secRequirement
    else {
        return false
    }

    return SecStaticCodeCheckValidity(staticCode, [], secRequirement) == errSecSuccess
}

let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.setActivationPolicy(.accessory)
app.run()
