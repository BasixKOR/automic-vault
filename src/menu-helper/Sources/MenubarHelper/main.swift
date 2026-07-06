import AppKit
import CProcessInfo
import CoreServices
import CryptoKit
import Darwin
import Foundation
import MenubarHelperCore
import Security
@preconcurrency import XPC

private let socketPath = "/tmp/com.automicvault.av2.credential-helper.\(getuid()).sock"
private let approvalServiceName = "com.automicvault.av2.approval"
private let legacyTrustedScriptApprovalsDefaultsKey = "TrustedLauncherScriptApprovals"
private let scanQueue = DispatchQueue(label: "com.automicvault.av2.scan")
private var toastWindows: [NSWindow] = []

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
    private let scanStatusItem = NSMenuItem(title: "Scan pending", action: nil, keyEquivalent: "")
    private let broker = CredentialBroker(socketPath: socketPath)
    private var approval: ApprovalServer?
    private var scanWorkItem: DispatchWorkItem?
    private var eventStream: FSEventStreamRef?
    private var mainWindow: NSWindow?

    func applicationDidFinishLaunching(_ notification: Notification) {
        UserDefaults.standard.removeObject(forKey: legacyTrustedScriptApprovalsDefaultsKey)

        statusItem.button?.image = menuImage()

        let menu = NSMenu()
        let title = NSMenuItem(title: "Automic Vault", action: nil, keyEquivalent: "")
        title.isEnabled = false
        menu.addItem(title)
        menu.addItem(NSMenuItem(title: "Credential broker running", action: nil, keyEquivalent: ""))
        menu.addItem(scanStatusItem)
        menu.addItem(.separator())
        let openItem = NSMenuItem(title: "Open Automic Vault", action: #selector(openMainWindow), keyEquivalent: "")
        openItem.target = self
        menu.addItem(openItem)
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(title: "Quit", action: #selector(quit), keyEquivalent: "q"))
        statusItem.menu = menu

        do {
            let approval = try ApprovalServer(serviceName: approvalServiceName)
            try approval.start()
            try broker.start()
            self.approval = approval
            scheduleScan(after: 0)
            startHomeWatcher()
        } catch {
            NSAlert(error: error).runModal()
            NSApp.terminate(nil)
        }
    }

    func applicationWillTerminate(_ notification: Notification) {
        if let eventStream {
            FSEventStreamStop(eventStream)
            FSEventStreamInvalidate(eventStream)
            FSEventStreamRelease(eventStream)
        }
        approval?.stop()
        broker.stop()
    }

    @MainActor @objc private func quit() {
        NSApp.terminate(nil)
    }

    @MainActor @objc private func openMainWindow() {
        if let mainWindow {
            mainWindow.makeKeyAndOrderFront(nil)
            NSApp.activate(ignoringOtherApps: true)
            return
        }

        let controller = AutomicVaultMainWindowController()
        let window = AutomicVaultWindow(
            contentRect: NSRect(origin: .zero, size: NSSize(width: 1120, height: 760)),
            styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
            backing: .buffered,
            defer: false
        )
        window.contentViewController = controller
        window.title = "Automic Vault"
        window.titlebarAppearsTransparent = true
        window.titleVisibility = .hidden
        window.titlebarSeparatorStyle = .none
        window.toolbarStyle = .automatic
        window.toolbar = controller.makeToolbar()
        window.isMovableByWindowBackground = true
        window.isReleasedWhenClosed = false
        window.minSize = NSSize(width: 860, height: 560)
        window.center()
        window.makeKeyAndOrderFront(nil)
        self.mainWindow = window
        NSApp.activate(ignoringOtherApps: true)
    }

    private func menuImage(alerted: Bool = false) -> NSImage? {
        let url = Bundle.main.url(forResource: "NSMenuItem", withExtension: "png")
        guard let url, let image = NSImage(contentsOf: url) else { return nil }
        image.size = NSSize(width: 15, height: 18)
        guard alerted else {
            image.isTemplate = true
            return image
        }

        let tinted = NSImage(size: image.size, flipped: false) { rect in
            image.draw(in: rect)
            NSColor.systemRed.setFill()
            rect.fill(using: .sourceIn)
            return true
        }
        tinted.isTemplate = false
        return tinted
    }

    private func startHomeWatcher() {
        // ponytail: one home FSEvents stream; add detector path metadata if rescans get noisy.
        var context = FSEventStreamContext(
            version: 0,
            info: Unmanaged.passUnretained(self).toOpaque(),
            retain: nil,
            release: nil,
            copyDescription: nil
        )
        let callback: FSEventStreamCallback = { _, info, _, _, _, _ in
            guard let info else { return }
            MainActor.assumeIsolated {
                Unmanaged<AppDelegate>.fromOpaque(info).takeUnretainedValue().scheduleScan(after: 1)
            }
        }
        guard let stream = FSEventStreamCreate(
            nil,
            callback,
            &context,
            [FileManager.default.homeDirectoryForCurrentUser.path] as CFArray,
            FSEventStreamEventId(kFSEventStreamEventIdSinceNow),
            1,
            FSEventStreamCreateFlags(kFSEventStreamCreateFlagFileEvents | kFSEventStreamCreateFlagUseCFTypes)
        ) else {
            scanStatusItem.title = "Scan watcher unavailable"
            return
        }
        eventStream = stream
        FSEventStreamSetDispatchQueue(stream, DispatchQueue.main)
        FSEventStreamStart(stream)
    }

    private func scheduleScan(after delay: TimeInterval) {
        scanWorkItem?.cancel()
        let workItem = DispatchWorkItem { [weak self] in
            self?.runScan()
        }
        scanWorkItem = workItem
        DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: workItem)
    }

    private func runScan() {
        scanQueue.async { [weak self] in
            let result = scanResult()
            Task { @MainActor in
                self?.applyScanResult(result)
            }
        }
    }

    private func applyScanResult(_ result: ScanResult) {
        switch result {
        case .clean:
            statusItem.button?.image = menuImage()
            scanStatusItem.title = "No scan findings"
        case .findings(let count):
            statusItem.button?.image = menuImage(alerted: true)
            scanStatusItem.title = count == 1 ? "1 scan finding" : "\(count) scan findings"
        case .failed:
            statusItem.button?.image = menuImage(alerted: true)
            scanStatusItem.title = "Scan failed"
        }
    }
}

private enum ScanResult {
    case clean
    case findings(Int)
    case failed
}

private func scanResult() -> ScanResult {
    let process = Process()
    process.executableURL = avExecutableURL()
    process.arguments = ["scan", "--json"]

    let output = Pipe()
    process.standardOutput = output
    process.standardError = Pipe()

    do {
        try process.run()
    } catch {
        return .failed
    }

    let data = output.fileHandleForReading.readDataToEndOfFile()
    process.waitUntilExit()
    guard process.terminationStatus == 0,
          let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
          let findings = object["findings"] as? [Any]
    else {
        return .failed
    }
    return findings.isEmpty ? .clean : .findings(findings.count)
}

private func avExecutableURL() -> URL {
    if let bundled = Bundle.main.executableURL?.deletingLastPathComponent().appendingPathComponent("av"),
       FileManager.default.isExecutableFile(atPath: bundled.path)
    {
        return bundled
    }
    return URL(fileURLWithPath: "/usr/local/bin/av")
}

private struct ApprovalRequest {
    let keys: [String]
    let target: String
    let args: [String]
    let cwd: String
    let replaceExistingEnv: Bool
    let allowMissingKeys: Bool
    let envConflicts: [String]
    let shebangScript: String?
}

private struct SigningInfo {
    let identifier: String
    let teamIdentifier: String
}

private struct LauncherIdentity {
    let pid: pid_t
    let path: String
    let identifier: String
    let teamIdentifier: String
    let designatedRequirement: String
}

private struct ScriptApproval {
    let path: String
    let checksum: String
}

private enum ApprovalDecision: Equatable {
    case denied
    case approved
    case alwaysAllow
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
        let scriptApproval = scriptApproval(for: request)
        let launcher = launcherIdentity(startingAt: identity.ppid)
        let trustedApproval = trustedApprovalRecord(
            script: scriptApproval,
            request: request,
            launcher: launcher
        )
        if let scriptApproval, let launcher, let trustedApproval, alwaysAllows(trustedApproval) {
            DispatchQueue.main.async {
                showAutoApprovedToast(
                    keys: request.keys,
                    script: scriptApproval.path,
                    launcher: launcher.identifier
                )
                self.reply(peer, to: message, ok: true, error: nil)
            }
            return
        }

        DispatchQueue.main.async {
            let decision = showApprovalAlert(
                request: request,
                callerPath: callerPath,
                pid: pid,
                signing: signing,
                scriptApproval: scriptApproval,
                launcher: launcher
            )
            if decision == .alwaysAllow, let trustedApproval {
                rememberAlwaysAllow(trustedApproval)
            }
            self.reply(
                peer,
                to: message,
                ok: decision != .denied,
                error: decision == .denied ? "injection denied" : nil
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
            envConflicts: envConflicts,
            shebangScript: xpc_dictionary_get_string(message, "shebang_script").map(String.init(cString:))
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

private func launcherIdentity(startingAt startPID: pid_t) -> LauncherIdentity? {
    var pid = startPID
    var seen = Set<pid_t>()
    for _ in 0..<32 {
        guard pid > 1, seen.insert(pid).inserted else { return nil }

        var identity = AVProcessIdentity()
        guard av_process_identity(pid, &identity) else { return nil }
        let path = pathString(identity)
        if let signing = liveSigningInfo(pid: pid),
           isAppBundleExecutable(path) || isAppBundleExecutable(signing.mainExecutable)
        {
            return LauncherIdentity(
                pid: pid,
                path: path,
                identifier: signing.identifier,
                teamIdentifier: signing.teamIdentifier,
                designatedRequirement: signing.designatedRequirement
            )
        }
        pid = identity.ppid
    }
    return nil
}

private struct LiveSigningInfo {
    let identifier: String
    let teamIdentifier: String
    let designatedRequirement: String
    let mainExecutable: String
}

private func liveSigningInfo(pid: pid_t) -> LiveSigningInfo? {
    var code: SecCode?
    let attributes = [kSecGuestAttributePid as String: NSNumber(value: pid)] as CFDictionary
    guard SecCodeCopyGuestWithAttributes(nil, attributes, [], &code) == errSecSuccess,
          let code
    else {
        return nil
    }
    guard SecCodeCheckValidity(code, [], nil) == errSecSuccess else { return nil }

    var staticCode: SecStaticCode?
    guard SecCodeCopyStaticCode(code, [], &staticCode) == errSecSuccess,
          let staticCode
    else {
        return nil
    }

    var info: CFDictionary?
    let flags = SecCSFlags(rawValue: kSecCSSigningInformation | kSecCSRequirementInformation)
    guard SecCodeCopySigningInformation(staticCode, flags, &info) == errSecSuccess,
          let dictionary = info as? [CFString: Any],
          let requirementValue = dictionary[kSecCodeInfoDesignatedRequirement]
    else {
        return nil
    }
    let requirement = requirementValue as! SecRequirement
    guard let requirementText = requirementString(requirement) else { return nil }

    let executable = (dictionary[kSecCodeInfoMainExecutable] as? URL)?.path ?? ""
    return LiveSigningInfo(
        identifier: dictionary[kSecCodeInfoIdentifier] as? String ?? "unknown",
        teamIdentifier: dictionary[kSecCodeInfoTeamIdentifier] as? String ?? "unknown",
        designatedRequirement: requirementText,
        mainExecutable: executable
    )
}

private func requirementString(_ requirement: SecRequirement) -> String? {
    var text: CFString?
    guard SecRequirementCopyString(requirement, [], &text) == errSecSuccess,
          let text
    else {
        return nil
    }
    return text as String
}

private func isAppBundleExecutable(_ path: String) -> Bool {
    path.range(of: ".app/Contents/", options: [.caseInsensitive]) != nil
}

private func scriptApproval(for request: ApprovalRequest) -> ScriptApproval? {
    guard let script = request.shebangScript else { return nil }
    let url = script.hasPrefix("/")
        ? URL(fileURLWithPath: script)
        : URL(fileURLWithPath: request.cwd).appendingPathComponent(script)
    let path = url.standardizedFileURL.path
    guard let data = try? Data(contentsOf: URL(fileURLWithPath: path)) else { return nil }
    let checksum = SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
    return ScriptApproval(path: path, checksum: checksum)
}

private func trustedApprovalRecord(
    script: ScriptApproval?,
    request: ApprovalRequest,
    launcher: LauncherIdentity?
) -> TrustedScriptApproval? {
    guard let script, let launcher else { return nil }
    return TrustedScriptApproval(
        scriptPath: script.path,
        scriptChecksum: script.checksum,
        keys: request.keys.sorted(),
        target: request.target,
        replaceExistingEnv: request.replaceExistingEnv,
        allowMissingKeys: request.allowMissingKeys,
        launcherRequirement: launcher.designatedRequirement
    )
}

private func alwaysAllows(
    _ approval: TrustedScriptApproval,
    service: String = trustedScriptApprovalsKeychainService,
    account: String = trustedScriptApprovalsKeychainAccount
) -> Bool {
    loadTrustedScriptApprovals(service: service, account: account).contains(approval)
}

private func rememberAlwaysAllow(
    _ approval: TrustedScriptApproval,
    service: String = trustedScriptApprovalsKeychainService,
    account: String = trustedScriptApprovalsKeychainAccount
) {
    var approvals = loadTrustedScriptApprovals(service: service, account: account)
    if !approvals.contains(approval) {
        approvals.append(approval)
    }
    _ = saveTrustedScriptApprovals(approvals, service: service, account: account)
}

@MainActor
private func showApprovalAlert(
    request: ApprovalRequest,
    callerPath: String,
    pid: pid_t,
    signing: SigningInfo,
    scriptApproval: ScriptApproval?,
    launcher: LauncherIdentity?
) -> ApprovalDecision {
    NSApp.activate(ignoringOtherApps: true)

    let alert = NSAlert()
    alert.alertStyle = .warning
    alert.messageText = "Approve secret injection?"
    var lines = [
        "Caller: \(callerPath) (pid \(pid))",
        "Signed: \(signing.identifier) / \(signing.teamIdentifier)",
        "Target: \(request.target)",
        "Arguments: \(request.args.isEmpty ? "(none)" : request.args.joined(separator: " "))",
        "Working directory: \(request.cwd)",
        "Keys: \(request.keys.joined(separator: ", "))",
        "Existing environment: \(request.envConflicts.isEmpty ? "(none)" : request.envConflicts.joined(separator: ", "))",
        "Replace existing environment: \(request.replaceExistingEnv ? "yes" : "no")",
        "Allow missing keys: \(request.allowMissingKeys ? "yes" : "no")",
    ]
    if let launcher {
        lines.append("Launcher: \(launcher.identifier) (pid \(launcher.pid))")
        lines.append("Launcher path: \(launcher.path)")
        lines.append("Launcher signed: \(launcher.identifier) / \(launcher.teamIdentifier)")
    } else {
        lines.append("Launcher: unavailable; persistent auto-approve disabled")
    }
    if let scriptApproval {
        lines.append("Script: \(scriptApproval.path)")
        lines.append("Script checksum: \(scriptApproval.checksum)")
    } else if let script = request.shebangScript {
        lines.append("Script: \(script)")
        lines.append("Script checksum: unavailable")
    }
    alert.informativeText = lines.joined(separator: "\n")
    alert.addButton(withTitle: "Deny")
    alert.addButton(withTitle: "Approve")
    if scriptApproval != nil, launcher != nil {
        alert.addButton(withTitle: "Always Allow From This App")
    }
    switch alert.runModal() {
    case .alertSecondButtonReturn:
        return .approved
    case .alertThirdButtonReturn:
        return .alwaysAllow
    default:
        return .denied
    }
}

@MainActor
private func showAutoApprovedToast(keys: [String], script: String, launcher: String) {
    let text = "Auto approved \(keys.joined(separator: ", ")) for \(script) from \(launcher)"
    let width = min(max((text as NSString).size(withAttributes: [.font: NSFont.systemFont(ofSize: 13, weight: .medium)]).width + 28, 280), 640)
    let height: CGFloat = 38

    let label = NSTextField(labelWithString: text)
    label.frame = NSRect(x: 12, y: 0, width: width - 24, height: height)
    label.autoresizingMask = [.width, .height]
    label.lineBreakMode = .byTruncatingMiddle
    label.maximumNumberOfLines = 1
    label.textColor = .labelColor
    label.font = .systemFont(ofSize: 13, weight: .medium)

    let box = NSBox()
    box.frame = NSRect(x: 0, y: 0, width: width, height: height)
    box.autoresizingMask = [.width, .height]
    box.boxType = .custom
    box.cornerRadius = 8
    box.borderWidth = 1
    box.borderColor = .separatorColor
    box.fillColor = .windowBackgroundColor

    let content = NSView(frame: NSRect(x: 0, y: 0, width: width, height: height))
    content.addSubview(box)
    content.addSubview(label)

    let screenFrame = NSScreen.main?.visibleFrame ?? NSRect(x: 0, y: 0, width: 800, height: 600)
    let frame = NSRect(
        x: screenFrame.midX - width / 2,
        y: screenFrame.maxY - height - 8,
        width: width,
        height: height
    )
    let window = NSPanel(
        contentRect: frame,
        styleMask: [.borderless, .nonactivatingPanel],
        backing: .buffered,
        defer: false
    )
    window.level = .statusBar
    window.isOpaque = false
    window.backgroundColor = .clear
    window.hasShadow = true
    window.contentView = content
    toastWindows.append(window)
    window.orderFront(nil)

    DispatchQueue.main.asyncAfter(deadline: .now() + 2.0) {
        window.orderOut(nil)
        toastWindows.removeAll { $0 === window }
    }
}

private func runApprovalSelfCheck() -> Int32 {
    let service = "com.automicvault.av2.approval-self-check.\(UUID().uuidString)"
    defer { _ = deleteStoredSecret(account: trustedScriptApprovalsKeychainAccount, service: service) }

    let request = ApprovalRequest(
        keys: ["B", "A"],
        target: "/bin/echo",
        args: ["ignored"],
        cwd: "/tmp",
        replaceExistingEnv: true,
        allowMissingKeys: false,
        envConflicts: ["ignored"],
        shebangScript: "/tmp/deploy"
    )
    let script = ScriptApproval(path: "/tmp/deploy", checksum: "abc")
    let launcher = LauncherIdentity(
        pid: 42,
        path: "/Applications/Codex.app/Contents/MacOS/Codex",
        identifier: "com.openai.codex",
        teamIdentifier: "TEAM",
        designatedRequirement: #"identifier "com.openai.codex" and anchor apple generic"#
    )
    guard let approval = trustedApprovalRecord(
        script: script,
        request: request,
        launcher: launcher
    ) else {
        return 1
    }

    func altered(
        checksum: String = "abc",
        keys: [String] = ["A", "B"],
        target: String = "/bin/echo",
        replaceExistingEnv: Bool = true,
        allowMissingKeys: Bool = false,
        launcherRequirement: String = #"identifier "com.openai.codex" and anchor apple generic"#
    ) -> TrustedScriptApproval {
        TrustedScriptApproval(
            scriptPath: "/tmp/deploy",
            scriptChecksum: checksum,
            keys: keys,
            target: target,
            replaceExistingEnv: replaceExistingEnv,
            allowMissingKeys: allowMissingKeys,
            launcherRequirement: launcherRequirement
        )
    }

    guard approval.keys == ["A", "B"],
          trustedApprovalRecord(script: script, request: request, launcher: nil) == nil,
          isAppBundleExecutable("/Applications/Vaultty.app/Contents/Helpers/vaultty-sessiond"),
          !alwaysAllows(approval, service: service)
    else {
        return 1
    }

    rememberAlwaysAllow(approval, service: service)
    guard alwaysAllows(approval, service: service),
          !alwaysAllows(altered(checksum: "def"), service: service),
          !alwaysAllows(altered(keys: ["A"]), service: service),
          !alwaysAllows(altered(target: "/usr/bin/env"), service: service),
          !alwaysAllows(altered(replaceExistingEnv: false), service: service),
          !alwaysAllows(altered(allowMissingKeys: true), service: service),
          !alwaysAllows(altered(launcherRequirement: #"identifier "com.apple.Terminal""#), service: service)
    else {
        return 1
    }
    return 0
}

if CommandLine.arguments.contains("--self-check-approvals") {
    exit(runApprovalSelfCheck())
}

let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.setActivationPolicy(.accessory)
app.run()
