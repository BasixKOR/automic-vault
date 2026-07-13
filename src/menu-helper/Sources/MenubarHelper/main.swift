import AppKit
import CProcessInfo
import CoreServices
import CryptoKit
import Darwin
import Foundation
import MenubarHelperCore
import Security
import SwiftUI
@preconcurrency import XPC

private let approvalServiceName = "com.automicvault.av2.approval"
private let approvalLaunchAgentName = "com.automicvault.menubar-helper"
private let secCodeSignatureAdHoc: UInt32 = 0x2
private let transientApprovalTTL: TimeInterval = 5 * 60
private let scanQueue = DispatchQueue(label: "com.automicvault.av2.scan")
private var toastWindows: [NSWindow] = []

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private static let visibleAutoApprovalCount = 5
    private lazy var statusItem = NSStatusBar.system.statusItem(withLength: 15)
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
    #if !DEBUG
    private let postHogTelemetry = PostHogTelemetry.shared
    private var lastTelemetryFindingCount: Int?
    #endif

    func applicationDidFinishLaunching(_ notification: Notification) {
        installStatusMenu()

        if shouldHandOffToLaunchAgent() {
            handOffToLaunchAgent()
            return
        }

        startServices()
    }

    private func installStatusMenu() {
        statusItem.button?.image = brandImage()

        let menu = NSMenu()
        menu.addItem(scanStatusItem)
        menu.addItem(.separator())
        let openItem = NSMenuItem(title: "Open Automic Vault", action: #selector(openMainWindow), keyEquivalent: "")
        openItem.target = self
        menu.addItem(openItem)
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(title: "Quit", action: #selector(quit), keyEquivalent: "q"))
        menu.delegate = self
        statusItem.menu = menu
    }

    private func handOffToLaunchAgent() {
        statusItem.button?.image = brandImage()
        statusItem.button?.alphaValue = 0.5
        scanStatusItem.title = "Starting Automic Vault"
        DispatchQueue.global(qos: .userInitiated).async {
            let result = Result { try handOffToLaunchAgentIfNeeded() }
            DispatchQueue.main.async {
                switch result {
                case .success(true):
                    NSApp.terminate(nil)
                case .success(false):
                    self.startServices()
                case .failure(let error):
                    NSAlert(error: error).runModal()
                    NSApp.terminate(nil)
                }
            }
        }
    }

    private func startServices() {
        statusItem.button?.image = brandImage()
        statusItem.button?.alphaValue = 1
        autoApprovals = loadAccessRequestRecords().compactMap(autoApprovalRecord)
        refreshAutoApprovalMenuItems()
        do {
            let approval = try ApprovalServer(serviceName: approvalServiceName) { [weak self] event in
                self?.recordAutoApproval(event)
            } onAccessRequest: { [weak self] record in
                Task { @MainActor in self?.recordAccessRequest(record) }
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

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        openMainWindow()
        return true
    }

    @MainActor @objc private func quit() {
        NSApp.terminate(nil)
    }

    @MainActor @objc private func openMainWindow() {
        let wasVisible = mainWindow?.isVisible ?? false
        if let mainWindow {
            mainWindow.makeKeyAndOrderFront(nil)
            NSApp.activate(ignoringOtherApps: true)
            #if !DEBUG
            if wasVisible == false {
                postHogTelemetry.captureMainWindowOpened()
            }
            #endif
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
        #if !DEBUG
        postHogTelemetry.captureMainWindowOpened()
        #endif
    }

    @MainActor @objc private func openAutoApproval(_ sender: NSMenuItem) {
        guard let idString = sender.representedObject as? String,
              let id = UUID(uuidString: idString)
        else { return }
        showAutoApproval(id: id)
    }

    private func showAutoApproval(id: UUID) {
        openMainWindow()
        (mainWindow?.contentViewController as? AutomicVaultMainWindowController)?.showAccessRequest(id: id)
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
            #if !DEBUG
            lastTelemetryFindingCount = nil
            #endif
            statusItem.button?.image = brandImage()
            setScanStatus(
                "No Vulnerabilities Detected",
                image: shieldImage(symbolName: "shield.fill", color: .systemGreen)
            )
        case .findings(let count, let detectorCount, let level):
            #if !DEBUG
            if lastTelemetryFindingCount != detectorCount {
                postHogTelemetry.captureDetectorTriggered(count: detectorCount)
                lastTelemetryFindingCount = detectorCount
            }
            #endif
            statusItem.button?.image = switch level {
            case .medium: brandImage()
            case .high: brandImage(color: .systemRed)
            }
            setScanStatus(
                count == 1 ? "1 scan finding" : "\(count) scan findings",
                image: shieldImage(color: level.color)
            )
        case .failed:
            statusItem.button?.image = brandImage(color: .systemRed)
            setScanStatus("Scan failed", image: shieldImage(color: .systemRed))
        }
    }

    private func setScanStatus(_ title: String, image: NSImage?) {
        scanStatusItem.attributedTitle = nil
        scanStatusItem.title = title
        scanStatusItem.image = image
    }

    private func brandImage(color: NSColor? = nil) -> NSImage? {
        let fallback = NSImage(systemSymbolName: "shield.fill", accessibilityDescription: "Automic Vault")
        guard let image = Bundle.main.url(forResource: "NSMenuItem", withExtension: "png")
            .flatMap(NSImage.init(contentsOf:)) ?? fallback else { return nil }
        image.size = NSSize(width: 15, height: 18)
        return tinted(image, color: color)
    }

    private func shieldImage(symbolName: String = "shield.lefthalf.filled", color: NSColor? = nil) -> NSImage? {
        guard let symbol = NSImage(systemSymbolName: symbolName, accessibilityDescription: "SHIELD") else {
            return nil
        }
        let image = symbol.withSymbolConfiguration(.init(pointSize: 14, weight: .semibold)) ?? symbol
        image.size = NSSize(width: 16, height: 16)
        return tinted(image, color: color)
    }

    private func tinted(_ image: NSImage, color: NSColor?) -> NSImage {
        guard let color else {
            image.isTemplate = true
            return image
        }
        let tinted = NSImage(size: image.size, flipped: false) { rect in
            image.draw(in: rect)
            color.setFill()
            rect.fill(using: .sourceIn)
            return true
        }
        tinted.isTemplate = false
        return tinted
    }

    private func recordAutoApproval(_ record: AutoApprovalRecord) {
        recordMenuAccess(record)
        showAutoApprovedToast(record, below: statusItem.button) { [weak self] in
            self?.showAutoApproval(id: record.accessRequestID)
        }
    }

    private func recordMenuAccess(_ record: AutoApprovalRecord) {
        autoApprovals.insert(record, at: 0)
        let capacity = NSScreen.screens.map { screen in
            Self.visibleAutoApprovalCount + autoApprovalSubmenuCapacity(visibleHeight: screen.visibleFrame.height)
        }.max() ?? Self.visibleAutoApprovalCount
        autoApprovals = Array(autoApprovals.prefix(capacity))
        refreshAutoApprovalMenuItems()
    }

    private func recordAccessRequest(_ record: AccessRequestRecord) {
        appendAccessRequestRecord(record)
        if record.decision == "Denied", let menuRecord = autoApprovalRecord(record) {
            recordMenuAccess(menuRecord)
        }
        (mainWindow?.contentViewController as? AutomicVaultMainWindowController)?.reload()
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
        autoApprovalItems = autoApprovals.prefix(Self.visibleAutoApprovalCount).map(autoApprovalMenuItem)
        let submenuRecords = autoApprovals.dropFirst(Self.visibleAutoApprovalCount).prefix(
            autoApprovalSubmenuCapacity(
                visibleHeight: statusItem.button?.window?.screen?.visibleFrame.height
                    ?? NSScreen.main?.visibleFrame.height
                    ?? 0
            )
        )
        if !submenuRecords.isEmpty {
            let moreItem = NSMenuItem(title: "More", action: nil, keyEquivalent: "")
            let submenu = NSMenu()
            submenuRecords.map(autoApprovalMenuItem).forEach(submenu.addItem)
            moreItem.submenu = submenu
            autoApprovalItems.append(moreItem)
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

    private func autoApprovalMenuItem(_ record: AutoApprovalRecord) -> NSMenuItem {
        let item = NSMenuItem(
            title: autoApprovalTitle(record, formatter: autoApprovalTimeFormatter),
            action: #selector(openAutoApproval),
            keyEquivalent: ""
        )
        item.target = self
        item.representedObject = record.accessRequestID.uuidString
        return item
    }
}

extension AppDelegate: NSMenuDelegate {
    func menuWillOpen(_ menu: NSMenu) {
        refreshAutoApprovalMenuItems()
    }
}

private struct AutoApprovalRecord {
    let accessRequestID: UUID
    let date: Date
    let launcher: String
    let launcherIconPath: String
    let tool: String
    let command: String
    let keys: [String]
    let wasDenied: Bool
}

private func autoApprovalTitle(_ record: AutoApprovalRecord, formatter: DateFormatter) -> String {
    let action = record.wasDenied ? "was denied use of" : "used"
    return "\(formatter.string(from: record.date)) – \(record.launcher) \(action) \(record.tool)"
}

private func autoApprovalSubmenuCapacity(visibleHeight: CGFloat) -> Int {
    guard visibleHeight > 0 else { return 0 }
    return max(0, Int((visibleHeight - 16) / 22))
}

private func autoApprovalRecord(
    accessRequestID: UUID,
    request: ApprovalRequest,
    script: ScriptApproval?,
    launcher: LauncherIdentity
) -> AutoApprovalRecord {
    AutoApprovalRecord(
        accessRequestID: accessRequestID,
        date: Date(),
        launcher: shortAppName(launcher.identifier),
        launcherIconPath: appBundleURL(containing: launcher.path)?.path ?? launcher.path,
        tool: autoApprovalToolName(request, scriptPath: script?.path),
        command: autoApprovalCommand(request, scriptPath: script?.path),
        keys: request.keys,
        wasDenied: false
    )
}

private func autoApprovalRecord(_ record: AccessRequestRecord) -> AutoApprovalRecord? {
    let wasDenied = record.decision == "Denied"
    guard wasDenied || (record.decision == "Approved" && record.approvalSourceLabel == "Auto") else { return nil }
    return AutoApprovalRecord(
        accessRequestID: record.id,
        date: record.date,
        launcher: record.launcher ?? "Unknown app",
        launcherIconPath: "",
        tool: record.tool,
        command: record.command,
        keys: record.keys,
        wasDenied: wasDenied
    )
}

private func accessRequestRecord(
    id: UUID = UUID(),
    request: ApprovalRequest,
    callerPath: String,
    decision: String,
    approvalSource: String,
    reason: String,
    launcher: LauncherIdentity?
) -> AccessRequestRecord {
    AccessRequestRecord(
        id: id,
        date: Date(),
        tool: autoApprovalToolName(request),
        command: ([autoApprovalToolName(request)] + request.args).joined(separator: " "),
        decision: decision,
        approvalSource: approvalSource,
        reason: reason,
        launcher: launcher.map { shortAppName($0.identifier) },
        callerPath: callerPath,
        target: request.target,
        cwd: request.cwd,
        keys: request.keys.sorted(),
        detail: request.detail
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

private func autoApprovalCommand(_ request: ApprovalRequest, scriptPath: String? = nil) -> String {
    let scriptPath = scriptPath ?? resolvedShebangScriptPath(request)
    var args = request.args
    if let scriptPath, let first = args.first {
        let firstPath = first.hasPrefix("/")
            ? URL(fileURLWithPath: first)
            : URL(fileURLWithPath: request.cwd).appendingPathComponent(first)
        if firstPath.standardizedFileURL.path == URL(fileURLWithPath: scriptPath).standardizedFileURL.path {
            args.removeFirst()
        }
    }
    return prettyShellCommand(target: autoApprovalToolName(request, scriptPath: scriptPath), args: args)
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
    case findings(Int, Int, ScanAlertLevel)
    case failed
}

private enum ScanAlertLevel {
    case medium
    case high

    var color: NSColor {
        switch self {
        case .medium: .systemOrange
        case .high: .systemRed
        }
    }
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
    let detectorCount = Set(findings.compactMap { $0["source"] as? String }).count
    return findings.isEmpty
        ? .clean(loadDetectorMetadata(avExecutableURL: executableURL).count)
        : .findings(findings.count, detectorCount, scanAlertLevel(findings))
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

private enum ApprovalDecision: Equatable {
    case denied
    case approved
}

private let humanApprovalRequiredEvent = "human-approval-required"

private func approvalEvent(for cachedDecision: ApprovalDecision?) -> String? {
    cachedDecision == nil ? humanApprovalRequiredEvent : nil
}

private struct TransientApprovalCache {
    private enum Key: Hashable {
        case approval(TransientApprovalKey)
        case denial(pid: Int32, startUsec: UInt64)
    }

    private var expirations: [Key: Date] = [:]

    mutating func decision(for key: TransientApprovalKey, now: Date = Date()) -> ApprovalDecision? {
        prune(now: now)
        if expirations[.denial(pid: key.pid, startUsec: key.startUsec)] != nil {
            return .denied
        }
        return expirations[.approval(key)] == nil ? nil : .approved
    }

    mutating func remember(_ decision: ApprovalDecision, for key: TransientApprovalKey, now: Date = Date()) {
        prune(now: now)
        let key = switch decision {
        case .denied: Key.denial(pid: key.pid, startUsec: key.startUsec)
        case .approved: Key.approval(key)
        }
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

private final class ApprovalServer: @unchecked Sendable {
    private let serviceName: String
    private let teamIdentifier: String
    private let hardeners: [HardenerMetadata]?
    private let onAutoApproval: @MainActor (AutoApprovalRecord) -> Void
    private let onAccessRequest: @Sendable (AccessRequestRecord) -> Void
    private var listener: xpc_connection_t?
    // ponytail: in-memory per-process cache; use persistent approvals for cross-process trust.
    private var transientApprovals = TransientApprovalCache()

    init(
        serviceName: String,
        hardeners: [HardenerMetadata]? = nil,
        onAutoApproval: @escaping @MainActor (AutoApprovalRecord) -> Void = { _ in },
        onAccessRequest: @escaping @Sendable (AccessRequestRecord) -> Void = { appendAccessRequestRecord($0) }
    ) throws {
        guard let teamIdentifier = selfTeamIdentifier() else {
            throw AppError("missing menu bar signing team identifier")
        }
        self.serviceName = serviceName
        self.teamIdentifier = teamIdentifier
        self.hardeners = hardeners
        self.onAutoApproval = onAutoApproval
        self.onAccessRequest = onAccessRequest
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
        case "save" where isTrustedAvCaller(path: callerPath, signing: signing):
            handleSave(message, on: peer)
        case "load" where isTrustedAvCaller(path: callerPath, signing: signing):
            handleLoad(message, on: peer)
        case "delete" where isTrustedAvCaller(path: callerPath, signing: signing):
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
        let scriptApproval = scriptApproval(for: request)
        var launchers = launcherIdentities(for: identity)
        let launcherFallbackPath = launcherFallbackPath(for: identity) ?? callerPath
        if launchers.isEmpty, let launcher = launcherIdentity(pid: pid, identity: identity) {
            launchers.append(launcher)
        }
        let configuredGate = matchingSecretGate(
            request: request,
            signing: signing,
            hardeners: hardeners ?? loadHardenerMetadata(avExecutableURL: avExecutableURL())
        )
        let resolvedPolicy = configuredGate.map { resolveSecretGatePolicy(gate: $0, launchers: launchers) }
        let launcher = resolvedPolicy?.launcher ?? launchers.first
        if let configuredGate,
           let resolvedPolicy,
           secretGateProtectionAllows(
               resolvedPolicy.protection,
               classification: classifySecretGateRequest(gateID: configuredGate.id, request: request)
           )
        {
            do {
                let secrets = try approvedSecrets(for: request)
                let reason = "\(resolvedPolicy.protection.title) from \(resolvedPolicy.source)"
                let accessRequestID = UUID()
                if let launcher {
                    Task { @MainActor in
                        self.onAutoApproval(autoApprovalRecord(
                            accessRequestID: accessRequestID,
                            request: request,
                            script: scriptApproval,
                            launcher: launcher
                        ))
                    }
                }
                onAccessRequest(accessRequestRecord(
                    id: accessRequestID,
                    request: request,
                    callerPath: callerPath,
                    decision: "Approved",
                    approvalSource: "Auto",
                    reason: reason,
                    launcher: launcher
                ))
                reply(peer, to: message, ok: true, error: nil, secrets: secrets)
            } catch {
                onAccessRequest(accessRequestRecord(
                    request: request,
                    callerPath: callerPath,
                    decision: "Failed",
                    approvalSource: "Auto",
                    reason: error.localizedDescription,
                    launcher: launcher
                ))
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
        DispatchQueue.main.async {
            let cachedDecision = self.transientApprovals.decision(for: transientApproval)
            if let decision = cachedDecision {
                if decision == .denied {
                    self.onAccessRequest(accessRequestRecord(
                        request: request,
                        callerPath: callerPath,
                        decision: "Denied",
                        approvalSource: "Auto",
                        reason: "Reused recent denial",
                        launcher: launcher
                    ))
                    self.reply(peer, to: message, ok: false, error: "\(request.op) denied")
                    return
                }
                do {
                    let secrets = try self.approvedSecrets(for: request)
                    self.onAccessRequest(accessRequestRecord(
                        request: request,
                        callerPath: callerPath,
                        decision: "Approved",
                        approvalSource: "Auto",
                        reason: "Reused recent approval",
                        launcher: launcher
                    ))
                    self.reply(peer, to: message, ok: true, error: nil, secrets: secrets)
                } catch {
                    self.onAccessRequest(accessRequestRecord(
                        request: request,
                        callerPath: callerPath,
                        decision: "Failed",
                        approvalSource: "Auto",
                        reason: error.localizedDescription,
                        launcher: launcher
                    ))
                    self.reply(peer, to: message, ok: false, error: error.localizedDescription)
                }
                return
            }

            if let event = approvalEvent(for: cachedDecision) {
                self.sendEvent(event, to: peer)
            }
            let decision = showApprovalAlert(
                request: request,
                callerPath: callerPath,
                pid: pid,
                signing: signing,
                scriptApproval: scriptApproval,
                launcher: launcher,
                launcherFallbackPath: launcherFallbackPath
            )
            guard decision != .denied else {
                self.transientApprovals.remember(.denied, for: transientApproval)
                self.onAccessRequest(accessRequestRecord(
                    request: request,
                    callerPath: callerPath,
                    decision: "Denied",
                    approvalSource: "Manual",
                    reason: "Denied in prompt",
                    launcher: launcher
                ))
                self.reply(peer, to: message, ok: false, error: "\(request.op) denied")
                return
            }
            self.transientApprovals.remember(.approved, for: transientApproval)
            do {
                let secrets = try self.approvedSecrets(for: request)
                self.onAccessRequest(accessRequestRecord(
                    request: request,
                    callerPath: callerPath,
                    decision: "Approved",
                    approvalSource: "Manual",
                    reason: "Approved in prompt",
                    launcher: launcher
                ))
                self.reply(peer, to: message, ok: true, error: nil, secrets: secrets)
            } catch {
                self.onAccessRequest(accessRequestRecord(
                    request: request,
                    callerPath: callerPath,
                    decision: "Failed",
                    approvalSource: "Manual",
                    reason: error.localizedDescription,
                    launcher: launcher
                ))
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

    private func handleLoad(_ message: xpc_object_t, on peer: xpc_connection_t) {
        guard let keyPointer = xpc_dictionary_get_string(message, "key") else {
            reply(peer, to: message, ok: false, error: "invalid load request")
            return
        }
        let key = String(cString: keyPointer)
        guard validSecretKeyName(key) else {
            reply(peer, to: message, ok: false, error: "invalid isotope key name: \(key)")
            return
        }
        guard let value = loadStoredSecret(account: key) else {
            reply(peer, to: message, ok: false, error: "failed to load isotope key \(key): \(errSecItemNotFound)")
            return
        }
        reply(peer, to: message, ok: true, error: nil, value: value)
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
        secrets: [String: String]? = nil,
        value: String? = nil
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
        if let value {
            value.withCString { xpc_dictionary_set_string(response, "value", $0) }
        }
        xpc_connection_send_message(peer, response)
    }

    private func sendEvent(_ event: String, to peer: xpc_connection_t) {
        let message = xpc_dictionary_create_empty()
        event.withCString { xpc_dictionary_set_string(message, "event", $0) }
        xpc_connection_send_message(peer, message)
    }
}

private func isAllowedCaller(path: String, signing: SigningInfo) -> Bool {
    if isTrustedAvCaller(path: path, signing: signing) {
        return true
    }
    if isTrustedGhCaller(path: path, signing: signing) {
        return true
    }
    let name = URL(fileURLWithPath: path).lastPathComponent
    return (name == "supabase" || name == "supabase-go")
        && (signing.identifier == "supabase"
            || signing.identifier == "supabase-go"
            || signing.identifier == "com.supabase.cli")
}

private func isTrustedAvCaller(path: String, signing: SigningInfo) -> Bool {
    URL(fileURLWithPath: path).lastPathComponent == "av"
        && signing.identifier == "com.automicvault.av"
}

private func isTrustedGhCaller(path: String, signing: SigningInfo) -> Bool {
    URL(fileURLWithPath: path).lastPathComponent == "gh"
        && (signing.identifier == "gh" || signing.identifier == "com.github.cli")
}

private struct ResolvedSecretGatePolicy {
    let protection: SecretGateProtection
    let source: String
    let launcher: LauncherIdentity?
}

private func matchingSecretGate(
    request: ApprovalRequest,
    signing: SigningInfo,
    hardeners: [HardenerMetadata],
    service: String = secretGatePoliciesKeychainService
) -> SecretGate? {
    loadSecretGates(hardeners: hardeners, service: service).first { gate in
        gate.routes.contains { route in
            route.operation == request.op
                && route.callerIdentifiers.contains(signing.identifier)
                && normalizedExecutablePath(route.targetPath) == normalizedExecutablePath(request.target)
                && route.scriptPath.map { standardizedPath($0, cwd: request.cwd) }
                    == resolvedShebangScriptPath(request)
                && routeKeysMatch(route.keyPatterns, request.keys)
                && route.replaceExistingEnv == request.replaceExistingEnv
                && route.allowMissingKeys == request.allowMissingKeys
        }
    }
}

private func routeKeysMatch(_ patterns: [String], _ keys: [String]) -> Bool {
    guard !keys.isEmpty else { return false }
    if patterns.allSatisfy({ !$0.hasSuffix("*") }) {
        return patterns.sorted() == keys.sorted()
    }
    return keys.allSatisfy { key in
        patterns.contains { pattern in
            pattern.hasSuffix("*")
                ? key.hasPrefix(String(pattern.dropLast()))
                : key == pattern
        }
    }
}

private func resolveSecretGatePolicy(
    gate: SecretGate,
    launchers: [LauncherIdentity]
) -> ResolvedSecretGatePolicy {
    for launcher in launchers {
        if let policy = gate.appPolicies.first(where: { $0.requirement == launcher.designatedRequirement }) {
            return ResolvedSecretGatePolicy(
                protection: policy.protection,
                source: shortAppName(launcher.identifier),
                launcher: launcher
            )
        }
    }
    return ResolvedSecretGatePolicy(
        protection: gate.defaultProtection,
        source: "All Other Apps",
        launcher: launchers.first
    )
}

private func secretGateProtectionAllows(
    _ protection: SecretGateProtection,
    classification: SecretGateRequestClassification
) -> Bool {
    protection.allows(classification)
}

private func classifySecretGateRequest(
    gateID: String,
    request: ApprovalRequest
) -> SecretGateRequestClassification {
    switch gateID {
    case "gh":
        if ghRequestIsSecretDump(request.args) { return .secretDump }
        return ghRequestIsReadOnly(request.args) ? .readOnly : .mutating
    case "aws":
        return awsRequestIsReadOnly(awsCommandWords(request)) ? .readOnly : .mutating
    default:
        return .unknown
    }
}

private func ghRequestIsSecretDump(_ args: [String]) -> Bool {
    let words = ghCommandWords(args).map { $0.lowercased() }
    guard words.count >= 2, words[0] == "auth" else { return false }
    return words[1] == "token"
        || (words[1] == "status" && words.dropFirst(2).contains("--show-token"))
}

private func awsRequestIsReadOnly(_ args: [String]) -> Bool {
    let words = awsCommandWords(args).map { $0.lowercased() }
    guard let service = words.first else { return false }
    if service == "help" { return true }
    guard words.count >= 2 else { return false }
    let operation = words[1]
    if operation == "help" { return true }
    if service == "s3", operation == "ls" { return true }
    if service == "sts", operation == "get-caller-identity" { return true }
    if service == "s3api", operation.hasPrefix("head-") { return true }
    return operation.hasPrefix("list-") || operation.hasPrefix("describe-")
}

private func awsCommandWords(_ request: ApprovalRequest) -> [String] {
    guard let scriptPath = resolvedShebangScriptPath(request),
          let firstArg = request.args.first,
          standardizedPath(firstArg, cwd: request.cwd) == scriptPath
    else {
        return []
    }
    return Array(request.args.dropFirst())
}

private func awsCommandWords(_ args: [String]) -> [String] {
    var index = 0
    while index < args.count {
        let arg = args[index]
        if arg == "--" {
            return []
        }
        if awsGlobalOptionsWithValue.contains(arg) {
            index += 2
            continue
        }
        if awsGlobalOptionsWithValue.contains(where: { arg.hasPrefix("\($0)=") }) || awsGlobalFlags.contains(arg) {
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

private let awsGlobalOptionsWithValue = Set([
    "--ca-bundle",
    "--cli-binary-format",
    "--cli-input-json",
    "--cli-input-yaml",
    "--color",
    "--endpoint-url",
    "--max-items",
    "--output",
    "--page-size",
    "--profile",
    "--query",
    "--region",
    "--starting-token"
])

private let awsGlobalFlags = Set([
    "--debug",
    "--no-cli-auto-prompt",
    "--no-cli-pager",
    "--no-paginate",
    "--no-sign-request",
    "--no-verify-ssl",
    "--only-show-errors",
    "--version"
])

private func standardizedPath(_ path: String, cwd: String) -> String {
    let url = path.hasPrefix("/")
        ? URL(fileURLWithPath: path)
        : URL(fileURLWithPath: cwd).appendingPathComponent(path)
    return url.standardizedFileURL.path
}

private func ghRequestIsReadOnly(_ args: [String]) -> Bool {
    let words = ghCommandWords(args).map { $0.lowercased() }
    guard let firstWord = words.first else { return false }
    let command = ghCanonicalCommand(firstWord)
    if words.contains("--show-token") { return false }
    if command == "api" { return ghApiRequestIsReadOnly(Array(words.dropFirst())) }
    if ["alias", "extension", "config", "skill"].contains(command) { return false }
    if ["status", "browse", "search"].contains(command) { return true }
    guard words.count >= 2 else { return false }
    let subcommand = words[1]
    switch command {
    case "auth":
        return subcommand == "status"
    case "repo":
        return subcommand == "view" || ghSubcommandIsList(subcommand)
    case "issue":
        return ["view", "status"].contains(subcommand) || ghSubcommandIsList(subcommand)
    case "pr":
        return ["view", "status", "checks", "diff"].contains(subcommand) || ghSubcommandIsList(subcommand)
    case "run":
        return ["view", "download"].contains(subcommand) || ghSubcommandIsList(subcommand)
    case "workflow":
        return subcommand == "view" || ghSubcommandIsList(subcommand)
    case "release":
        return ["view", "download"].contains(subcommand) || ghSubcommandIsList(subcommand)
    case "gist":
        return subcommand == "view" || ghSubcommandIsList(subcommand)
    case "cache", "secret", "variable", "ruleset", "org", "label", "gpg-key", "ssh-key":
        return ghSubcommandIsList(subcommand) || (command == "ruleset" && subcommand == "view")
    case "attestation":
        return ["verify", "download", "trusted-root"].contains(subcommand)
    case "agent-task":
        return ["view", "list"].contains(subcommand)
    default:
        return false
    }
}

private func ghCanonicalCommand(_ command: String) -> String {
    switch command {
    case "agent-tasks", "agent", "agents":
        return "agent-task"
    case "at":
        return "attestation"
    case "rs":
        return "ruleset"
    default:
        return command
    }
}

private func ghSubcommandIsList(_ subcommand: String) -> Bool {
    subcommand == "list" || subcommand == "ls"
}

private func ghApiRequestIsReadOnly(_ args: [String]) -> Bool {
    var index = 0
    var endpointSeen = false
    var method: String?
    var hasFields = false
    while index < args.count {
        let arg = args[index]
        switch arg {
        case "--":
            return false
        case "-x", "--method":
            guard index + 1 < args.count else { return false }
            method = args[index + 1].uppercased()
            index += 2
        case "-f", "--field", "--raw-field":
            guard index + 1 < args.count else { return false }
            hasFields = true
            index += 2
        case "--input":
            return false
        case "-h", "--header", "--preview", "--cache", "-q", "--jq", "-t", "--template", "--hostname":
            guard index + 1 < args.count else { return false }
            index += 2
        case "-i", "--include", "--paginate", "--slurp", "--silent", "--verbose":
            index += 1
        default:
            if let value = arg.value(afterOption: "--method=") {
                method = value.uppercased()
            } else if arg.hasPrefix("--field=") || arg.hasPrefix("--raw-field=") {
                hasFields = true
            } else if arg.hasPrefix("--input=") {
                return false
            } else if arg.hasPrefix("--header=")
                || arg.hasPrefix("--preview=")
                || arg.hasPrefix("--cache=")
                || arg.hasPrefix("--jq=")
                || arg.hasPrefix("--template=")
                || arg.hasPrefix("--hostname=") {
                // read-only option with inline value
            } else if arg.hasPrefix("-x"), arg.count > 2 {
                method = String(arg.dropFirst(2)).uppercased()
            } else if arg.hasPrefix("-f") {
                hasFields = true
            } else if arg.hasPrefix("-") {
                return false
            } else {
                endpointSeen = true
            }
            index += 1
        }
    }
    return endpointSeen && (method ?? (hasFields ? "POST" : "GET")) == "GET"
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

private extension String {
    func value(afterOption prefix: String) -> String? {
        hasPrefix(prefix) ? String(dropFirst(prefix.count)) : nil
    }
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
    guard shouldHandOffToLaunchAgent(),
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
    try runLaunchctl(["kickstart", "\(domain)/\(approvalLaunchAgentName)"])
    return true
}

private func shouldHandOffToLaunchAgent(
    environment: [String: String] = ProcessInfo.processInfo.environment,
    launchAgentURL: URL? = bundledLaunchAgentURL()
) -> Bool {
    !isLaunchAgentInstance(environment: environment) && launchAgentURL != nil
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

private func launcherIdentities(for identity: AVProcessIdentity) -> [LauncherIdentity] {
    for pid in launcherAncestorStartPIDs(identity) {
        let launchers = launcherIdentities(startingAt: pid)
        if !launchers.isEmpty { return launchers }
    }
    return []
}

private func launcherAncestorStartPIDs(_ identity: AVProcessIdentity) -> [pid_t] {
    var seen = Set<pid_t>()
    return [identity.ppid, identity.sid].filter { $0 > 1 && seen.insert($0).inserted }
}

private func launcherFallbackPath(for identity: AVProcessIdentity) -> String? {
    launcherAncestorStartPIDs(identity)
        .compactMap(launcherAncestorPath(startingAt:))
        .max { $0.depth < $1.depth }?
        .path
}

private func launcherAncestorPath(startingAt startPID: pid_t) -> (path: String, depth: Int)? {
    var pid = startPID
    var seen = Set<pid_t>()
    var result: (path: String, depth: Int)?
    for depth in 1...32 {
        guard pid > 1, seen.insert(pid).inserted else { return result }
        var identity = AVProcessIdentity()
        guard av_process_identity(pid, &identity) else { return result }
        let path = pathString(identity)
        if !path.isEmpty { result = (path, depth) }
        pid = identity.ppid
    }
    return result
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

private final class ApprovalPanel: NSPanel {
    override var canBecomeKey: Bool { true }
}

@MainActor
private func fitApprovalPanel(_ panel: NSPanel, maximumHeight: CGFloat, animate: Bool) {
    guard let contentView = panel.contentView else { return }
    contentView.layoutSubtreeIfNeeded()
    var size = contentView.fittingSize
    size.height = min(size.height, maximumHeight)
    var frame = panel.frame
    let top = frame.maxY
    frame.size = size
    frame.origin.y = top - size.height
    if let visibleFrame = panel.screen?.visibleFrame ?? NSScreen.main?.visibleFrame {
        frame.origin.y = max(visibleFrame.minY, min(frame.origin.y, visibleFrame.maxY - size.height))
    }
    panel.setFrame(frame, display: true, animate: animate)
}

@MainActor
private func showApprovalAlert(
    request: ApprovalRequest,
    callerPath: String,
    pid: pid_t,
    signing: SigningInfo,
    scriptApproval: ScriptApproval?,
    launcher: LauncherIdentity?,
    launcherFallbackPath: String
) -> ApprovalDecision {
    NSApp.activate(ignoringOtherApps: true)
    let requester = approvalPromptRequester(launcher: launcher, fallback: launcherFallbackPath)
    let content = ApprovalPromptContent(
        requesterName: requester.name,
        requesterIconPath: requester.iconPath,
        command: prettyShellCommand(target: request.target, args: request.args),
        title: request.title,
        detail: request.detail,
        cwd: request.cwd,
        keys: request.keys.joined(separator: ", "),
        sections: approvalPromptSections(
            request: request,
            callerPath: callerPath,
            pid: pid,
            signing: signing,
            scriptApproval: scriptApproval,
            launcher: launcher
        )
    )
    var decision = ApprovalDecision.denied
    let maximumHeight = NSScreen.main?.visibleFrame.height ?? 660
    let panel = ApprovalPanel(
        contentRect: NSRect(x: 0, y: 0, width: 560, height: 660),
        styleMask: [.borderless],
        backing: .buffered,
        defer: false
    )
    panel.backgroundColor = .clear
    panel.isOpaque = false
    panel.hasShadow = true
    panel.isMovableByWindowBackground = true
    panel.isFloatingPanel = true
    panel.hidesOnDeactivate = false
    panel.level = .modalPanel
    panel.collectionBehavior = [.moveToActiveSpace, .fullScreenAuxiliary]
    panel.contentView = NSHostingView(
        rootView: ApprovalPromptView(
            content: content,
            maximumHeight: maximumHeight,
            decide: {
                decision = $0
                NSApp.stopModal()
            },
            contentSizeDidChange: { [weak panel] in
                Task { @MainActor in
                    await Task.yield()
                    if let panel {
                        fitApprovalPanel(panel, maximumHeight: maximumHeight, animate: true)
                    }
                }
            }
        )
    )
    fitApprovalPanel(panel, maximumHeight: maximumHeight, animate: false)
    panel.center()
    panel.makeKeyAndOrderFront(nil)
    NSApp.runModal(for: panel)
    panel.orderOut(nil)
    return decision
}

private func approvalPromptRequester(
    launcher: LauncherIdentity?,
    fallback: String
) -> (name: String, iconPath: String) {
    guard let launcher else {
        return (URL(fileURLWithPath: fallback).lastPathComponent, fallback)
    }
    if let appURL = appBundleURL(containing: launcher.path)
        ?? NSWorkspace.shared.urlForApplication(withBundleIdentifier: launcher.identifier)
    {
        let name = Bundle(url: appURL)?.object(forInfoDictionaryKey: "CFBundleDisplayName") as? String
            ?? appURL.deletingPathExtension().lastPathComponent
        return (name, appURL.path)
    }
    return (shortAppName(launcher.identifier), launcher.path)
}

private func prettyShellCommand(target: String, args: [String]) -> String {
    ([target] + args).map(shellQuote).enumerated().map { index, word in
        if args.isEmpty { return word }
        return index == 0 ? "\(word) \\" : "  \(word)" + (index == args.count ? "" : " \\")
    }.joined(separator: "\n")
}

private func shellQuote(_ word: String) -> String {
    guard !word.isEmpty,
          word.rangeOfCharacter(from: CharacterSet.whitespacesAndNewlines.union(CharacterSet(charactersIn: #"'"\\$`!&|;()<>{}[]*?"#))) == nil
    else {
        return "'" + word.replacingOccurrences(of: "'", with: "'\\''") + "'"
    }
    return word
}

@MainActor
private func approvalPromptSections(
    request: ApprovalRequest,
    callerPath: String,
    pid: pid_t,
    signing: SigningInfo,
    scriptApproval: ScriptApproval?,
    launcher: LauncherIdentity?
) -> [ApprovalPromptSection] {
    var sections = [
        ApprovalPromptSection("Environment", "arrow.triangle.2.circlepath", [
            ApprovalPromptRow("Existing", request.envConflicts.isEmpty ? "(none)" : request.envConflicts.joined(separator: ", ")),
            ApprovalPromptRow("Replace existing", request.replaceExistingEnv ? "yes" : "no"),
            ApprovalPromptRow("Allow missing keys", request.allowMissingKeys ? "yes" : "no"),
        ]),
        ApprovalPromptSection("Caller Identity", "terminal", [
            ApprovalPromptRow("Caller", "\(callerPath) (pid \(pid))"),
            ApprovalPromptRow("Signed", "\(signing.identifier) / \(signing.teamIdentifier)"),
        ]),
    ]

    sections.append(ApprovalPromptSection("Launcher", "app.badge", launcher.map {
        [
            ApprovalPromptRow("App", "\($0.identifier) (pid \($0.pid))"),
            ApprovalPromptRow("Path", $0.path),
            ApprovalPromptRow("Signed", "\($0.identifier) / \($0.teamIdentifier)"),
        ]
    } ?? [
        ApprovalPromptRow("Status", "unavailable; persistent auto-approve disabled"),
    ]))

    if let scriptApproval {
        sections.append(ApprovalPromptSection("Script", "doc.text", [
            ApprovalPromptRow("Path", scriptApproval.path),
            ApprovalPromptRow("Checksum", scriptApproval.checksum),
        ]))
    } else if let script = request.shebangScript {
        sections.append(ApprovalPromptSection("Script", "doc.text", [
            ApprovalPromptRow("Path", script),
            ApprovalPromptRow("Checksum", "unavailable"),
        ]))
    }

    return sections
}

private struct ApprovalPromptSection: Identifiable {
    let id: String
    let title: String
    let systemImage: String
    let rows: [ApprovalPromptRow]

    init(_ title: String, _ systemImage: String, _ rows: [ApprovalPromptRow]) {
        self.id = title
        self.title = title
        self.systemImage = systemImage
        self.rows = rows
    }
}

private struct ApprovalPromptRow: Identifiable {
    let id: String
    let label: String
    let value: String

    init(_ label: String, _ value: String) {
        self.id = label
        self.label = label
        self.value = value
    }
}

private struct ApprovalPromptContent {
    let requesterName: String
    let requesterIconPath: String
    let command: String
    let title: String?
    let detail: String?
    let cwd: String
    let keys: String
    let sections: [ApprovalPromptSection]
}

private struct ApprovalPromptView: View {
    let content: ApprovalPromptContent
    var maximumHeight: CGFloat? = nil
    let decide: (ApprovalDecision) -> Void
    let contentSizeDidChange: () -> Void
    @State private var showsDetails = false

    var body: some View {
        VStack(spacing: 22) {
            VStack(spacing: 8) {
                Image(nsImage: NSWorkspace.shared.icon(forFile: content.requesterIconPath))
                    .resizable()
                    .interpolation(.high)
                    .frame(width: 72, height: 72)
                    .clipShape(RoundedRectangle(cornerRadius: 16, style: .continuous))
                    .accessibilityLabel(content.requesterName)
                Text(content.requesterName)
                    .font(.title2.weight(.bold))
                    .lineLimit(1)
                Text("WANTS TO RUN")
                    .font(.caption.weight(.semibold))
                    .tracking(1.6)
                    .foregroundStyle(.secondary)
            }

            ApprovalPromptCommandView(content: content)
                .layoutPriority(-1)

            VStack(alignment: .leading, spacing: 5) {
                if let title = content.title, !title.isEmpty {
                    Text(title)
                        .font(.headline)
                }
                if let detail = content.detail, !detail.isEmpty {
                    Text(detail)
                        .font(.callout)
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                if content.title?.isEmpty != false, content.detail?.isEmpty != false {
                    Text("Review the request details before allowing access.")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)

            DisclosureGroup(isExpanded: $showsDetails) {
                ScrollView {
                    VStack(spacing: 8) {
                        ForEach(content.sections) { section in
                            ApprovalPromptSectionView(section: section)
                        }
                    }
                    .padding(.top, 8)
                }
                .frame(maxHeight: 170)
                .scrollIndicators(.visible)
            } label: {
                Label("Details", systemImage: "info.circle")
                    .font(.callout.weight(.medium))
            }
            .transaction { $0.animation = nil }
            .onChange(of: showsDetails) { _, _ in contentSizeDidChange() }

            HStack(spacing: 12) {
                Button("Deny", role: .cancel) { decide(.denied) }
                    .buttonStyle(.glass)
                    .controlSize(.large)
                    .frame(maxWidth: .infinity)
                    .keyboardShortcut(.cancelAction)
                Button("Approve Once") { decide(.approved) }
                    .buttonStyle(.glassProminent)
                    .controlSize(.large)
                    .tint(.blue)
                    .frame(maxWidth: .infinity)
                    .keyboardShortcut(.defaultAction)
            }

            Text("“Always Approve” for apps available in the Automic Vault app")
                .font(.footnote)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
        }
        .padding(28)
        .frame(maxHeight: maximumHeight)
        .frame(width: 560)
        .fixedSize(horizontal: false, vertical: true)
        .background {
            RoundedRectangle(cornerRadius: 28, style: .continuous)
                .fill(.regularMaterial)
                // .overlay {
                //     RoundedRectangle(cornerRadius: 28, style: .continuous)
                //         .fill(.blue.opacity(0.18))
                // }
        }
        .overlay {
            RoundedRectangle(cornerRadius: 28, style: .continuous)
                .stroke(.white.opacity(0.18), lineWidth: 1)
        }
        .contentShape(RoundedRectangle(cornerRadius: 28, style: .continuous))
    }
}

private struct ApprovalPromptCommandView: View {
    let content: ApprovalPromptContent

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            ScrollView([.horizontal, .vertical]) {
                Text(content.command)
                    .font(.system(.body, design: .monospaced))
                    .foregroundStyle(.white)
                    .textSelection(.enabled)
                    .fixedSize(horizontal: true, vertical: true)
            }
            .scrollIndicators(.visible)
            ViewThatFits(in: .horizontal) {
                HStack(spacing: 14) {
                    ApprovalPromptInlineMeta(label: "cwd", value: content.cwd)
                    ApprovalPromptInlineMeta(label: "keys", value: content.keys)
                }
                VStack(alignment: .leading, spacing: 5) {
                    ApprovalPromptInlineMeta(label: "cwd", value: content.cwd)
                    ApprovalPromptInlineMeta(label: "keys", value: content.keys)
                }
            }
        }
        .padding(18)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.black.opacity(0.72), in: RoundedRectangle(cornerRadius: 14, style: .continuous))
        .overlay {
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .stroke(.white.opacity(0.12), lineWidth: 1)
        }
    }
}

private struct ApprovalPromptInlineMeta: View {
    let label: String
    let value: String

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 5) {
            Text(label)
                .font(.caption.weight(.semibold))
                .foregroundStyle(.white.opacity(0.6))
            Text(value.isEmpty ? "(none)" : value)
                .font(.system(.caption, design: .monospaced))
                .foregroundStyle(.white.opacity(0.82))
                .lineLimit(1)
                .truncationMode(.middle)
                .textSelection(.enabled)
        }
    }
}

private struct ApprovalPromptSectionView: View {
    let section: ApprovalPromptSection

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label(section.title, systemImage: section.systemImage)
                .font(.headline)
                .symbolRenderingMode(.hierarchical)
            VStack(alignment: .leading, spacing: 6) {
                ForEach(section.rows) { row in
                    HStack(alignment: .firstTextBaseline, spacing: 10) {
                        Text(row.label)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .frame(width: 124, alignment: .trailing)
                        Text(row.value)
                            .font(.system(.callout, design: .monospaced))
                            .foregroundStyle(.primary)
                            .textSelection(.enabled)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                }
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            Color(nsColor: .controlBackgroundColor).opacity(0.72),
            in: RoundedRectangle(cornerRadius: 10, style: .continuous)
        )
    }
}

private struct AutoApprovedToastView: View {
    let record: AutoApprovalRecord
    let open: () -> Void

    var body: some View {
        Button(action: open) {
            content
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Open approval details for \(record.command)")
    }

    private var content: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 12) {
                Image(nsImage: NSWorkspace.shared.icon(forFile: record.launcherIconPath))
                    .resizable()
                    .interpolation(.high)
                    .frame(width: 42, height: 42)
                    .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
                    .accessibilityLabel(record.launcher)
                VStack(alignment: .leading, spacing: 2) {
                    Text(record.launcher)
                        .font(.headline)
                        .lineLimit(1)
                    Text("AUTO APPROVED")
                        .font(.caption2.weight(.semibold))
                        .tracking(1.2)
                        .foregroundStyle(.secondary)
                }
                Spacer(minLength: 8)
                Image(systemName: "checkmark.shield.fill")
                    .font(.title2)
                    .symbolRenderingMode(.hierarchical)
                    .foregroundStyle(.green)
                    .accessibilityLabel("Approved")
            }

            VStack(alignment: .leading, spacing: 3) {
                Text(record.command)
                    .font(.system(.callout, design: .monospaced).weight(.medium))
                    .foregroundStyle(.white)
                    .fixedSize(horizontal: false, vertical: true)
                Text(record.keys.joined(separator: ", "))
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(.white.opacity(0.68))
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 9)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(.black.opacity(0.72), in: RoundedRectangle(cornerRadius: 10, style: .continuous))
        }
        .padding(16)
        .frame(width: 360)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
        .overlay {
            RoundedRectangle(cornerRadius: 16, style: .continuous)
                .stroke(.white.opacity(0.18), lineWidth: 1)
        }
        .contentShape(RoundedRectangle(cornerRadius: 16, style: .continuous))
    }
}

private func autoApprovalToastFrame(anchor: NSRect, visibleFrame: NSRect, size: NSSize) -> NSRect {
    let margin: CGFloat = 8
    let x = min(max(anchor.midX - size.width / 2, visibleFrame.minX + margin), visibleFrame.maxX - size.width - margin)
    let y = max(visibleFrame.minY + margin, min(anchor.minY - 4, visibleFrame.maxY) - size.height)
    return NSRect(origin: NSPoint(x: x, y: y), size: size)
}

@MainActor
private func showAutoApprovedToast(
    _ record: AutoApprovalRecord,
    below button: NSStatusBarButton?,
    open: @escaping () -> Void
) {
    guard let button, let statusWindow = button.window else { return }
    let anchor = statusWindow.convertToScreen(button.convert(button.bounds, to: nil))
    let window = NSPanel(
        contentRect: .zero,
        styleMask: [.borderless, .nonactivatingPanel],
        backing: .buffered,
        defer: false
    )
    let hostingView = NSHostingView(rootView: AutoApprovedToastView(record: record) { [weak window] in
        if let window {
            window.orderOut(nil)
            toastWindows.removeAll { $0 === window }
        }
        open()
    })
    let size = hostingView.fittingSize
    hostingView.frame.size = size
    let visibleFrame = statusWindow.screen?.visibleFrame ?? NSScreen.main?.visibleFrame
        ?? NSRect(x: 0, y: 0, width: 800, height: 600)
    let frame = autoApprovalToastFrame(anchor: anchor, visibleFrame: visibleFrame, size: size)
    window.setFrame(frame, display: false)
    window.level = .statusBar
    window.isOpaque = false
    window.backgroundColor = .clear
    window.hasShadow = true
    window.contentView = hostingView
    window.alphaValue = 0
    toastWindows.append(window)
    window.orderFront(nil)
    NSAnimationContext.runAnimationGroup { context in
        context.duration = 0.15
        window.animator().alphaValue = 1
    }

    DispatchQueue.main.asyncAfter(deadline: .now() + 5) {
        NSAnimationContext.runAnimationGroup({ context in
            context.duration = 0.25
            window.animator().alphaValue = 0
        }, completionHandler: {
            Task { @MainActor in
                window.orderOut(nil)
                toastWindows.removeAll { $0 === window }
            }
        })
    }
}

@MainActor
private func runApprovalSelfCheck() -> Int32 {
    let requester = approvalPromptRequester(
        launcher: LauncherIdentity(
            pid: 41,
            path: "/Applications/Vaultty.app/Contents/Helpers/vaultty-sessiond",
            identifier: "com.automicvault.vaultty",
            teamIdentifier: "TEAM",
            designatedRequirement: #"identifier "com.automicvault.vaultty" and anchor apple generic"#
        ),
        fallback: "/opt/homebrew/bin/gh"
    )
    let unverifiedRequester = approvalPromptRequester(
        launcher: nil,
        fallback: "/Applications/Vaultty.app/Contents/Helpers/vaultty-sessiond"
    )
    let collapsedHeight = NSHostingView(
        rootView: ApprovalPromptView(
            content: ApprovalPromptContent(
                requesterName: requester.name,
                requesterIconPath: requester.iconPath,
                command: "/opt/homebrew/bin/gh auth token",
                title: "GitHub token requested",
                detail: "gh needs the GitHub token",
                cwd: "/tmp",
                keys: "GH_TOKEN_GITHUB_COM",
                sections: []
            ),
            decide: { _ in },
            contentSizeDidChange: {}
        )
    ).fittingSize.height
    let constrainedHeight = NSHostingView(
        rootView: ApprovalPromptView(
            content: ApprovalPromptContent(
                requesterName: requester.name,
                requesterIconPath: requester.iconPath,
                command: Array(repeating: "  --long-option \\", count: 100).joined(separator: "\n"),
                title: nil,
                detail: nil,
                cwd: "/tmp",
                keys: "GH_TOKEN_GITHUB_COM",
                sections: []
            ),
            maximumHeight: 500,
            decide: { _ in },
            contentSizeDidChange: {}
        )
    ).fittingSize.height
    guard prettyShellCommand(target: "/bin/echo", args: ["hello world", "it's-ok"]) == """
    /bin/echo \\
      'hello world' \\
      'it'\\''s-ok'
    """,
          prettyShellCommand(target: "/bin/echo", args: []) == "/bin/echo",
          requester.name == "Vaultty",
          requester.iconPath == "/Applications/Vaultty.app",
          unverifiedRequester.name == "vaultty-sessiond",
          unverifiedRequester.iconPath == "/Applications/Vaultty.app/Contents/Helpers/vaultty-sessiond",
          collapsedHeight > 0,
          collapsedHeight < 660,
          constrainedHeight <= 500
    else {
        return 1
    }
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
    var detachedCaller = AVProcessIdentity()
    detachedCaller.ppid = 1
    detachedCaller.sid = 43
    let pythonSigning = LiveSigningInfo(
        identifier: "org.python.python",
        teamIdentifier: "unknown",
        designatedRequirement: #"identifier "org.python.python" and anchor apple generic"#,
        mainExecutable: "/opt/homebrew/Cellar/python@3.14/3.14.6/Frameworks/Python.framework/Versions/3.14/Resources/Python.app/Contents/MacOS/Python",
        isAdHoc: true
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
    guard parentlessVaulttyLauncher?.designatedRequirement == vaulttyAppSigning.designatedRequirement,
          vaulttyBridgeLauncher?.designatedRequirement == vaulttyAppSigning.designatedRequirement,
          launcherAncestorStartPIDs(detachedCaller) == [43],
          launcherIdentity(pid: 45, path: pythonSigning.mainExecutable, signing: pythonSigning) == nil,
          launcherIdentity(pid: 46, path: "/usr/local/bin/av", signing: unbundledSigning) == nil
    else {
        return 1
    }
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
    let ghMetadata = HardenerMetadata(
        name: "gh",
        hardened: true,
        secretGate: SecretGateDescriptor(
            id: "gh",
            keyPatterns: ["GH_TOKEN_*"],
            routes: [SecretGateRoute(
                operation: "keys",
                scriptPath: nil,
                targetPath: "/opt/homebrew/opt/gh-cli/bin/gh",
                callerIdentifiers: ["gh", "com.github.cli"],
                keyPatterns: ["GH_TOKEN_*"],
                replaceExistingEnv: true,
                allowMissingKeys: false
            )]
        )
    )
    guard matchingSecretGate(request: readOnlyGh, signing: ghSigning, hardeners: [ghMetadata])?.id == "gh",
          matchingSecretGate(request: ghRequest(keys: ["OTHER_TOKEN"]), signing: ghSigning, hardeners: [ghMetadata]) == nil,
          matchingSecretGate(request: ghRequest(op: "inject"), signing: ghSigning, hardeners: [ghMetadata]) == nil,
          matchingSecretGate(
              request: readOnlyGh,
              signing: SigningInfo(identifier: "com.automicvault.av", teamIdentifier: "TEAM"),
              hardeners: [ghMetadata]
          ) == nil,
          classifySecretGateRequest(gateID: "gh", request: readOnlyGh) == .readOnly,
          classifySecretGateRequest(gateID: "gh", request: ghRequest(args: ["repo", "delete", "owner/name"])) == .mutating,
          classifySecretGateRequest(gateID: "gh", request: ghRequest(args: ["auth", "token"])) == .secretDump,
          classifySecretGateRequest(gateID: "gh", request: ghRequest(args: ["auth", "status", "--show-token"])) == .secretDump
    else { return 1 }

    let avSigning = SigningInfo(identifier: "com.automicvault.av", teamIdentifier: "TEAM")
    func awsRequest(
        keys: [String] = ["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY"],
        args: [String] = ["/usr/local/bin/aws", "s3", "ls"],
        shebangScript: String? = "/usr/local/bin/aws"
    ) -> ApprovalRequest {
        ApprovalRequest(
            op: "inject",
            keys: keys,
            target: "/bin/zsh",
            args: args,
            cwd: "/tmp",
            replaceExistingEnv: false,
            allowMissingKeys: false,
            envConflicts: [],
            shebangScript: shebangScript,
            tool: nil,
            title: nil,
            detail: nil
        )
    }
    let readOnlyAws = awsRequest()
    let awsMetadata = HardenerMetadata(
        name: "aws",
        hardened: true,
        secretGate: SecretGateDescriptor(
            id: "aws",
            keyPatterns: ["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY"],
            routes: [SecretGateRoute(
                operation: "inject",
                scriptPath: "/usr/local/bin/aws",
                targetPath: "/bin/zsh",
                callerIdentifiers: ["com.automicvault.av"],
                keyPatterns: ["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY"],
                replaceExistingEnv: false,
                allowMissingKeys: false
            )]
        )
    )
    guard matchingSecretGate(request: readOnlyAws, signing: avSigning, hardeners: [awsMetadata])?.id == "aws",
          matchingSecretGate(request: awsRequest(keys: ["AWS_ACCESS_KEY_ID"]), signing: avSigning, hardeners: [awsMetadata]) == nil,
          matchingSecretGate(request: awsRequest(shebangScript: nil), signing: avSigning, hardeners: [awsMetadata]) == nil,
          matchingSecretGate(
              request: readOnlyAws,
              signing: SigningInfo(identifier: "aws", teamIdentifier: "TEAM"),
              hardeners: [awsMetadata]
          ) == nil,
          classifySecretGateRequest(gateID: "aws", request: readOnlyAws) == .readOnly,
          classifySecretGateRequest(
              gateID: "aws",
              request: awsRequest(args: ["/usr/local/bin/aws", "s3", "rm", "s3://bucket/key"])
          ) == .mutating,
          SecretGateRequestClassification.allCases.allSatisfy({
              secretGateProtectionAllows(.fullIncludingSecretDumps, classification: $0)
          }),
          !secretGateProtectionAllows(.noAccess, classification: .readOnly),
          secretGateProtectionAllows(.readOnly, classification: .readOnly),
          !secretGateProtectionAllows(.readOnly, classification: .unknown),
          !secretGateProtectionAllows(.fullExceptSecretDumps, classification: .secretDump),
          secretGateProtectionAllows(.fullExceptSecretDumps, classification: .unknown)
    else { return 1 }

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
        ["repo", "ls"],
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
        ["rs", "view"],
        ["rs", "list"],
        ["rs", "ls"],
        ["attestation", "verify"],
        ["attestation", "download"],
        ["attestation", "trusted-root"],
        ["at", "verify"],
        ["at", "download"],
        ["at", "trusted-root"],
        ["agent-task", "view"],
        ["agent-task", "list"],
        ["agent", "view"],
        ["agents", "list"],
        ["agent-tasks", "list"],
        ["org", "list"],
        ["label", "list"],
        ["gpg-key", "list"],
        ["ssh-key", "list"],
        ["-R", "owner/repo", "pr", "view"],
        ["--hostname=github.example.com", "repo", "view"],
        ["api", "repos/owner/repo"],
        ["api", "--method", "GET", "repos/owner/repo"],
        ["api", "-XGET", "-H", "Accept: application/vnd.github+json", "repos/owner/repo/releases/latest"],
        ["api", "--method=GET", "-f", "per_page=1", "search/issues"],
        ["api", "--paginate", "repos/owner/repo/actions/runs", "--jq", ".workflow_runs[].id"],
    ]
    guard allowed.allSatisfy(ghRequestIsReadOnly) else { return 1 }

    let denied = [
        ["api"],
        ["api", "--method", "POST", "repos/owner/repo/dispatches"],
        ["api", "-X", "DELETE", "repos/owner/repo"],
        ["api", "-f", "name=value", "repos/owner/repo"],
        ["api", "--input", "body.json", "repos/owner/repo"],
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

private func runAwsReadOnlySelfCheck() -> Int32 {
    let allowed = [
        ["s3", "ls"],
        ["--profile", "dev", "s3", "ls"],
        ["--region=us-east-1", "ec2", "describe-instances"],
        ["ec2", "describe-vpcs", "--filters", "Name=is-default,Values=true"],
        ["iam", "list-users"],
        ["s3api", "list-objects-v2"],
        ["s3api", "head-object"],
        ["sts", "get-caller-identity"],
        ["help"],
    ]
    guard allowed.allSatisfy(awsRequestIsReadOnly) else { return 1 }

    let denied = [
        ["s3", "rm", "s3://bucket/key"],
        ["s3", "cp", "file", "s3://bucket/key"],
        ["ec2", "start-instances"],
        ["lambda", "invoke"],
        ["sts", "get-session-token"],
        ["ecr", "get-login-password"],
        ["secretsmanager", "get-secret-value"],
        ["ssm", "get-parameter", "--with-decryption"],
        ["configure", "get", "aws_secret_access_key"],
        ["--unknown", "s3", "ls"],
        [],
    ]
    guard denied.allSatisfy({ !awsRequestIsReadOnly($0) }) else { return 1 }
    return 0
}

private func runTransientApprovalSelfCheck() -> Int32 {
    func key(
        startUsec: UInt64 = 456,
        args: [String] = ["repo", "view"],
        keys: [String] = ["GH_TOKEN_GITHUB_COM"]
    ) -> TransientApprovalKey {
        TransientApprovalKey(
            pid: 123,
            startUsec: startUsec,
            callerPath: "/opt/homebrew/bin/gh",
            signingIdentifier: "gh",
            signingTeamIdentifier: "TEAM",
            op: "keys",
            keys: keys,
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
    let denial = key(
        args: ["auth", "token"],
        keys: ["GH_TOKEN_GITHUB_COM_MXCL"]
    )
    let fallbackAfterDenial = key(args: ["auth", "token"])
    var cache = TransientApprovalCache()
    cache.remember(.approved, for: approval, now: Date(timeIntervalSince1970: 100))
    guard cache.decision(for: approval, now: Date(timeIntervalSince1970: 200)) == .approved,
          cache.decision(for: fallbackAfterDenial, now: Date(timeIntervalSince1970: 200)) == nil,
          cache.decision(for: key(startUsec: 789), now: Date(timeIntervalSince1970: 200)) == nil
    else {
        return 1
    }
    cache.remember(.denied, for: denial, now: Date(timeIntervalSince1970: 200))
    guard cache.decision(for: denial, now: Date(timeIntervalSince1970: 300)) == .denied,
          cache.decision(for: fallbackAfterDenial, now: Date(timeIntervalSince1970: 300)) == .denied,
          cache.decision(for: key(startUsec: 789), now: Date(timeIntervalSince1970: 300)) == nil,
          cache.decision(for: fallbackAfterDenial, now: Date(timeIntervalSince1970: 501)) == nil
    else {
        return 1
    }
    return 0
}

private func runLaunchAgentHandoffSelfCheck() -> Int32 {
    guard !isLaunchAgentInstance(environment: [:]),
          isLaunchAgentInstance(environment: ["XPC_SERVICE_NAME": approvalLaunchAgentName]),
          shouldHandOffToLaunchAgent(environment: [:], launchAgentURL: URL(fileURLWithPath: "/tmp/agent.plist")),
          !shouldHandOffToLaunchAgent(environment: ["XPC_SERVICE_NAME": approvalLaunchAgentName], launchAgentURL: URL(fileURLWithPath: "/tmp/agent.plist")),
          !shouldHandOffToLaunchAgent(environment: [:], launchAgentURL: nil)
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
        args: ["/usr/local/bin/aws", "s3", "ls"],
        cwd: "/tmp",
        replaceExistingEnv: true,
        allowMissingKeys: false,
        envConflicts: [],
        shebangScript: "/usr/local/bin/aws",
        tool: nil,
        title: nil,
        detail: nil
    )
    let recordedApproval = AccessRequestRecord(
        date: Date(timeIntervalSince1970: 18_900),
        tool: "aws",
        command: "aws s3 ls",
        decision: "Approved",
        approvalSource: "Auto",
        reason: "Read Only from app policy",
        launcher: "Codex",
        callerPath: "/usr/local/bin/av",
        target: "/bin/zsh",
        cwd: "/tmp",
        keys: ["AWS_SECRET_ACCESS_KEY"],
        detail: nil
    )
    guard let restoredApproval = autoApprovalRecord(recordedApproval) else { return 1 }
    guard shortAppName("com.openai.codex") == "Codex",
          approvalEvent(for: nil) == humanApprovalRequiredEvent,
          approvalEvent(for: .approved) == nil,
          approvalEvent(for: .denied) == nil,
          autoApprovalToolName(request) == "aws",
          scanAlertLevel([["severity": "medium"]]) == .medium,
          scanAlertLevel([["severity": "medium"], ["severity": "high"]]) == .high,
          autoApprovalTitle(
              AutoApprovalRecord(
                  accessRequestID: UUID(),
                  date: Date(timeIntervalSince1970: 18_900),
                  launcher: "Codex",
                  launcherIconPath: "/Applications/Codex.app",
                  tool: "aws",
                  command: "aws s3 ls",
                  keys: ["AWS_SECRET_ACCESS_KEY"],
                  wasDenied: false
              ),
              formatter: formatter
          ) == "5:15 AM – Codex used aws",
          autoApprovalSubmenuCapacity(visibleHeight: 600) == 26,
          restoredApproval.accessRequestID == recordedApproval.id,
          restoredApproval.launcher == "Codex",
          restoredApproval.tool == "aws",
          restoredApproval.command == "aws s3 ls",
          restoredApproval.keys == ["AWS_SECRET_ACCESS_KEY"],
          let restoredDenial = autoApprovalRecord(AccessRequestRecord(
              id: recordedApproval.id,
              date: recordedApproval.date,
              tool: "gh",
              command: "gh auth token",
              decision: "Denied",
              approvalSource: "Manual",
              reason: "Denied in prompt",
              launcher: recordedApproval.launcher,
              callerPath: recordedApproval.callerPath,
              target: recordedApproval.target,
              cwd: recordedApproval.cwd,
              keys: recordedApproval.keys,
              detail: recordedApproval.detail
          )),
          restoredDenial.wasDenied,
          autoApprovalTitle(restoredDenial, formatter: formatter) == "5:15 AM – Codex was denied use of gh",
          autoApprovalCommand(request) == """
          aws \\
            s3 \\
            ls
          """,
          autoApprovalToastFrame(
              anchor: NSRect(x: 760, y: 600, width: 24, height: 24),
              visibleFrame: NSRect(x: 0, y: 0, width: 800, height: 600),
              size: NSSize(width: 360, height: 120)
          ) == NSRect(x: 432, y: 476, width: 360, height: 120)
    else {
        return 1
    }
    return 0
}

if CommandLine.arguments.contains("--self-check-approvals") {
    exit(MainActor.assumeIsolated { runApprovalSelfCheck() })
}

if CommandLine.arguments.contains("--self-check-gh-read-only") {
    exit(runGhReadOnlySelfCheck())
}

if CommandLine.arguments.contains("--self-check-aws-read-only") {
    exit(runAwsReadOnlySelfCheck())
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
