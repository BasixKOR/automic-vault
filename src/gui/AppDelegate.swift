import AppKit
import ServiceManagement

final class AppDelegate: NSObject, NSApplicationDelegate {
    private static let toggleStartAtLoginArgument = "--toggle-start-at-login"
    private static let remoteDatabaseRefreshInterval: TimeInterval = 60 * 60
    private var window: NSWindow?
    private let statusStore = NucleusStatusStore()
    private let vaultApprovalStore = VaultApprovalStore()
    private let containmentLogStore = ContainmentLogStore()
    private let isotopeApprovalStore = IsotopeApprovalStore()
    private let dotenvApprovalStore = DotenvApprovalStore()
    private let gateApprovalStore = GateApprovalStore()
    private let helperBridge = NukeHelperBridge()
    #if !DEBUG
    private let postHogTelemetry = PostHogTelemetry.shared
    #endif
    private var openWindowObserver: NSObjectProtocol?
    private var startAtLoginObserver: NSObjectProtocol?
    private var containmentLogObserver: NSObjectProtocol?
    private var pendingApprovalObserver: NSObjectProtocol?
    private var pendingIsotopeApprovalObserver: NSObjectProtocol?
    private var pendingDotenvApprovalObserver: NSObjectProtocol?
    private var pendingGateApprovalObserver: NSObjectProtocol?
    private var activeApprovalID: String?
    private var activeIsotopeApprovalID: String?
    private var activeDotenvApprovalID: String?
    private var activeGateApprovalID: String?
    private var containmentWindowControllers: [String: ContainmentLogWindowController] = [:]
    private var remoteDatabaseRefreshTimer: Timer?

    func applicationDidFinishLaunching(_ notification: Notification) {
        if CommandLine.arguments.contains(Self.toggleStartAtLoginArgument) {
            toggleStartAtLoginFromHelper()
            NSApp.terminate(nil)
            return
        }

        NSApp.mainMenu = makeMainMenu()
        publishStartAtLoginStatus()
        launchMenuBarHelperIfNeeded()
        installOpenWindowObserverIfNeeded()
        installStartAtLoginObserverIfNeeded()
        installContainmentLogObserverIfNeeded()
        installVaultApprovalObserverIfNeeded()
        installIsotopeApprovalObserverIfNeeded()
        installDotenvApprovalObserverIfNeeded()
        installGateApprovalObserverIfNeeded()
        startRemoteDatabaseRefreshTimer()
        showMainWindow()
        presentPendingVaultApprovalIfNeeded()
        presentPendingIsotopeApprovalIfNeeded()
        presentPendingDotenvApprovalIfNeeded()
        presentPendingGateApprovalIfNeeded()
    }

    func applicationWillTerminate(_ notification: Notification) {
        if let openWindowObserver {
            DistributedNotificationCenter.default().removeObserver(openWindowObserver)
        }
        if let startAtLoginObserver {
            DistributedNotificationCenter.default().removeObserver(startAtLoginObserver)
        }
        if let containmentLogObserver {
            DistributedNotificationCenter.default().removeObserver(containmentLogObserver)
        }
        if let pendingApprovalObserver {
            DistributedNotificationCenter.default().removeObserver(pendingApprovalObserver)
        }
        if let pendingIsotopeApprovalObserver {
            DistributedNotificationCenter.default().removeObserver(pendingIsotopeApprovalObserver)
        }
        if let pendingDotenvApprovalObserver {
            DistributedNotificationCenter.default().removeObserver(pendingDotenvApprovalObserver)
        }
        if let pendingGateApprovalObserver {
            DistributedNotificationCenter.default().removeObserver(pendingGateApprovalObserver)
        }
        remoteDatabaseRefreshTimer?.invalidate()
        (window?.contentViewController as? RootViewController)?
            .applicationWillTerminate()
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }

    func applicationShouldHandleReopen(
        _ sender: NSApplication,
        hasVisibleWindows flag: Bool
    ) -> Bool {
        guard !flag else { return true }
        showMainWindow()
        return true
    }

    private func makeMainMenu() -> NSMenu {
        let menu = NSMenu(title: "Main Menu")
        menu.addItem(makeAppMenuItem())
        menu.addItem(makeEditMenuItem())
        menu.addItem(makeWindowMenuItem())
        return menu
    }

    private func makeAppMenuItem() -> NSMenuItem {
        let appItem = NSMenuItem()
        let appMenu = NSMenu(title: "Automic Vault")
        let appName = ProcessInfo.processInfo.processName

        appMenu.addItem(
            withTitle: "About \(appName)",
            action: #selector(NSApplication.orderFrontStandardAboutPanel(_:)),
            keyEquivalent: ""
        )
        appMenu.addItem(.separator())
        appMenu.addItem(
            withTitle: "Hide \(appName)",
            action: #selector(NSApplication.hide(_:)),
            keyEquivalent: "h"
        )
        let hideOthers = appMenu.addItem(
            withTitle: "Hide Others",
            action: #selector(NSApplication.hideOtherApplications(_:)),
            keyEquivalent: "h"
        )
        hideOthers.keyEquivalentModifierMask = [.command, .option]
        appMenu.addItem(
            withTitle: "Show All",
            action: #selector(NSApplication.unhideAllApplications(_:)),
            keyEquivalent: ""
        )
        appMenu.addItem(.separator())
        appMenu.addItem(
            withTitle: "Quit \(appName)",
            action: #selector(NSApplication.terminate(_:)),
            keyEquivalent: "q"
        )

        appItem.submenu = appMenu
        return appItem
    }

    private func makeEditMenuItem() -> NSMenuItem {
        let editItem = NSMenuItem()
        let editMenu = NSMenu(title: "Edit")

        editMenu.addItem(
            withTitle: "Undo",
            action: Selector(("undo:")),
            keyEquivalent: "z"
        )
        let redoItem = editMenu.addItem(
            withTitle: "Redo",
            action: Selector(("redo:")),
            keyEquivalent: "z"
        )
        redoItem.keyEquivalentModifierMask = [.command, .shift]
        editMenu.addItem(.separator())
        editMenu.addItem(
            withTitle: "Cut",
            action: #selector(NSText.cut(_:)),
            keyEquivalent: "x"
        )
        editMenu.addItem(
            withTitle: "Copy",
            action: #selector(NSText.copy(_:)),
            keyEquivalent: "c"
        )
        editMenu.addItem(
            withTitle: "Paste",
            action: #selector(NSText.paste(_:)),
            keyEquivalent: "v"
        )
        editMenu.addItem(
            withTitle: "Select All",
            action: #selector(NSText.selectAll(_:)),
            keyEquivalent: "a"
        )

        editItem.submenu = editMenu
        return editItem
    }

    private func makeWindowMenuItem() -> NSMenuItem {
        let windowItem = NSMenuItem()
        let windowMenu = NSMenu(title: "Window")

        let refreshItem = windowMenu.addItem(
            withTitle: "Refresh",
            action: #selector(refreshPackages(_:)),
            keyEquivalent: "r"
        )
        refreshItem.target = self
        #if DEBUG
        let fakeUpdateItem = windowMenu.addItem(
            withTitle: "Run Fake Update",
            action: #selector(runFakeUpdate(_:)),
            keyEquivalent: "u"
        )
        fakeUpdateItem.keyEquivalentModifierMask = [.command, .shift]
        fakeUpdateItem.target = self
        windowMenu.addItem(.separator())
        #endif
        windowMenu.addItem(
            withTitle: "Close",
            action: #selector(NSWindow.performClose(_:)),
            keyEquivalent: "w"
        )
        windowMenu.addItem(
            withTitle: "Minimize",
            action: #selector(NSWindow.performMiniaturize(_:)),
            keyEquivalent: "m"
        )
        windowMenu.addItem(
            withTitle: "Zoom",
            action: #selector(NSWindow.performZoom(_:)),
            keyEquivalent: ""
        )
        windowItem.submenu = windowMenu
        NSApp.windowsMenu = windowMenu
        return windowItem
    }

    private func startRemoteDatabaseRefreshTimer() {
        guard remoteDatabaseRefreshTimer == nil else { return }
        refreshRemoteDatabase()
        let timer = Timer(
            timeInterval: Self.remoteDatabaseRefreshInterval,
            repeats: true
        ) { [weak self] _ in
            self?.refreshRemoteDatabase()
        }
        remoteDatabaseRefreshTimer = timer
        RunLoop.main.add(timer, forMode: .common)
    }

    private func refreshRemoteDatabase() {
        helperBridge.refreshRemoteDatabase { result in
            switch result {
            case .success(.completed(_)):
                try? self.statusStore.saveRemoteDatabaseRefreshState(.normal)
            case .success(.pendingHelperInstallation):
                try? self.statusStore.saveRemoteDatabaseRefreshState(.pendingHelperInstallation)
            case .failure(let error):
                NSLog("remote database refresh failed: %@", error.localizedDescription)
            }
        }
    }

    private func showMainWindow() {
        let wasVisible = window?.isVisible ?? false
        let window = makeOrRestoreMainWindow()
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        #if !DEBUG
        if wasVisible == false {
            postHogTelemetry.captureMainWindowOpened()
        }
        #endif
    }

    private func installOpenWindowObserverIfNeeded() {
        guard openWindowObserver == nil else { return }
        openWindowObserver = statusStore.observeOpenMainWindowRequests { [weak self] _ in
            self?.showMainWindow()
        }
    }

    private func installStartAtLoginObserverIfNeeded() {
        guard startAtLoginObserver == nil else { return }
        startAtLoginObserver = statusStore.observeStartAtLoginToggleRequests { [weak self] _ in
            self?.toggleStartAtLoginFromHelper()
        }
    }

    private func installContainmentLogObserverIfNeeded() {
        guard containmentLogObserver == nil else { return }
        containmentLogObserver = containmentLogStore.observeChanges { [weak self] notification in
            guard let self else { return }
            guard let sessionID = notification.userInfo?[ContainmentLogNotification.sessionIDKey]
                as? String else {
                return
            }
            self.showContainmentWindow(sessionID: sessionID)
        }
    }

    private func installVaultApprovalObserverIfNeeded() {
        guard pendingApprovalObserver == nil else { return }
        pendingApprovalObserver = vaultApprovalStore.observePendingApprovalChanges { [weak self] _ in
            self?.presentPendingVaultApprovalIfNeeded()
        }
    }

    private func installIsotopeApprovalObserverIfNeeded() {
        guard pendingIsotopeApprovalObserver == nil else { return }
        pendingIsotopeApprovalObserver = isotopeApprovalStore.observePendingApprovalChanges { [weak self] _ in
            self?.presentPendingIsotopeApprovalIfNeeded()
        }
    }

    private func installDotenvApprovalObserverIfNeeded() {
        guard pendingDotenvApprovalObserver == nil else { return }
        pendingDotenvApprovalObserver = dotenvApprovalStore.observePendingApprovalChanges { [weak self] _ in
            self?.presentPendingDotenvApprovalIfNeeded()
        }
    }

    private func installGateApprovalObserverIfNeeded() {
        guard pendingGateApprovalObserver == nil else { return }
        pendingGateApprovalObserver = gateApprovalStore.observePendingApprovalChanges { [weak self] _ in
            self?.presentPendingGateApprovalIfNeeded()
        }
    }

    @objc private func refreshPackages(_ sender: Any?) {
        (window?.contentViewController as? RootViewController)?.requestRefresh()
    }

    #if DEBUG
    @objc private func runFakeUpdate(_ sender: Any?) {
        let window = makeOrRestoreMainWindow()
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        (window.contentViewController as? RootViewController)?.runDebugFakeUpdate()
    }
    #endif

    private func toggleStartAtLoginFromHelper() {
        let service = SMAppService.loginItem(identifier: "com.automicvault.menu-helper")

        do {
            if service.status == .enabled {
                try service.unregister()
            } else {
                try service.register()
            }
        } catch {
            publishStartAtLoginStatus(error: error.localizedDescription)
            return
        }

        if service.status == .requiresApproval {
            SMAppService.openSystemSettingsLoginItems()
        }
        publishStartAtLoginStatus()
    }

    private func publishStartAtLoginStatus(error: String? = nil) {
        let service = SMAppService.loginItem(identifier: "com.automicvault.menu-helper")
        let status: StartAtLoginSnapshot.Status
        switch service.status {
        case .enabled:
            status = .enabled
        case .requiresApproval:
            status = .requiresApproval
        case .notFound:
            status = .notFound
        case .notRegistered:
            status = .disabled
        @unknown default:
            status = .unavailable
        }

        try? statusStore.saveStartAtLoginSnapshot(
            StartAtLoginSnapshot(
                status: status,
                updatedAt: Date(),
                lastError: error
            )
        )
    }

    private func presentPendingVaultApprovalIfNeeded() {
        guard let approval = vaultApprovalStore.loadPendingApproval() else {
            activeApprovalID = nil
            return
        }
        guard activeApprovalID != approval.id else { return }
        activeApprovalID = approval.id

        let window = showContainmentWindow(sessionID: approval.intent.agentID)
            ?? makeOrRestoreMainWindow()
        if NSApp.isActive == false || window.isKeyWindow == false {
            _ = NSApp.requestUserAttention(.criticalRequest)
        }
        NSApp.activate(ignoringOtherApps: true)

        let alert = NSAlert()
        alert.messageText = "Approve Command Execution"
        alert.informativeText = ""
        alert.alertStyle = .warning
        alert.addButton(withTitle: "Approve")
        alert.addButton(withTitle: "Deny")
        alert.accessoryView = approvalAccessoryView(for: approval)
        alert.beginSheetModal(for: window) { [weak self] response in
            guard let self else { return }
            try? self.vaultApprovalStore.saveDecision(
                VaultApprovalDecision(
                    id: approval.id,
                    approved: response == .alertFirstButtonReturn,
                    reason: response == .alertFirstButtonReturn ? nil : "Denied by operator"
                )
            )
            self.activeApprovalID = nil
            DispatchQueue.main.async {
                self.presentPendingVaultApprovalIfNeeded()
            }
        }
    }

    private func presentPendingIsotopeApprovalIfNeeded() {
        guard let approval = isotopeApprovalStore.loadPendingApproval() else {
            activeIsotopeApprovalID = nil
            return
        }
        guard activeIsotopeApprovalID != approval.id else { return }
        activeIsotopeApprovalID = approval.id

        let window = makeOrRestoreMainWindow()
        if NSApp.isActive == false || window.isKeyWindow == false {
            _ = NSApp.requestUserAttention(.criticalRequest)
        }
        showMainWindow()

        let alert = NSAlert()
        alert.messageText = "Approve Key Injection"
        alert.informativeText = isotopeApprovalSummary(for: approval)
        alert.alertStyle = .warning
        alert.addButton(withTitle: "Allow")
        alert.addButton(withTitle: "Deny")
        if approval.canAlwaysAllow {
            alert.addButton(withTitle: "Always Allow")
        }
        alert.accessoryView = isotopeApprovalAccessoryView(for: approval)
        alert.beginSheetModal(for: window) { [weak self] response in
            guard let self else { return }
            if response == .alertThirdButtonReturn, approval.canAlwaysAllow {
                self.rememberIsotopeAlwaysAllow(approval)
                return
            }
            self.saveIsotopeDecision(
                approval: approval,
                approved: response == .alertFirstButtonReturn,
                alwaysAllow: false,
                reason: response == .alertFirstButtonReturn ? nil : "Denied by operator"
            )
        }
    }

    private func presentPendingDotenvApprovalIfNeeded() {
        guard let approval = dotenvApprovalStore.loadPendingApproval() else {
            activeDotenvApprovalID = nil
            return
        }
        guard activeDotenvApprovalID != approval.id else { return }
        activeDotenvApprovalID = approval.id

        let window = makeOrRestoreMainWindow()
        if NSApp.isActive == false || window.isKeyWindow == false {
            _ = NSApp.requestUserAttention(.criticalRequest)
        }
        showMainWindow()

        let alert = NSAlert()
        alert.messageText = "Approve Secret Access"
        alert.informativeText = "Application code requested \(approval.secret)."
        alert.alertStyle = .warning
        alert.addButton(withTitle: "Allow Once")
        alert.addButton(withTitle: "Deny")
        alert.addButton(withTitle: "Always Allow")
        alert.accessoryView = dotenvApprovalAccessoryView(for: approval)
        alert.beginSheetModal(for: window) { [weak self] response in
            guard let self else { return }
            try? self.dotenvApprovalStore.saveDecision(
                DotenvApprovalDecision(
                    id: approval.id,
                    approved: response == .alertFirstButtonReturn || response == .alertThirdButtonReturn,
                    alwaysAllow: response == .alertThirdButtonReturn,
                    reason: response == .alertSecondButtonReturn ? "Denied by operator" : nil
                )
            )
            self.activeDotenvApprovalID = nil
            DispatchQueue.main.async {
                self.presentPendingDotenvApprovalIfNeeded()
            }
        }
    }

    private func presentPendingGateApprovalIfNeeded() {
        guard let approval = gateApprovalStore.loadPendingApproval() else {
            activeGateApprovalID = nil
            return
        }
        guard activeGateApprovalID != approval.id else { return }
        activeGateApprovalID = approval.id

        let window = makeOrRestoreMainWindow()
        if NSApp.isActive == false || window.isKeyWindow == false {
            _ = NSApp.requestUserAttention(.criticalRequest)
        }
        showMainWindow()

        let alert = NSAlert()
        alert.messageText = "Approve Gate"
        alert.informativeText = approval.message
        alert.alertStyle = .warning
        alert.addButton(withTitle: "Approve")
        alert.addButton(withTitle: "Deny")
        alert.accessoryView = gateApprovalAccessoryView(for: approval)
        alert.beginSheetModal(for: window) { [weak self] response in
            guard let self else { return }
            try? self.gateApprovalStore.saveDecision(
                GateApprovalDecision(
                    id: approval.id,
                    approved: response == .alertFirstButtonReturn,
                    reason: response == .alertFirstButtonReturn ? nil : "Denied by operator"
                )
            )
            self.activeGateApprovalID = nil
            DispatchQueue.main.async {
                self.presentPendingGateApprovalIfNeeded()
            }
        }
    }

    private func rememberIsotopeAlwaysAllow(_ approval: IsotopeApprovalRequestSnapshot) {
        helperBridge.rememberIsotopeAlwaysAllow(
            executablePath: approval.executablePath,
            scriptPath: approval.scriptPath,
            scriptSha256: approval.scriptSha256,
            keys: approval.keys
        ) { [weak self] result in
            guard let self else { return }
            switch result {
            case .success:
                try? self.statusStore.saveRemoteDatabaseRefreshState(.normal)
                self.saveIsotopeDecision(
                    approval: approval,
                    approved: true,
                    alwaysAllow: true,
                    reason: nil
                )
            case .failure(let error):
                self.presentIsotopeAlwaysAllowError(error)
                self.activeIsotopeApprovalID = nil
                DispatchQueue.main.async {
                    self.presentPendingIsotopeApprovalIfNeeded()
                }
            }
        }
    }

    private func saveIsotopeDecision(
        approval: IsotopeApprovalRequestSnapshot,
        approved: Bool,
        alwaysAllow: Bool,
        reason: String?
    ) {
        try? isotopeApprovalStore.saveDecision(
            IsotopeApprovalDecision(
                id: approval.id,
                approved: approved,
                alwaysAllow: alwaysAllow,
                reason: reason
            )
        )
        activeIsotopeApprovalID = nil
        DispatchQueue.main.async {
            self.presentPendingIsotopeApprovalIfNeeded()
        }
    }

    private func presentIsotopeAlwaysAllowError(_ error: Error) {
        let alert = NSAlert()
        alert.messageText = "Could Not Remember Approval"
        alert.informativeText = error.localizedDescription
        alert.alertStyle = .warning
        alert.runModal()
    }

    private func isotopeApprovalSummary(for approval: IsotopeApprovalRequestSnapshot) -> String {
        ""
    }

    private func isotopeApprovalAccessoryView(for approval: IsotopeApprovalRequestSnapshot) -> NSView {
        IsotopeApprovalView(approval: approval)
    }

    private func dotenvApprovalAccessoryView(for approval: DotenvApprovalRequestSnapshot) -> NSView {
        let scrollView = NSScrollView(frame: NSRect(x: 0, y: 0, width: 640, height: 260))
        scrollView.hasVerticalScroller = true
        scrollView.borderType = .bezelBorder

        let textView = NSTextView(frame: scrollView.bounds)
        textView.isEditable = false
        textView.isRichText = false
        textView.font = UIStyle.monoFont(size: 11, weight: .regular)
        textView.string = dotenvApprovalDetailText(for: approval)
        scrollView.documentView = textView
        return scrollView
    }

    private func dotenvApprovalDetailText(for approval: DotenvApprovalRequestSnapshot) -> String {
        [
            "Secret",
            approval.secret,
            "",
            "Runtime",
            "\(approval.runtime) / \(approval.mode)",
            "",
            "Process",
            "pid \(approval.pid)",
            approval.executablePath ?? "unknown executable",
            "",
            "Project",
            approval.projectRoot,
            "",
            "Working Directory",
            approval.cwd,
            "",
            "Fingerprint",
            approval.fingerprint,
            "",
            "Backtrace",
            approval.normalizedBacktrace.isEmpty
                ? "<none>"
                : approval.normalizedBacktrace.joined(separator: "\n")
        ].joined(separator: "\n")
    }

    private func isotopeParentProcessSummary(
        _ parentProcess: IsotopeParentProcessSnapshot
    ) -> String {
        let name = parentProcess.displayName
            ?? parentProcess.executablePath
            ?? "unknown"
        return "\(name) (pid \(parentProcess.pid))"
    }

    private func isotopeParentProcessDetail(
        _ parentProcess: IsotopeParentProcessSnapshot
    ) -> String {
        let executable = parentProcess.executablePath ?? "unknown"
        let name = parentProcess.displayName ?? "unknown"
        return [
            "PID: \(parentProcess.pid)",
            "Name: \(name)",
            "Executable: \(executable)"
        ].joined(separator: "\n")
    }

    private func gateApprovalAccessoryView(for approval: GateApprovalRequestSnapshot) -> NSView {
        let scrollView = NSScrollView(frame: NSRect(x: 0, y: 0, width: 560, height: 180))
        scrollView.hasVerticalScroller = true
        scrollView.borderType = .bezelBorder

        let textView = NSTextView(frame: scrollView.bounds)
        textView.isEditable = false
        textView.isRichText = false
        textView.font = UIStyle.monoFont(size: 11, weight: .regular)
        textView.string = gateApprovalDetailText(for: approval)
        scrollView.documentView = textView
        return scrollView
    }

    private func gateApprovalDetailText(for approval: GateApprovalRequestSnapshot) -> String {
        [
            "Message",
            approval.message,
            "",
            "Working Directory",
            approval.cwd,
            "",
            "Invoked By",
            isotopeParentProcessDetail(approval.parentProcess)
        ].joined(separator: "\n")
    }

    private func approvalAccessoryView(for approval: VaultApprovalRequestSnapshot) -> NSView {
        CommandExecutionApprovalView(approval: approval)
    }

    private func launchMenuBarHelperIfNeeded() {
        guard let helperURL = embeddedMenuBarHelperURL() else { return }
        let helperBundleIdentifier = "com.automicvault.menu-helper"

        if NSRunningApplication.runningApplications(
            withBundleIdentifier: helperBundleIdentifier
        ).isEmpty == false {
            return
        }

        let configuration = NSWorkspace.OpenConfiguration()
        configuration.activates = false
        NSWorkspace.shared.openApplication(
            at: helperURL,
            configuration: configuration
        ) { _, _ in
        }
    }

    private func embeddedMenuBarHelperURL() -> URL? {
        let url = Bundle.main.bundleURL
            .appendingPathComponent("Contents/Library/LoginItems", isDirectory: true)
            .appendingPathComponent("Automic Vault Menu.app", isDirectory: true)
        guard FileManager.default.fileExists(atPath: url.path) else {
            return nil
        }
        return url
    }

    @discardableResult
    private func showContainmentWindow(sessionID: String?) -> NSWindow? {
        guard let sessionID, sessionID.isEmpty == false else {
            return nil
        }
        guard let snapshot = containmentLogStore.load(sessionID: sessionID) else {
            return nil
        }

        let controller: ContainmentLogWindowController
        if let existing = containmentWindowControllers[sessionID] {
            controller = existing
            controller.apply(snapshot: snapshot)
        } else {
            controller = ContainmentLogWindowController(snapshot: snapshot)
            containmentWindowControllers[sessionID] = controller
        }

        controller.showWindow(nil)
        controller.window?.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        return controller.window
    }

    private func makeOrRestoreMainWindow() -> NSWindow {
        if let window {
            return window
        }

        let controller = RootViewController()
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 1380, height: 860),
            styleMask: [
                .titled,
                .closable,
                .miniaturizable,
                .resizable,
                .fullSizeContentView
            ],
            backing: .buffered,
            defer: false
        )
        window.center()
        window.title = "Automic Vault"
        window.backgroundColor = UIStyle.background
        window.isOpaque = true
        window.isReleasedWhenClosed = false
        window.titleVisibility = .hidden
        window.titlebarAppearsTransparent = true
        window.isMovableByWindowBackground = true
        window.contentViewController = controller
        window.makeFirstResponder(controller.view)
        self.window = window
        return window
    }
}
