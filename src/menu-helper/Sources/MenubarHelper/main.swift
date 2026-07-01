import AppKit
import CProcessInfo
import Darwin
import Foundation
import Security
@preconcurrency import XPC

private let socketPath = "/tmp/com.automicvault.av2.credential-helper.\(getuid()).sock"
private let approvalServiceName = "com.automicvault.av2.approval"

final class AppDelegate: NSObject, NSApplicationDelegate {
    private let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
    private let broker = CredentialBroker(socketPath: socketPath)
    private var approval: ApprovalServer?

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
            let approval = try ApprovalServer(serviceName: approvalServiceName)
            try approval.start()
            try broker.start()
            self.approval = approval
        } catch {
            NSAlert(error: error).runModal()
            NSApp.terminate(nil)
        }
    }

    func applicationWillTerminate(_ notification: Notification) {
        approval?.stop()
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

private struct ApprovalRequest {
    let keys: [String]
    let target: String
    let args: [String]
    let cwd: String
    let replaceExistingEnv: Bool
    let allowMissingKeys: Bool
    let envConflicts: [String]
}

private struct SigningInfo {
    let identifier: String
    let teamIdentifier: String
}

private final class ApprovalServer: @unchecked Sendable {
    private let serviceName: String
    private let teamIdentifier: String
    private var listener: xpc_connection_t?

    init(serviceName: String) throws {
        guard let teamIdentifier = selfTeamIdentifier() else {
            throw BrokerError("missing menu bar signing team identifier")
        }
        self.serviceName = serviceName
        self.teamIdentifier = teamIdentifier
    }

    func start() throws {
        listener = serviceName.withCString {
            xpc_connection_create_mach_service(
                $0,
                nil,
                UInt64(XPC_CONNECTION_MACH_SERVICE_LISTENER)
            )
        }
        guard let listener else { throw BrokerError("approval XPC listener failed") }

        let requirement = """
        identifier "com.automicvault.av" and anchor apple generic and \
        certificate leaf[subject.OU] = \(teamIdentifier)
        """
        let status = requirement.withCString {
            xpc_connection_set_peer_code_signing_requirement(listener, $0)
        }
        guard status == 0 else {
            throw BrokerError("approval XPC signing requirement failed")
        }

        xpc_connection_set_event_handler(listener) { [weak self] event in
            self?.accept(event)
        }
        xpc_connection_activate(listener)
    }

    func stop() {
        if let listener {
            xpc_connection_cancel(listener)
            self.listener = nil
        }
    }

    private func accept(_ event: xpc_object_t) {
        guard xpc_get_type(event) == XPC_TYPE_CONNECTION else { return }
        let peer = event
        xpc_connection_set_event_handler(peer) { [weak self] message in
            self?.handle(message, on: peer)
        }
        xpc_connection_activate(peer)
    }

    private func handle(_ message: xpc_object_t, on peer: xpc_connection_t) {
        guard xpc_get_type(message) == XPC_TYPE_DICTIONARY else { return }

        let pid = xpc_connection_get_pid(peer)
        var identity = AVProcessIdentity()
        guard av_process_identity(pid, &identity) else {
            reply(peer, to: message, ok: false, error: "approval caller identity is unavailable")
            return
        }

        let callerPath = pathString(identity)
        guard callerPath == "/usr/local/bin/av"
                || URL(fileURLWithPath: callerPath).lastPathComponent == "av"
        else {
            reply(peer, to: message, ok: false, error: "approval caller is not av")
            return
        }

        let signing = signingInfo(path: callerPath)
        guard signing.identifier == "com.automicvault.av",
              signing.teamIdentifier == teamIdentifier
        else {
            reply(peer, to: message, ok: false, error: "approval caller is not signed as av")
            return
        }

        guard let request = approvalRequest(from: message) else {
            reply(peer, to: message, ok: false, error: "invalid approval request")
            return
        }

        DispatchQueue.main.async {
            let approved = showApprovalAlert(
                request: request,
                callerPath: callerPath,
                pid: pid,
                signing: signing
            )
            self.reply(
                peer,
                to: message,
                ok: approved,
                error: approved ? nil : "injection denied"
            )
        }
    }

    private func approvalRequest(from message: xpc_object_t) -> ApprovalRequest? {
        guard let opPointer = xpc_dictionary_get_string(message, "op"),
              String(cString: opPointer) == "inject",
              let targetPointer = xpc_dictionary_get_string(message, "target"),
              let cwdPointer = xpc_dictionary_get_string(message, "cwd"),
              let keys = stringArray(message, "keys"),
              let args = stringArray(message, "args"),
              let envConflicts = stringArray(message, "env_conflicts")
        else {
            return nil
        }

        return ApprovalRequest(
            keys: keys,
            target: String(cString: targetPointer),
            args: args,
            cwd: String(cString: cwdPointer),
            replaceExistingEnv: xpc_dictionary_get_bool(message, "replace_existing_env"),
            allowMissingKeys: xpc_dictionary_get_bool(message, "allow_missing_keys"),
            envConflicts: envConflicts
        )
    }

    private func stringArray(_ message: xpc_object_t, _ key: String) -> [String]? {
        guard let value = xpc_dictionary_get_value(message, key),
              xpc_get_type(value) == XPC_TYPE_ARRAY
        else {
            return nil
        }
        var strings: [String] = []
        for index in 0..<xpc_array_get_count(value) {
            guard let pointer = xpc_array_get_string(value, index) else { return nil }
            strings.append(String(cString: pointer))
        }
        return strings
    }

    private func reply(_ peer: xpc_connection_t, to message: xpc_object_t, ok: Bool, error: String?) {
        let response = xpc_dictionary_create_reply(message) ?? xpc_dictionary_create_empty()
        xpc_dictionary_set_bool(response, "ok", ok)
        if let error {
            error.withCString {
                xpc_dictionary_set_string(response, "error", $0)
            }
        }
        xpc_connection_send_message(peer, response)
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
    guard let teamIdentifier = selfTeamIdentifier() else { return false }
    let requirement = """
    identifier "com.automicvault.av" and anchor apple generic and \
    certificate leaf[subject.OU] = \(teamIdentifier)
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

private func signingInfo(path: String) -> SigningInfo {
    var staticCode: SecStaticCode?
    let url = URL(fileURLWithPath: path) as CFURL
    guard SecStaticCodeCreateWithPath(url, [], &staticCode) == errSecSuccess,
          let staticCode,
          let info = copySigningInformation(staticCode)
    else {
        return SigningInfo(identifier: "unknown", teamIdentifier: "unknown")
    }

    return SigningInfo(
        identifier: info[kSecCodeInfoIdentifier] as? String ?? "unknown",
        teamIdentifier: info[kSecCodeInfoTeamIdentifier] as? String ?? "unknown"
    )
}

private func selfTeamIdentifier() -> String? {
    var code: SecCode?
    var staticCode: SecStaticCode?
    guard SecCodeCopySelf([], &code) == errSecSuccess,
          let code,
          SecCodeCopyStaticCode(code, [], &staticCode) == errSecSuccess,
          let staticCode,
          let info = copySigningInformation(staticCode)
    else {
        return nil
    }
    return info[kSecCodeInfoTeamIdentifier] as? String
}

private func copySigningInformation(_ code: SecStaticCode) -> [CFString: Any]? {
    var info: CFDictionary?
    guard SecCodeCopySigningInformation(
        code,
        SecCSFlags(rawValue: kSecCSSigningInformation),
        &info
    ) == errSecSuccess else {
        return nil
    }
    return info as? [CFString: Any]
}

@MainActor
private func showApprovalAlert(
    request: ApprovalRequest,
    callerPath: String,
    pid: pid_t,
    signing: SigningInfo
) -> Bool {
    NSApp.activate(ignoringOtherApps: true)

    let alert = NSAlert()
    alert.alertStyle = .warning
    alert.messageText = "Approve secret injection?"
    alert.informativeText = [
        "Caller: \(callerPath) (pid \(pid))",
        "Signed: \(signing.identifier) / \(signing.teamIdentifier)",
        "Target: \(request.target)",
        "Arguments: \(request.args.isEmpty ? "(none)" : request.args.joined(separator: " "))",
        "Working directory: \(request.cwd)",
        "Keys: \(request.keys.joined(separator: ", "))",
        "Existing environment: \(request.envConflicts.isEmpty ? "(none)" : request.envConflicts.joined(separator: ", "))",
        "Replace existing environment: \(request.replaceExistingEnv ? "yes" : "no")",
        "Allow missing keys: \(request.allowMissingKeys ? "yes" : "no")",
    ].joined(separator: "\n")
    alert.addButton(withTitle: "Deny")
    alert.addButton(withTitle: "Approve")
    return alert.runModal() == .alertSecondButtonReturn
}

let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.setActivationPolicy(.accessory)
app.run()
