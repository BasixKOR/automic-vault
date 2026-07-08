import AppKit
import CProcessInfo
import CoreServices
import CryptoKit
import Darwin
import Foundation
import MenubarHelperCore
import Security
@preconcurrency import XPC

private let approvalServiceName = "com.automicvault.av2.approval"
private let approvalLaunchAgentName = "com.automicvault.menubar-helper"
private let legacyTrustedScriptApprovalsDefaultsKey = "TrustedLauncherScriptApprovals"
private let secCodeSignatureAdHoc: UInt32 = 0x2
private let transientApprovalTTL: TimeInterval = 5 * 60
private let scanQueue = DispatchQueue(label: "com.automicvault.av2.scan")
private var toastWindows: [NSWindow] = []

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private lazy var statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
    private lazy var scanStatusItem = NSMenuItem(title: "Scan pending", action: nil, keyEquivalent: "")
    private var autoApprovalItems: [NSMenuItem] = []
    private var autoApprovalSeparator: NSMenuItem?
    private var autoApprovals: [AutoApprovalRecord] = []
    private let autoApprovalTimeFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.timeStyle = .short
        formatter.dateStyle = .none
        return formatter
    }()
    private var approval: ApprovalServer?
    private var scanWorkItem: DispatchWorkItem?
    private var eventStream: FSEventStreamRef?
    private var mainWindow: NSWindow?

    func applicationDidFinishLaunching(_ notification: Notification) {
        do {
            if try handOffToLaunchAgentIfNeeded() {
                NSApp.terminate(nil)
                return
            }
        } catch {
            NSAlert(error: error).runModal()
            NSApp.terminate(nil)
            return
        }

        UserDefaults.standard.removeObject(forKey: legacyTrustedScriptApprovalsDefaultsKey)

        statusItem.button?.image = menuImage()

        let menu = NSMenu()
        menu.addItem(scanStatusItem)
        menu.addItem(.separator())
        let openItem = NSMenuItem(title: "Open Automic Vault", action: #selector(openMainWindow), keyEquivalent: "")
        openItem.target = self
        menu.addItem(openItem)
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(title: "Quit", action: #selector(quit), keyEquivalent: "q"))
        statusItem.menu = menu

        do {
            let approval = try ApprovalServer(serviceName: approvalServiceName) { [weak self] event in
                self?.recordAutoApproval(event)
            }
            try approval.start()
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
        let defaultWindowSize = NSSize(width: 860, height: 578)
        let window = AutomicVaultWindow(
            contentRect: NSRect(origin: .zero, size: defaultWindowSize),
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
        window.isReleasedWhenClosed = false
        window.minSize = NSSize(width: 860, height: 558)
        window.center()
        window.makeKeyAndOrderFront(nil)
        self.mainWindow = window
        NSApp.activate(ignoringOtherApps: true)
        window.setContentSize(defaultWindowSize)
        window.center()
    }

    private func menuImage(alertColor: NSColor? = nil) -> NSImage? {
        let url = Bundle.main.url(forResource: "NSMenuItem", withExtension: "png")
        guard let url, let image = NSImage(contentsOf: url) else { return nil }
        image.size = NSSize(width: 15, height: 18)
        guard let alertColor else {
            image.isTemplate = true
            return image
        }

        let tinted = NSImage(size: image.size, flipped: false) { rect in
            image.draw(in: rect)
            alertColor.setFill()
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
            scanStatusItem.attributedTitle = nil
            scanStatusItem.image = shieldImage()
            scanStatusItem.title = "No Vulnerabilities Detected"
        case .findings(let count, let level):
            statusItem.button?.image = switch level {
            case .medium: menuImage()
            case .high: menuImage(alertColor: .systemRed)
            }
            scanStatusItem.attributedTitle = nil
            scanStatusItem.image = nil
            scanStatusItem.title = count == 1 ? "1 scan finding" : "\(count) scan findings"
        case .failed:
            statusItem.button?.image = menuImage(alertColor: .systemRed)
            scanStatusItem.attributedTitle = nil
            scanStatusItem.image = nil
            scanStatusItem.title = "Scan failed"
        }
    }

    private func shieldImage() -> NSImage? {
        guard let symbol = NSImage(systemSymbolName: "shield.lefthalf.filled", accessibilityDescription: "SHIELD") else {
            return nil
        }
        let image = symbol.withSymbolConfiguration(.init(pointSize: 14, weight: .semibold)) ?? symbol
        image.size = NSSize(width: 16, height: 16)
        let tinted = NSImage(size: image.size, flipped: false) { rect in
            image.draw(in: rect)
            NSColor.systemGreen.setFill()
            rect.fill(using: .sourceIn)
            return true
        }
        tinted.isTemplate = false
        return tinted
    }

    private func recordAutoApproval(_ record: AutoApprovalRecord) {
        autoApprovals.insert(record, at: 0)
        autoApprovals = Array(autoApprovals.prefix(5))
        refreshAutoApprovalMenuItems()
    }

    private func refreshAutoApprovalMenuItems() {
        guard let menu = statusItem.menu else { return }
        for item in autoApprovalItems {
            menu.removeItem(item)
        }
        if let separator = autoApprovalSeparator {
            menu.removeItem(separator)
            autoApprovalSeparator = nil
        }
        autoApprovalItems = autoApprovals.map {
            let item = NSMenuItem(title: autoApprovalTitle($0, formatter: autoApprovalTimeFormatter), action: nil, keyEquivalent: "")
            item.isEnabled = false
            return item
        }
        for item in autoApprovalItems.reversed() {
            menu.insertItem(item, at: 0)
        }
        if !autoApprovalItems.isEmpty {
            let separator = NSMenuItem.separator()
            menu.insertItem(separator, at: autoApprovalItems.count)
            autoApprovalSeparator = separator
        }
    }
}

private struct AutoApprovalRecord {
    let date: Date
    let launcher: String
    let tool: String
}

private func autoApprovalTitle(_ record: AutoApprovalRecord, formatter: DateFormatter) -> String {
    "\(formatter.string(from: record.date)) – \(record.launcher) used \(record.tool)"
}

private func autoApprovalRecord(
    request: ApprovalRequest,
    script: ScriptApproval?,
    launcher: LauncherIdentity
) -> AutoApprovalRecord {
    AutoApprovalRecord(
        date: Date(),
        launcher: shortAppName(launcher.identifier),
        tool: autoApprovalToolName(request, scriptPath: script?.path)
    )
}

private func shortAppName(_ identifier: String) -> String {
    let name = identifier.split(separator: ".").last.map(String.init) ?? identifier
    return name.prefix(1).uppercased() + name.dropFirst()
}

private func autoApprovalToolName(_ request: ApprovalRequest, scriptPath: String? = nil) -> String {
    if let tool = request.tool {
        return tool
    }
    if let scriptPath {
        return URL(fileURLWithPath: scriptPath).lastPathComponent
    }
    if let scriptPath = resolvedShebangScriptPath(request) {
        return URL(fileURLWithPath: scriptPath).lastPathComponent
    }
    return URL(fileURLWithPath: request.target).lastPathComponent
}

private func resolvedShebangScriptPath(_ request: ApprovalRequest) -> String? {
    guard let script = request.shebangScript else { return nil }
    let url = script.hasPrefix("/")
        ? URL(fileURLWithPath: script)
        : URL(fileURLWithPath: request.cwd).appendingPathComponent(script)
    return url.standardizedFileURL.path
}

private enum ScanResult {
    case clean(Int)
    case findings(Int, ScanAlertLevel)
    case failed
}

private enum ScanAlertLevel {
    case medium
    case high
}

private func scanResult() -> ScanResult {
    let executableURL = avExecutableURL()
    let process = Process()
    process.executableURL = executableURL
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
          let findings = object["findings"] as? [[String: Any]]
    else {
        return .failed
    }
    return findings.isEmpty
        ? .clean(loadDetectorMetadata(avExecutableURL: executableURL).count)
        : .findings(findings.count, scanAlertLevel(findings))
}

private func scanAlertLevel(_ findings: [[String: Any]]) -> ScanAlertLevel {
    findings.allSatisfy {
        matchesMediumSeverity($0["severity"] as? String)
    } ? .medium : .high
}

private func matchesMediumSeverity(_ severity: String?) -> Bool {
    switch severity?.lowercased() {
    case "medium", "mid": true
    default: false
    }
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
    let op: String
    let keys: [String]
    let target: String
    let args: [String]
    let cwd: String
    let replaceExistingEnv: Bool
    let allowMissingKeys: Bool
    let envConflicts: [String]
    let shebangScript: String?
    let tool: String?
    let title: String?
    let detail: String?
}

private struct TransientApprovalKey: Hashable {
    let pid: Int32
    let startUsec: UInt64
    let callerPath: String
    let signingIdentifier: String
    let signingTeamIdentifier: String
    let op: String
    let keys: [String]
    let target: String
    let args: [String]
    let cwd: String
    let replaceExistingEnv: Bool
    let allowMissingKeys: Bool
    let envConflicts: [String]
    let shebangScript: String?
    let tool: String?
}

private struct TransientApprovalCache {
    private var expirations: [TransientApprovalKey: Date] = [:]

    mutating func contains(_ key: TransientApprovalKey, now: Date = Date()) -> Bool {
        prune(now: now)
        return expirations[key].map { $0 > now } ?? false
    }

    mutating func remember(_ key: TransientApprovalKey, now: Date = Date()) {
        prune(now: now)
        expirations[key] = now.addingTimeInterval(transientApprovalTTL)
    }

    private mutating func prune(now: Date) {
        expirations = expirations.filter { $0.value > now }
    }
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
    private let onAutoApproval: @MainActor (AutoApprovalRecord) -> Void
    private var listener: xpc_connection_t?
    // ponytail: in-memory per-process cache; use persistent approvals for cross-process trust.
    private var transientApprovals = TransientApprovalCache()

    init(
        serviceName: String,
        onAutoApproval: @escaping @MainActor (AutoApprovalRecord) -> Void = { _ in }
    ) throws {
        guard let teamIdentifier = selfTeamIdentifier() else {
            throw AppError("missing menu bar signing team identifier")
        }
        self.serviceName = serviceName
        self.teamIdentifier = teamIdentifier
        self.onAutoApproval = onAutoApproval
    }

    func start() throws {
        listener = serviceName.withCString {
            xpc_connection_create_mach_service(
                $0,
                nil,
                UInt64(XPC_CONNECTION_MACH_SERVICE_LISTENER)
            )
        }
        guard let listener else { throw AppError("approval XPC listener failed") }

        let requirement = """
        anchor apple generic and certificate leaf[subject.OU] = \(teamIdentifier) and \
        (identifier "com.automicvault.av" or identifier "gh" or identifier "com.github.cli" or \
        identifier "supabase" or identifier "supabase-go" or identifier "com.supabase.cli")
        """
        let status = requirement.withCString {
            xpc_connection_set_peer_code_signing_requirement(listener, $0)
        }
        guard status == 0 else {
            throw AppError("approval XPC signing requirement failed")
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
        let signing = signingInfo(path: callerPath)

        guard let opPointer = xpc_dictionary_get_string(message, "op") else {
            reply(peer, to: message, ok: false, error: "invalid XPC request")
            return
        }
        let op = String(cString: opPointer)

        guard isAllowedCaller(path: callerPath, signing: signing) else {
            reply(peer, to: message, ok: false, error: "approval caller is not trusted")
            return
        }

        switch op {
        case "inject", "keys":
            handleInject(message, on: peer, pid: pid, identity: identity, callerPath: callerPath, signing: signing)
        case "save":
            handleSave(message, on: peer)
        case "delete":
            handleDelete(message, on: peer)
        default:
            reply(peer, to: message, ok: false, error: "invalid XPC operation")
        }
    }

    private func handleInject(
        _ message: xpc_object_t,
        on peer: xpc_connection_t,
        pid: pid_t,
        identity: AVProcessIdentity,
        callerPath: String,
        signing: SigningInfo
    ) {
        guard let request = approvalRequest(from: message) else {
            reply(peer, to: message, ok: false, error: "invalid approval request")
            return
        }
        if canAutoApproveReadOnlyGhRequest(request: request, callerPath: callerPath, signing: signing) {
            do {
                let secrets = try approvedSecrets(for: request)
                reply(peer, to: message, ok: true, error: nil, secrets: secrets)
            } catch {
                reply(peer, to: message, ok: false, error: error.localizedDescription)
            }
            return
        }
        let transientApproval = TransientApprovalKey(
            pid: pid,
            startUsec: identity.start_usec,
            callerPath: callerPath,
            signingIdentifier: signing.identifier,
            signingTeamIdentifier: signing.teamIdentifier,
            op: request.op,
            keys: request.keys.sorted(),
            target: request.target,
            args: request.args,
            cwd: request.cwd,
            replaceExistingEnv: request.replaceExistingEnv,
            allowMissingKeys: request.allowMissingKeys,
            envConflicts: request.envConflicts.sorted(),
            shebangScript: request.shebangScript,
            tool: request.tool
        )
        let scriptApproval = scriptApproval(for: request)
        var launchers = launcherIdentities(startingAt: identity.ppid)
        if launchers.isEmpty, let launcher = launcherIdentity(pid: pid, identity: identity) {
            launchers.append(launcher)
        }
        let launcher = trustedLauncher(script: scriptApproval, request: request, launchers: launchers) ?? launchers.first
        let trustedApproval = trustedApprovalRecord(
            script: scriptApproval,
            request: request,
            launcher: launcher
        )
        if let launcher, let trustedApproval, alwaysAllows(trustedApproval) {
            DispatchQueue.main.async {
                do {
                    let secrets = try self.approvedSecrets(for: request)
                    self.onAutoApproval(autoApprovalRecord(request: request, script: scriptApproval, launcher: launcher))
                    showAutoApprovedToast(
                        keys: request.keys,
                        script: scriptApproval?.path ?? request.tool ?? request.target,
                        launcher: launcher.identifier
                    )
                    self.reply(peer, to: message, ok: true, error: nil, secrets: secrets)
                } catch {
                    self.reply(peer, to: message, ok: false, error: error.localizedDescription)
                }
            }
            return
        }

        DispatchQueue.main.async {
            if self.transientApprovals.contains(transientApproval) {
                do {
                    let secrets = try self.approvedSecrets(for: request)
                    self.reply(peer, to: message, ok: true, error: nil, secrets: secrets)
                } catch {
                    self.reply(peer, to: message, ok: false, error: error.localizedDescription)
                }
                return
            }

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
            guard decision != .denied else {
                self.reply(peer, to: message, ok: false, error: "\(request.op) denied")
                return
            }
            self.transientApprovals.remember(transientApproval)
            do {
                let secrets = try self.approvedSecrets(for: request)
                self.reply(peer, to: message, ok: true, error: nil, secrets: secrets)
            } catch {
                self.reply(peer, to: message, ok: false, error: error.localizedDescription)
            }
        }
    }

    private func handleSave(_ message: xpc_object_t, on peer: xpc_connection_t) {
        guard let keyPointer = xpc_dictionary_get_string(message, "key"),
              let valuePointer = xpc_dictionary_get_string(message, "value")
        else {
            reply(peer, to: message, ok: false, error: "invalid save request")
            return
        }
        let key = String(cString: keyPointer)
        guard validSecretKeyName(key) else {
            reply(peer, to: message, ok: false, error: "invalid isotope key name: \(key)")
            return
        }
        let value = String(cString: valuePointer)
        let status = saveStoredSecret(account: key, value: value)
        if status == errSecSuccess {
            reply(peer, to: message, ok: true, error: nil)
        } else {
            reply(peer, to: message, ok: false, error: "failed to store isotope key \(key): \(status)")
        }
    }

    private func handleDelete(_ message: xpc_object_t, on peer: xpc_connection_t) {
        guard let keyPointer = xpc_dictionary_get_string(message, "key") else {
            reply(peer, to: message, ok: false, error: "invalid delete request")
            return
        }
        let key = String(cString: keyPointer)
        guard validSecretKeyName(key) else {
            reply(peer, to: message, ok: false, error: "invalid isotope key name: \(key)")
            return
        }
        let status = deleteStoredSecret(account: key)
        if status == errSecSuccess || status == errSecItemNotFound {
            reply(peer, to: message, ok: true, error: nil)
        } else {
            reply(peer, to: message, ok: false, error: "failed to delete isotope key \(key): \(status)")
        }
    }

    private func approvedSecrets(for request: ApprovalRequest) throws -> [String: String] {
        let conflicts = Set(request.envConflicts)
        var secrets: [String: String] = [:]
        for key in request.keys where request.replaceExistingEnv || !conflicts.contains(key) {
            guard let value = loadStoredSecret(account: key) else {
                if request.allowMissingKeys { continue }
                throw AppError("failed to load isotope key \(key): \(errSecItemNotFound)")
            }
            secrets[key] = value
        }
        return secrets
    }

    private func approvalRequest(from message: xpc_object_t) -> ApprovalRequest? {
        guard let opPointer = xpc_dictionary_get_string(message, "op"),
              let targetPointer = xpc_dictionary_get_string(message, "target"),
              let cwdPointer = xpc_dictionary_get_string(message, "cwd"),
              let keys = stringArray(message, "keys"),
              let args = stringArray(message, "args"),
              let envConflicts = stringArray(message, "env_conflicts")
        else {
            return nil
        }
        let op = String(cString: opPointer)
        guard op == "inject" || op == "keys" else { return nil }

        return ApprovalRequest(
            op: op,
            keys: keys,
            target: String(cString: targetPointer),
            args: args,
            cwd: String(cString: cwdPointer),
            replaceExistingEnv: xpc_dictionary_get_bool(message, "replace_existing_env"),
            allowMissingKeys: xpc_dictionary_get_bool(message, "allow_missing_keys"),
            envConflicts: envConflicts,
            shebangScript: xpc_dictionary_get_string(message, "shebang_script").map(String.init(cString:)),
            tool: xpc_dictionary_get_string(message, "tool").map(String.init(cString:)),
            title: xpc_dictionary_get_string(message, "title").map(String.init(cString:)),
            detail: xpc_dictionary_get_string(message, "detail").map(String.init(cString:))
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

    private func reply(
        _ peer: xpc_connection_t,
        to message: xpc_object_t,
        ok: Bool,
        error: String?,
        secrets: [String: String]? = nil
    ) {
        let response = xpc_dictionary_create_reply(message) ?? xpc_dictionary_create_empty()
        xpc_dictionary_set_bool(response, "ok", ok)
        if let error {
            error.withCString {
                xpc_dictionary_set_string(response, "error", $0)
            }
        }
        if let secrets {
            let values = xpc_dictionary_create_empty()
            for (key, value) in secrets {
                key.withCString { keyPointer in
                    value.withCString { valuePointer in
                        xpc_dictionary_set_string(values, keyPointer, valuePointer)
                    }
                }
            }
            xpc_dictionary_set_value(response, "secrets", values)
        }
        xpc_connection_send_message(peer, response)
    }
}

private func isAllowedCaller(path: String, signing: SigningInfo) -> Bool {
    let name = URL(fileURLWithPath: path).lastPathComponent
    if name == "av", signing.identifier == "com.automicvault.av" {
        return true
    }
    if isTrustedGhCaller(path: path, signing: signing) {
        return true
    }
    return (name == "supabase" || name == "supabase-go")
        && (signing.identifier == "supabase"
            || signing.identifier == "supabase-go"
            || signing.identifier == "com.supabase.cli")
}

private func isTrustedGhCaller(path: String, signing: SigningInfo) -> Bool {
    URL(fileURLWithPath: path).lastPathComponent == "gh"
        && (signing.identifier == "gh" || signing.identifier == "com.github.cli")
}

private func canAutoApproveReadOnlyGhRequest(
    request: ApprovalRequest,
    callerPath: String,
    signing: SigningInfo,
    defaults: UserDefaults = .standard
) -> Bool {
    request.op == "keys"
        && !request.keys.isEmpty
        && request.keys.allSatisfy { $0.hasPrefix("GH_TOKEN_") }
        && defaults.bool(forKey: ghReadOnlyAutoApprovalDefaultsKey)
        && isTrustedGhCaller(path: callerPath, signing: signing)
        && ghRequestIsReadOnly(request.args)
}

private func ghRequestIsReadOnly(_ args: [String]) -> Bool {
    let words = ghCommandWords(args).map { $0.lowercased() }
    guard let command = words.first else { return false }
    if words.contains("--show-token") { return false }
    if ["api", "alias", "extension", "config", "skill"].contains(command) { return false }
    if ["status", "browse", "search"].contains(command) { return true }
    guard words.count >= 2 else { return false }
    let subcommand = words[1]
    switch command {
    case "auth":
        return subcommand == "status"
    case "repo":
        return ["view", "list"].contains(subcommand)
    case "issue":
        return ["view", "list", "status"].contains(subcommand)
    case "pr":
        return ["view", "list", "status", "checks", "diff"].contains(subcommand)
    case "run":
        return ["view", "list", "download"].contains(subcommand)
    case "workflow":
        return ["view", "list"].contains(subcommand)
    case "release":
        return ["view", "list", "download"].contains(subcommand)
    case "gist":
        return ["view", "list"].contains(subcommand)
    case "cache", "secret", "variable", "ruleset", "org", "label", "gpg-key", "ssh-key":
        return subcommand == "list" || (command == "ruleset" && subcommand == "view")
    case "attestation":
        return ["verify", "download", "trusted-root"].contains(subcommand)
    case "agent-task":
        return ["view", "list"].contains(subcommand)
    default:
        return false
    }
}

private func ghCommandWords(_ args: [String]) -> [String] {
    var index = 0
    while index < args.count {
        let arg = args[index]
        if arg == "--" {
            return []
        }
        if ["-R", "--repo", "--hostname"].contains(arg) {
            index += 2
            continue
        }
        if arg.hasPrefix("--repo=") || arg.hasPrefix("--hostname=") {
            index += 1
            continue
        }
        if arg.hasPrefix("-") {
            return []
        }
        return Array(args[index...])
    }
    return []
}

private func validSecretKeyName(_ key: String) -> Bool {
    guard let first = key.unicodeScalars.first,
          first == "_" || first.isASCIIAlpha
    else {
        return false
    }
    return key.unicodeScalars.dropFirst().allSatisfy {
        $0 == "_" || $0.isASCIIAlpha || $0.isASCIIDigit
    }
}

private extension UnicodeScalar {
    var isASCIIAlpha: Bool {
        (65...90).contains(value) || (97...122).contains(value)
    }

    var isASCIIDigit: Bool {
        (48...57).contains(value)
    }
}

private struct AppError: LocalizedError {
    let errorDescription: String?

    init(_ description: String) {
        errorDescription = description
    }
}

private func handOffToLaunchAgentIfNeeded() throws -> Bool {
    guard !isLaunchAgentInstance(),
          let launchAgent = bundledLaunchAgentURL()
    else {
        return false
    }

    let installed = FileManager.default.homeDirectoryForCurrentUser
        .appendingPathComponent("Library/LaunchAgents/\(approvalLaunchAgentName).plist")
    try FileManager.default.createDirectory(
        at: installed.deletingLastPathComponent(),
        withIntermediateDirectories: true
    )
    try? FileManager.default.removeItem(at: installed)
    try FileManager.default.copyItem(at: launchAgent, to: installed)

    let domain = "gui/\(getuid())"
    try? runLaunchctl(["bootout", "\(domain)/\(approvalLaunchAgentName)"])
    do {
        try runLaunchctl(["bootstrap", domain, installed.path])
    } catch {
        usleep(200_000)
        try runLaunchctl(["bootstrap", domain, installed.path])
    }
    try runLaunchctl(["enable", "\(domain)/\(approvalLaunchAgentName)"])
    try runLaunchctl(["kickstart", "-k", "\(domain)/\(approvalLaunchAgentName)"])
    return true
}

private func bundledLaunchAgentURL() -> URL? {
    let url = Bundle.main.bundleURL
        .appendingPathComponent("Contents/Library/LaunchAgents/\(approvalLaunchAgentName).plist")
    return FileManager.default.fileExists(atPath: url.path) ? url : nil
}

private func isLaunchAgentInstance(
    environment: [String: String] = ProcessInfo.processInfo.environment
) -> Bool {
    environment["XPC_SERVICE_NAME"] == approvalLaunchAgentName
}

private func runLaunchctl(_ arguments: [String]) throws {
    let process = Process()
    process.executableURL = URL(fileURLWithPath: "/bin/launchctl")
    process.arguments = arguments
    let pipe = Pipe()
    process.standardError = pipe
    process.standardOutput = pipe
    try process.run()
    process.waitUntilExit()
    guard process.terminationStatus == 0 else {
        let data = pipe.fileHandleForReading.readDataToEndOfFile()
        let output = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines)
        throw AppError("launchctl \(arguments.joined(separator: " ")) failed: \(output ?? "exit \(process.terminationStatus)")")
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
    launcherIdentities(startingAt: startPID).first
}

private func launcherIdentities(startingAt startPID: pid_t) -> [LauncherIdentity] {
    var pid = startPID
    var seen = Set<pid_t>()
    var launchers: [LauncherIdentity] = []
    for _ in 0..<32 {
        guard pid > 1, seen.insert(pid).inserted else { return launchers }

        var identity = AVProcessIdentity()
        guard av_process_identity(pid, &identity) else { return launchers }
        if let launcher = launcherIdentity(pid: pid, identity: identity) {
            launchers.append(launcher)
        }
        pid = identity.ppid
    }
    return launchers
}

private func launcherIdentity(pid: pid_t, identity: AVProcessIdentity) -> LauncherIdentity? {
    let path = pathString(identity)
    guard let signing = liveSigningInfo(pid: pid) ?? executableSigningInfo(path: path) else { return nil }
    return launcherIdentity(pid: pid, path: path, signing: signing)
}

private func launcherIdentity(
    pid: pid_t,
    path: String,
    signing: LiveSigningInfo,
    appSigning: (URL) -> StaticSigningInfo? = staticSigningInfo
) -> LauncherIdentity? {
    guard !signing.isAdHoc,
          let appURL = appBundleURL(containing: path)
              ?? appBundleURL(containing: signing.mainExecutable)
              ?? associatedAppBundleURL(path: path, signing: signing),
          let app = appSigning(appURL)
    else {
        return nil
    }
    return LauncherIdentity(
        pid: pid,
        path: path,
        identifier: app.identifier,
        teamIdentifier: app.teamIdentifier,
        designatedRequirement: app.designatedRequirement
    )
}

private struct LiveSigningInfo {
    let identifier: String
    let teamIdentifier: String
    let designatedRequirement: String
    let mainExecutable: String
    let isAdHoc: Bool
}

private struct StaticSigningInfo {
    let identifier: String
    let teamIdentifier: String
    let designatedRequirement: String
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
    let signatureFlags = (dictionary[kSecCodeInfoFlags] as? NSNumber)?.uint32Value ?? 0
    return LiveSigningInfo(
        identifier: dictionary[kSecCodeInfoIdentifier] as? String ?? "unknown",
        teamIdentifier: dictionary[kSecCodeInfoTeamIdentifier] as? String ?? "unknown",
        designatedRequirement: requirementText,
        mainExecutable: executable,
        isAdHoc: signatureFlags & secCodeSignatureAdHoc != 0
    )
}

private func executableSigningInfo(path: String) -> LiveSigningInfo? {
    var staticCode: SecStaticCode?
    guard SecStaticCodeCreateWithPath(URL(fileURLWithPath: path) as CFURL, [], &staticCode) == errSecSuccess,
          let staticCode,
          SecStaticCodeCheckValidity(staticCode, [], nil) == errSecSuccess
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

    let executable = (dictionary[kSecCodeInfoMainExecutable] as? URL)?.path ?? path
    let signatureFlags = (dictionary[kSecCodeInfoFlags] as? NSNumber)?.uint32Value ?? 0
    return LiveSigningInfo(
        identifier: dictionary[kSecCodeInfoIdentifier] as? String ?? "unknown",
        teamIdentifier: dictionary[kSecCodeInfoTeamIdentifier] as? String ?? "unknown",
        designatedRequirement: requirementText,
        mainExecutable: executable,
        isAdHoc: signatureFlags & secCodeSignatureAdHoc != 0
    )
}

private func staticSigningInfo(url: URL) -> StaticSigningInfo? {
    var staticCode: SecStaticCode?
    guard SecStaticCodeCreateWithPath(url as CFURL, [], &staticCode) == errSecSuccess,
          let staticCode
    else {
        return nil
    }

    var info: CFDictionary?
    let flags = SecCSFlags(rawValue: kSecCSSigningInformation | kSecCSRequirementInformation)
    guard SecCodeCopySigningInformation(staticCode, flags, &info) == errSecSuccess,
          let dictionary = info as? [CFString: Any],
          let requirementValue = dictionary[kSecCodeInfoDesignatedRequirement],
          ((dictionary[kSecCodeInfoFlags] as? NSNumber)?.uint32Value ?? 0) & secCodeSignatureAdHoc == 0
    else {
        return nil
    }
    let requirement = requirementValue as! SecRequirement
    guard let requirementText = requirementString(requirement) else { return nil }

    return StaticSigningInfo(
        identifier: dictionary[kSecCodeInfoIdentifier] as? String ?? "unknown",
        teamIdentifier: dictionary[kSecCodeInfoTeamIdentifier] as? String ?? "unknown",
        designatedRequirement: requirementText
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

private func appBundleURL(containing path: String) -> URL? {
    var url = URL(fileURLWithPath: path)
    while url.path != "/" {
        if url.pathExtension.caseInsensitiveCompare("app") == .orderedSame {
            return url
        }
        url.deleteLastPathComponent()
    }
    return nil
}

private func associatedAppBundleURL(path: String, signing: LiveSigningInfo) -> URL? {
    guard signing.identifier == "com.automicvault.vaultty.session-bridge",
          path.hasSuffix("/Library/Application Support/Vaultty/vaultty-session-bridge")
    else {
        return nil
    }
    return NSWorkspace.shared.urlForApplication(withBundleIdentifier: "com.automicvault.vaultty")
        ?? URL(fileURLWithPath: "/Applications/Vaultty.app")
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
    guard let launcher else { return nil }
    return TrustedScriptApproval(
        scriptPath: script?.path,
        scriptChecksum: script?.checksum,
        keys: request.keys.sorted(),
        target: request.target,
        replaceExistingEnv: request.replaceExistingEnv,
        allowMissingKeys: request.allowMissingKeys,
        launcherRequirement: launcher.designatedRequirement
    )
}

private func trustedLauncher(
    script: ScriptApproval?,
    request: ApprovalRequest,
    launchers: [LauncherIdentity],
    service: String = trustedScriptApprovalsKeychainService,
    account: String = trustedScriptApprovalsKeychainAccount
) -> LauncherIdentity? {
    launchers.first {
        trustedApprovalRecord(script: script, request: request, launcher: $0).map {
            alwaysAllows($0, service: service, account: account)
        } == true
    }
}

private func trustedLauncher(
    script: ScriptApproval?,
    request: ApprovalRequest,
    launchers: [LauncherIdentity],
    approvals: [TrustedScriptApproval]
) -> LauncherIdentity? {
    launchers.first {
        trustedApprovalRecord(script: script, request: request, launcher: $0).map {
            alwaysAllows($0, approvals: approvals)
        } == true
    }
}

private func alwaysAllows(
    _ approval: TrustedScriptApproval,
    service: String = trustedScriptApprovalsKeychainService,
    account: String = trustedScriptApprovalsKeychainAccount
) -> Bool {
    loadTrustedScriptApprovals(service: service, account: account).contains(approval)
}

private func alwaysAllows(
    _ approval: TrustedScriptApproval,
    approvals: [TrustedScriptApproval]
) -> Bool {
    approvals.contains(approval)
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
    alert.messageText = request.title ?? (request.op == "keys" ? "Approve key request?" : "Approve secret injection?")
    var lines = [
        "Caller: \(callerPath) (pid \(pid))",
        "Signed: \(signing.identifier) / \(signing.teamIdentifier)",
        "Operation: \(request.op)",
        "Target: \(request.target)",
        "Arguments: \(request.args.isEmpty ? "(none)" : request.args.joined(separator: " "))",
        "Working directory: \(request.cwd)",
        "Keys: \(request.keys.joined(separator: ", "))",
        "Existing environment: \(request.envConflicts.isEmpty ? "(none)" : request.envConflicts.joined(separator: ", "))",
        "Replace existing environment: \(request.replaceExistingEnv ? "yes" : "no")",
        "Allow missing keys: \(request.allowMissingKeys ? "yes" : "no")",
    ]
    if let tool = request.tool {
        lines.append("Tool: \(tool)")
    }
    if let detail = request.detail {
        lines.append("Detail: \(detail)")
    }
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
    if launcher != nil {
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
    let request = ApprovalRequest(
        op: "inject",
        keys: ["B", "A"],
        target: "/bin/echo",
        args: ["ignored"],
        cwd: "/tmp",
        replaceExistingEnv: true,
        allowMissingKeys: false,
        envConflicts: ["ignored"],
        shebangScript: "/tmp/deploy",
        tool: nil,
        title: nil,
        detail: nil
    )
    let script = ScriptApproval(path: "/tmp/deploy", checksum: "abc")
    let launcher = LauncherIdentity(
        pid: 42,
        path: "/Applications/Codex.app/Contents/MacOS/Codex",
        identifier: "com.openai.codex",
        teamIdentifier: "TEAM",
        designatedRequirement: #"identifier "com.openai.codex" and anchor apple generic"#
    )
    let vaulttySigning = LiveSigningInfo(
        identifier: "app.vaultty.Vaultty",
        teamIdentifier: "TEAM",
        designatedRequirement: #"identifier "app.vaultty.Vaultty" and anchor apple generic"#,
        mainExecutable: "/Applications/Vaultty.app/Contents/Helpers/vaultty-sessiond",
        isAdHoc: false
    )
    let vaulttyBridgeSigning = LiveSigningInfo(
        identifier: "com.automicvault.vaultty.session-bridge",
        teamIdentifier: "TEAM",
        designatedRequirement: #"identifier "com.automicvault.vaultty.session-bridge" and anchor apple generic"#,
        mainExecutable: "/Users/mxcl/Library/Application Support/Vaultty/vaultty-session-bridge",
        isAdHoc: false
    )
    let vaulttyAppSigning = StaticSigningInfo(
        identifier: "com.automicvault.vaultty",
        teamIdentifier: "TEAM",
        designatedRequirement: #"identifier "com.automicvault.vaultty" and anchor apple generic"#
    )
    let pythonSigning = LiveSigningInfo(
        identifier: "org.python.python",
        teamIdentifier: "unknown",
        designatedRequirement: #"identifier "org.python.python" and anchor apple generic"#,
        mainExecutable: "/opt/homebrew/Cellar/python@3.14/3.14.6/Frameworks/Python.framework/Versions/3.14/Resources/Python.app/Contents/MacOS/Python",
        isAdHoc: true
    )
    let wrapperLauncher = LauncherIdentity(
        pid: 45,
        path: "/Applications/Wrapper.app/Contents/MacOS/Wrapper",
        identifier: "com.example.wrapper",
        teamIdentifier: "TEAM",
        designatedRequirement: #"identifier "com.example.wrapper" and anchor apple generic"#
    )
    let unbundledSigning = LiveSigningInfo(
        identifier: "com.automicvault.av",
        teamIdentifier: "TEAM",
        designatedRequirement: #"identifier "com.automicvault.av" and anchor apple generic"#,
        mainExecutable: "/usr/local/bin/av",
        isAdHoc: false
    )
    let parentlessVaulttyLauncher = launcherIdentity(
        pid: 43,
        path: "/Applications/Vaultty.app/Contents/Helpers/vaultty-sessiond",
        signing: vaulttySigning,
        appSigning: { _ in vaulttyAppSigning }
    )
    let vaulttyBridgeLauncher = launcherIdentity(
        pid: 44,
        path: "/Users/mxcl/Library/Application Support/Vaultty/vaultty-session-bridge",
        signing: vaulttyBridgeSigning,
        appSigning: { _ in vaulttyAppSigning }
    )
    guard let approval = trustedApprovalRecord(
        script: script,
        request: request,
        launcher: launcher
    ), let appApproval = trustedApprovalRecord(
        script: nil,
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
          appApproval.scriptPath == nil,
          appApproval.scriptChecksum == nil,
          appApproval.keys == ["A", "B"],
          trustedApprovalRecord(script: script, request: request, launcher: nil) == nil,
          parentlessVaulttyLauncher?.designatedRequirement == vaulttyAppSigning.designatedRequirement,
          vaulttyBridgeLauncher?.designatedRequirement == vaulttyAppSigning.designatedRequirement,
          launcherIdentity(pid: 45, path: pythonSigning.mainExecutable, signing: pythonSigning) == nil,
          launcherIdentity(pid: 46, path: "/usr/local/bin/av", signing: unbundledSigning) == nil,
          !alwaysAllows(approval, approvals: [])
    else {
        return 1
    }

    let scriptApprovals = [approval]
    guard alwaysAllows(approval, approvals: scriptApprovals),
          trustedLauncher(script: script, request: request, launchers: [wrapperLauncher, launcher], approvals: scriptApprovals)?.designatedRequirement == launcher.designatedRequirement,
          !alwaysAllows(altered(checksum: "def"), approvals: scriptApprovals),
          !alwaysAllows(altered(keys: ["A"]), approvals: scriptApprovals),
          !alwaysAllows(altered(target: "/usr/bin/env"), approvals: scriptApprovals),
          !alwaysAllows(altered(replaceExistingEnv: false), approvals: scriptApprovals),
          !alwaysAllows(altered(allowMissingKeys: true), approvals: scriptApprovals),
          !alwaysAllows(altered(launcherRequirement: #"identifier "com.apple.Terminal""#), approvals: scriptApprovals)
    else {
        return 1
    }
    let appApprovals = [approval, appApproval]
    guard alwaysAllows(appApproval, approvals: appApprovals),
          trustedLauncher(script: nil, request: request, launchers: [wrapperLauncher, launcher], approvals: appApprovals)?.designatedRequirement == launcher.designatedRequirement,
          !alwaysAllows(TrustedScriptApproval(
              scriptPath: nil,
              scriptChecksum: nil,
              keys: ["A"],
              target: "/bin/echo",
              replaceExistingEnv: true,
              allowMissingKeys: false,
              launcherRequirement: launcher.designatedRequirement
          ), approvals: appApprovals)
    else {
        return 1
    }
    let defaultsName = "com.automicvault.av2.approval-self-check.defaults.\(UUID().uuidString)"
    guard let defaults = UserDefaults(suiteName: defaultsName) else { return 1 }
    defaults.removePersistentDomain(forName: defaultsName)
    defer { defaults.removePersistentDomain(forName: defaultsName) }
    let ghSigning = SigningInfo(identifier: "gh", teamIdentifier: "TEAM")
    func ghRequest(
        op: String = "keys",
        keys: [String] = ["GH_TOKEN_GITHUB_COM"],
        args: [String] = ["repo", "view"]
    ) -> ApprovalRequest {
        ApprovalRequest(
            op: op,
            keys: keys,
            target: "/opt/homebrew/Cellar/gh-cli/2.94.0/bin/gh",
            args: args,
            cwd: "/tmp",
            replaceExistingEnv: true,
            allowMissingKeys: false,
            envConflicts: [],
            shebangScript: nil,
            tool: "gh",
            title: nil,
            detail: nil
        )
    }
    let readOnlyGh = ghRequest()
    guard !canAutoApproveReadOnlyGhRequest(
        request: readOnlyGh,
        callerPath: "/opt/homebrew/bin/gh",
        signing: ghSigning,
        defaults: defaults
    ) else {
        return 1
    }
    defaults.set(true, forKey: ghReadOnlyAutoApprovalDefaultsKey)
    guard canAutoApproveReadOnlyGhRequest(
        request: readOnlyGh,
        callerPath: "/opt/homebrew/bin/gh",
        signing: ghSigning,
        defaults: defaults
    ),
          !canAutoApproveReadOnlyGhRequest(
              request: ghRequest(args: ["repo", "delete", "owner/name"]),
              callerPath: "/opt/homebrew/bin/gh",
              signing: ghSigning,
              defaults: defaults
          ),
          !canAutoApproveReadOnlyGhRequest(
              request: ghRequest(keys: ["OTHER_TOKEN"]),
              callerPath: "/opt/homebrew/bin/gh",
              signing: ghSigning,
              defaults: defaults
          ),
          !canAutoApproveReadOnlyGhRequest(
              request: ghRequest(op: "inject"),
              callerPath: "/opt/homebrew/bin/gh",
              signing: ghSigning,
              defaults: defaults
          ),
          !canAutoApproveReadOnlyGhRequest(
              request: readOnlyGh,
              callerPath: "/usr/local/bin/av",
              signing: SigningInfo(identifier: "com.automicvault.av", teamIdentifier: "TEAM"),
              defaults: defaults
          )
    else {
        return 1
    }
    return 0
}

private func runGhReadOnlySelfCheck() -> Int32 {
    let allowed = [
        ["auth", "status"],
        ["status"],
        ["browse"],
        ["search", "prs", "foo"],
        ["repo", "view"],
        ["repo", "list"],
        ["issue", "view", "1"],
        ["issue", "list"],
        ["issue", "status"],
        ["pr", "view"],
        ["pr", "list"],
        ["pr", "status"],
        ["pr", "checks"],
        ["pr", "diff"],
        ["run", "view"],
        ["run", "list"],
        ["run", "download"],
        ["workflow", "view"],
        ["workflow", "list"],
        ["release", "view"],
        ["release", "list"],
        ["release", "download"],
        ["gist", "view"],
        ["gist", "list"],
        ["cache", "list"],
        ["secret", "list"],
        ["variable", "list"],
        ["ruleset", "view"],
        ["ruleset", "list"],
        ["attestation", "verify"],
        ["attestation", "download"],
        ["attestation", "trusted-root"],
        ["agent-task", "view"],
        ["agent-task", "list"],
        ["org", "list"],
        ["label", "list"],
        ["gpg-key", "list"],
        ["ssh-key", "list"],
        ["-R", "owner/repo", "pr", "view"],
        ["--hostname=github.example.com", "repo", "view"],
    ]
    guard allowed.allSatisfy(ghRequestIsReadOnly) else { return 1 }

    let denied = [
        ["api", "repos/owner/repo"],
        ["auth", "token"],
        ["auth", "status", "--show-token"],
        ["alias", "set", "x", "repo view"],
        ["extension", "install", "owner/gh-ext"],
        ["config", "set", "editor", "vim"],
        ["skill", "install", "foo"],
        ["repo", "delete", "owner/name"],
        ["issue", "create"],
        ["pr", "merge"],
        ["run", "rerun"],
        ["workflow", "enable"],
        ["release", "create"],
        ["unknown", "view"],
        ["--unknown", "repo", "view"],
    ]
    guard denied.allSatisfy({ !ghRequestIsReadOnly($0) }) else { return 1 }
    return 0
}

private func runTransientApprovalSelfCheck() -> Int32 {
    func key(startUsec: UInt64 = 456, args: [String] = ["repo", "view"]) -> TransientApprovalKey {
        TransientApprovalKey(
            pid: 123,
            startUsec: startUsec,
            callerPath: "/opt/homebrew/bin/gh",
            signingIdentifier: "gh",
            signingTeamIdentifier: "TEAM",
            op: "keys",
            keys: ["GH_TOKEN_GITHUB_COM"],
            target: "/opt/homebrew/Cellar/gh-cli/2.94.0/bin/gh",
            args: args,
            cwd: "/tmp",
            replaceExistingEnv: true,
            allowMissingKeys: false,
            envConflicts: [],
            shebangScript: nil,
            tool: "gh"
        )
    }
    let approval = key()
    var cache = TransientApprovalCache()
    cache.remember(approval, now: Date(timeIntervalSince1970: 100))
    guard cache.contains(approval, now: Date(timeIntervalSince1970: 200)),
          !cache.contains(key(args: ["auth", "token"]), now: Date(timeIntervalSince1970: 200)),
          !cache.contains(key(startUsec: 789), now: Date(timeIntervalSince1970: 200)),
          !cache.contains(approval, now: Date(timeIntervalSince1970: 500))
    else {
        return 1
    }
    return 0
}

private func runLaunchAgentHandoffSelfCheck() -> Int32 {
    guard !isLaunchAgentInstance(environment: [:]),
          isLaunchAgentInstance(environment: ["XPC_SERVICE_NAME": approvalLaunchAgentName])
    else {
        return 1
    }
    return 0
}

private func runMenuStatusSelfCheck() -> Int32 {
    let formatter = DateFormatter()
    formatter.locale = Locale(identifier: "en_US_POSIX")
    formatter.timeZone = TimeZone(secondsFromGMT: 0)
    formatter.dateFormat = "h:mm a"
    let request = ApprovalRequest(
        op: "inject",
        keys: ["AWS_SECRET_ACCESS_KEY"],
        target: "/bin/zsh",
        args: [],
        cwd: "/tmp",
        replaceExistingEnv: true,
        allowMissingKeys: false,
        envConflicts: [],
        shebangScript: "/usr/local/bin/aws",
        tool: nil,
        title: nil,
        detail: nil
    )
    guard shortAppName("com.openai.codex") == "Codex",
          autoApprovalToolName(request) == "aws",
          scanAlertLevel([["severity": "medium"]]) == .medium,
          scanAlertLevel([["severity": "medium"], ["severity": "high"]]) == .high,
          autoApprovalTitle(
              AutoApprovalRecord(date: Date(timeIntervalSince1970: 18_900), launcher: "Codex", tool: "aws"),
              formatter: formatter
          ) == "5:15 AM – Codex used aws"
    else {
        return 1
    }
    return 0
}

if CommandLine.arguments.contains("--self-check-approvals") {
    exit(runApprovalSelfCheck())
}

if CommandLine.arguments.contains("--self-check-gh-read-only") {
    exit(runGhReadOnlySelfCheck())
}

if CommandLine.arguments.contains("--self-check-transient-approvals") {
    exit(runTransientApprovalSelfCheck())
}

if CommandLine.arguments.contains("--self-check-dashboard-search") {
    exit(MainActor.assumeIsolated { runDashboardSearchSelfCheck() })
}

if CommandLine.arguments.contains("--self-check-launch-agent-handoff") {
    exit(runLaunchAgentHandoffSelfCheck())
}

if CommandLine.arguments.contains("--self-check-menu-status") {
    exit(runMenuStatusSelfCheck())
}

let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.setActivationPolicy(.accessory)
app.run()
