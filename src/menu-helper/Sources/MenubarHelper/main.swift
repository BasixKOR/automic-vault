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
private let scanQueue = DispatchQueue(label: "com.automicvault.av2.scan")
private var toastWindows: [NSWindow] = []

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private lazy var statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
    private lazy var scanStatusItem = NSMenuItem(title: "Scan pending", action: nil, keyEquivalent: "")
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
            let approval = try ApprovalServer(serviceName: approvalServiceName)
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
        case .clean(let detectorCount):
            statusItem.button?.image = menuImage()
            scanStatusItem.title = "Vulnerability Detectors: 🟢 (\(detectorCount)/\(detectorCount))"
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
    case clean(Int)
    case findings(Int)
    case failed
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
          let findings = object["findings"] as? [Any]
    else {
        return .failed
    }
    return findings.isEmpty
        ? .clean(loadDetectorMetadata(avExecutableURL: executableURL).count)
        : .findings(findings.count)
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
            throw AppError("missing menu bar signing team identifier")
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
        guard let listener else { throw AppError("approval XPC listener failed") }

        let requirement = """
        identifier "com.automicvault.av" and anchor apple generic and \
        certificate leaf[subject.OU] = \(teamIdentifier)
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
          parentlessVaulttyLauncher?.designatedRequirement == vaulttyAppSigning.designatedRequirement,
          vaulttyBridgeLauncher?.designatedRequirement == vaulttyAppSigning.designatedRequirement,
          launcherIdentity(pid: 45, path: pythonSigning.mainExecutable, signing: pythonSigning) == nil,
          launcherIdentity(pid: 46, path: "/usr/local/bin/av", signing: unbundledSigning) == nil,
          !alwaysAllows(approval, service: service)
    else {
        return 1
    }

    rememberAlwaysAllow(approval, service: service)
    guard alwaysAllows(approval, service: service),
          trustedLauncher(script: script, request: request, launchers: [wrapperLauncher, launcher], service: service)?.designatedRequirement == launcher.designatedRequirement,
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

private func runLaunchAgentHandoffSelfCheck() -> Int32 {
    guard !isLaunchAgentInstance(environment: [:]),
          isLaunchAgentInstance(environment: ["XPC_SERVICE_NAME": approvalLaunchAgentName])
    else {
        return 1
    }
    return 0
}

if CommandLine.arguments.contains("--self-check-approvals") {
    exit(runApprovalSelfCheck())
}

if CommandLine.arguments.contains("--self-check-dashboard-search") {
    exit(MainActor.assumeIsolated { runDashboardSearchSelfCheck() })
}

if CommandLine.arguments.contains("--self-check-launch-agent-handoff") {
    exit(runLaunchAgentHandoffSelfCheck())
}

let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.setActivationPolicy(.accessory)
app.run()
