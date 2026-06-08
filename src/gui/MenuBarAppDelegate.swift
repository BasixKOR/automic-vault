import AppKit
import QuartzCore
import UserNotifications

final class MenuBarAppDelegate: NSObject, NSApplicationDelegate, NSMenuDelegate, UNUserNotificationCenterDelegate {
    private static let remoteDatabaseRefreshInterval: TimeInterval = 60 * 60
    private static let neutralMenuBarIndicatorColor = NSColor(
        name: NSColor.Name("NeutralMenuBarIndicatorColor")
    ) { appearance in
        appearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
            ? .white
            : .black
    }
    private enum MenuBarIndicatorMetrics {
        static let diameter: CGFloat = 7
        static let imageOverlap: CGFloat = 1.5
        static let edgeInset: CGFloat = 1
    }

    private let bridge = NucleusBridge(
        compatibilityPolicy: .protocolOnly,
        daemonOwnership: .owner
    )
    private let helperBridge = NukeHelperBridge()
    private let statusStore = NucleusStatusStore()
    private let automaticSecretApprovalToast = MenuBarInlineNotification()
    private lazy var vaultDaemon = VaultDaemon(
        openMainWindow: { [weak self] in
            self?.openMainWindow(nil)
        },
        notifyUser: { [weak self] in
            self?.postApprovalNotification()
        }
    )
    private let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
    private let statusIcon = Bundle.main.image(forResource: "NSMenuItem")
    private let menuBarIndicatorLayer = CALayer()
    private let menu = NSMenu()
    private let refreshedItem = NSMenuItem(
        title: L10n.string("Last Refresh: --"),
        action: nil,
        keyEquivalent: ""
    )
    private let errorItem = NSMenuItem(title: "", action: nil, keyEquivalent: "")
    private let startAtLoginItem = NSMenuItem(
        title: L10n.string("Start at Login"),
        action: #selector(toggleStartAtLogin(_:)),
        keyEquivalent: ""
    )
    private var refreshTimer: Timer?
    private var remoteDatabaseRefreshTimer: Timer?
    private var refreshObserver: NSObjectProtocol?
    private var startAtLoginObserver: NSObjectProtocol?
    private var appUpdateObserver: NSObjectProtocol?
    private var autoApprovedSecretObserverInstalled = false
    private var autoRejectedDotenvObserverInstalled = false
    private var refreshInFlight = false
    private var snapshot = NucleusStatusSnapshot.empty
    private var appUpdateSnapshot = AppUpdateSnapshot.empty
    private var packageStatusItems: [NSMenuItem] = []

    func applicationDidFinishLaunching(_ notification: Notification) {
        guard terminateIfDuplicateInstanceExists() == false else { return }
        configureNotifications()
        configureMenu()
        snapshot = statusStore.loadSnapshot()
        appUpdateSnapshot = statusStore.loadAppUpdateSnapshot()
        apply(snapshot: snapshot)
        installRefreshObserverIfNeeded()
        installStartAtLoginObserverIfNeeded()
        installAppUpdateObserverIfNeeded()
        installAutoApprovedSecretObserverIfNeeded()
        installAutoRejectedDotenvObserverIfNeeded()
        startRemoteDatabaseRefreshTimer()
        refreshSnapshot(reason: "launch")
        startRefreshTimer()
        vaultDaemon.start()
    }

    func applicationWillTerminate(_ notification: Notification) {
        refreshTimer?.invalidate()
        remoteDatabaseRefreshTimer?.invalidate()
        if let refreshObserver {
            DistributedNotificationCenter.default().removeObserver(refreshObserver)
        }
        if let startAtLoginObserver {
            DistributedNotificationCenter.default().removeObserver(startAtLoginObserver)
        }
        if let appUpdateObserver {
            DistributedNotificationCenter.default().removeObserver(appUpdateObserver)
        }
        if autoApprovedSecretObserverInstalled {
            DistributedNotificationCenter.default().removeObserver(
                self,
                name: IsotopeNotification.automaticApprovalGranted,
                object: nil
            )
        }
        if autoRejectedDotenvObserverInstalled {
            DistributedNotificationCenter.default().removeObserver(
                self,
                name: DotenvNotification.automaticExportRejected,
                object: nil
            )
        }
        vaultDaemon.stop()
        bridge.invalidate()
    }

    func menuWillOpen(_ menu: NSMenu) {
        updateStartAtLoginState()
        if Date().timeIntervalSince(snapshot.refreshedAt) > 60 {
            refreshSnapshot(reason: "menu-open")
        }
    }

    @objc private func openMainWindow(_ sender: Any?) {
        statusStore.requestOpenMainWindow()
        if let app = NSRunningApplication.runningApplications(
            withBundleIdentifier: "com.automicvault"
        ).first {
            app.activate(options: [])
            return
        }

        guard let mainAppURL = mainApplicationURL() else { return }
        let configuration = NSWorkspace.OpenConfiguration()
        configuration.activates = true
        NSWorkspace.shared.openApplication(
            at: mainAppURL,
            configuration: configuration
        ) { _, _ in
        }
    }

    @objc private func quitFromMenu(_ sender: Any?) {
        NSRunningApplication.runningApplications(withBundleIdentifier: "com.automicvault")
            .forEach { $0.terminate() }
        NSApp.terminate(nil)
    }

    @objc private func refreshFromMenu(_ sender: Any?) {
        refreshSnapshot(reason: "manual")
    }

    @objc private func toggleStartAtLogin(_ sender: Any?) {
        startAtLoginItem.isEnabled = false
        if mainApplicationIsRunning() {
            statusStore.requestStartAtLoginToggle()
        } else {
            openMainApplicationForStartAtLoginToggle()
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 1) { [weak self] in
            self?.updateStartAtLoginState()
        }
    }

    private func configureNotifications() {
        let center = UNUserNotificationCenter.current()
        center.delegate = self
        center.requestAuthorization(options: [.alert, .sound]) { _, _ in
        }
    }

    private func postApprovalNotification() {
        let content = UNMutableNotificationContent()
        content.title = L10n.string("Automic Vault Approval Needed")
        content.body = L10n.string("A command is waiting for approval.")
        content.sound = .default
        let request = UNNotificationRequest(
            identifier: "com.automicvault.vault.approval",
            content: content,
            trigger: nil
        )
        UNUserNotificationCenter.current().add(request)
    }

    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse,
        withCompletionHandler completionHandler: @escaping () -> Void
    ) {
        openMainWindow(nil)
        completionHandler()
    }

    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification,
        withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
    ) {
        completionHandler([.banner, .sound])
    }

    private func configureMenu() {
        refreshedItem.isEnabled = false
        errorItem.isEnabled = false

        menu.delegate = self
        rebuildMenuItems()
        statusItem.menu = menu
        updateStartAtLoginState()
        applyButtonAppearance(
            outdatedCount: snapshot.flaggedOutdatedPackageCount,
            hazardousCount: snapshot.hazardousPackageCount
        )
    }

    private func rebuildMenuItems() {
        menu.removeAllItems()
        packageStatusItems = packageItems(for: snapshot)
        packageStatusItems.forEach(menu.addItem)

        menu.addItem(.separator())
        menu.addItem(
            withTitle: L10n.string("Refresh"),
            action: #selector(refreshFromMenu(_:)),
            keyEquivalent: "r"
        ).target = self
        menu.addItem(refreshedItem)
        menu.addItem(.separator())

        menu.addItem(errorItem)
        menu.addItem(
            withTitle: L10n.string("Open Main Window"),
            action: #selector(openMainWindow(_:)),
            keyEquivalent: ""
        ).target = self
        menu.addItem(.separator())
        startAtLoginItem.target = self
        menu.addItem(startAtLoginItem)
        menu.addItem(.separator())
        menu.addItem(
            withTitle: L10n.string("Quit"),
            action: #selector(quitFromMenu(_:)),
            keyEquivalent: "q"
        ).target = self
    }

    private func updateStartAtLoginState() {
        let snapshot = statusStore.loadStartAtLoginSnapshot()
        startAtLoginItem.state = snapshot.status == .enabled ? .on : .off
        startAtLoginItem.isEnabled = snapshot.status != .notFound
    }

    private func presentStartAtLoginError(_ error: Error) {
        let alert = NSAlert()
        alert.messageText = L10n.string("Could Not Update Login Item")
        alert.informativeText = error.localizedDescription
        alert.alertStyle = .warning
        alert.runModal()
    }

    private func startRefreshTimer() {
        refreshTimer?.invalidate()
        refreshTimer = Timer.scheduledTimer(
            withTimeInterval: 15 * 60,
            repeats: true
        ) { [weak self] _ in
            self?.refreshSnapshot(reason: "timer")
        }
        if let refreshTimer {
            RunLoop.main.add(refreshTimer, forMode: .common)
        }
    }

    private func startRemoteDatabaseRefreshTimer() {
        remoteDatabaseRefreshTimer?.invalidate()
        refreshRemoteDatabase()
        remoteDatabaseRefreshTimer = Timer.scheduledTimer(
            withTimeInterval: Self.remoteDatabaseRefreshInterval,
            repeats: true
        ) { [weak self] _ in
            self?.refreshRemoteDatabase()
        }
        if let remoteDatabaseRefreshTimer {
            RunLoop.main.add(remoteDatabaseRefreshTimer, forMode: .common)
        }
    }

    private func refreshRemoteDatabase() {
        helperBridge.refreshRemoteDatabase { [weak self] result in
            guard let self else { return }
            switch result {
            case .success(.completed(let updated)):
                try? self.statusStore.saveRemoteDatabaseRefreshState(.normal)
                guard updated else { return }
                self.bridge.invalidate()
                self.refreshSnapshot(reason: "remote database")
            case .success(.pendingHelperInstallation):
                try? self.statusStore.saveRemoteDatabaseRefreshState(.pendingHelperInstallation)
            case .failure(let error):
                NSLog("remote database refresh failed: %@", error.localizedDescription)
            }
        }
    }

    private func installRefreshObserverIfNeeded() {
        guard refreshObserver == nil else { return }
        refreshObserver = statusStore.observeRefreshRequests { [weak self] _ in
            self?.bridge.invalidate()
            self?.refreshSnapshot(reason: "requested")
        }
    }

    private func installStartAtLoginObserverIfNeeded() {
        guard startAtLoginObserver == nil else { return }
        startAtLoginObserver = statusStore.observeStartAtLoginChanges { [weak self] _ in
            self?.updateStartAtLoginState()
        }
    }

    private func installAppUpdateObserverIfNeeded() {
        guard appUpdateObserver == nil else { return }
        appUpdateObserver = statusStore.observeAppUpdateChanges { [weak self] _ in
            guard let self else { return }
            self.appUpdateSnapshot = self.statusStore.loadAppUpdateSnapshot()
            self.applyButtonAppearance(
                outdatedCount: self.snapshot.flaggedOutdatedPackageCount,
                hazardousCount: self.snapshot.hazardousPackageCount
            )
        }
    }

    private func installAutoApprovedSecretObserverIfNeeded() {
        guard autoApprovedSecretObserverInstalled == false else { return }
        DistributedNotificationCenter.default().addObserver(
            self,
            selector: #selector(autoApprovedSecretNotification(_:)),
            name: IsotopeNotification.automaticApprovalGranted,
            object: nil,
            suspensionBehavior: .deliverImmediately
        )
        autoApprovedSecretObserverInstalled = true
    }

    private func installAutoRejectedDotenvObserverIfNeeded() {
        guard autoRejectedDotenvObserverInstalled == false else { return }
        DistributedNotificationCenter.default().addObserver(
            self,
            selector: #selector(autoRejectedDotenvNotification(_:)),
            name: DotenvNotification.automaticExportRejected,
            object: nil,
            suspensionBehavior: .deliverImmediately
        )
        autoRejectedDotenvObserverInstalled = true
    }

    @objc private func autoApprovedSecretNotification(_ notification: Notification) {
        showAutomaticSecretApprovalNotification(secretNames: notification.object as? String)
    }

    @objc private func autoRejectedDotenvNotification(_ notification: Notification) {
        showAutomaticDotenvRejectionNotification(sourceName: notification.object as? String)
    }

    private func showAutomaticSecretApprovalNotification(secretNames: String?) {
        guard let button = statusItem.button else { return }
        automaticSecretApprovalToast.show(
            message: automaticSecretApprovalMessage(secretNames: secretNames),
            anchoredTo: button
        )
    }

    private func showAutomaticDotenvRejectionNotification(sourceName: String?) {
        guard let button = statusItem.button else { return }
        automaticSecretApprovalToast.show(
            message: automaticDotenvRejectionMessage(sourceName: sourceName),
            anchoredTo: button
        )
    }

    private func automaticSecretApprovalMessage(secretNames: String?) -> String {
        let names = secretNames?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        guard names.isEmpty == false else {
            return L10n.string("Secret auto-approved")
        }
        return names.contains(",")
            ? L10n.format("Secrets auto-approved: %@", names)
            : L10n.format("Secret auto-approved: %@", names)
    }

    private func automaticDotenvRejectionMessage(sourceName: String?) -> String {
        let name = sourceName?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        guard name.isEmpty == false else {
            return L10n.string("Dotenv export auto-rejected")
        }
        return L10n.format("Dotenv export auto-rejected: %@", name)
    }

    private func mainApplicationIsRunning() -> Bool {
        NSRunningApplication.runningApplications(
            withBundleIdentifier: "com.automicvault"
        ).isEmpty == false
    }

    private func terminateIfDuplicateInstanceExists() -> Bool {
        let currentProcessIdentifier = ProcessInfo.processInfo.processIdentifier
        let otherInstance = NSRunningApplication.runningApplications(
            withBundleIdentifier: "com.automicvault.menu-helper"
        ).contains { application in
            application.processIdentifier != currentProcessIdentifier
        }
        if otherInstance {
            NSApp.terminate(nil)
        }
        return otherInstance
    }

    private func openMainApplicationForStartAtLoginToggle() {
        guard let mainAppURL = mainApplicationURL() else {
            startAtLoginItem.isEnabled = true
            return
        }
        let configuration = NSWorkspace.OpenConfiguration()
        configuration.activates = false
        configuration.arguments = ["--toggle-start-at-login"]
        NSWorkspace.shared.openApplication(
            at: mainAppURL,
            configuration: configuration
        ) { [weak self] _, error in
            if let error {
                self?.presentStartAtLoginError(error)
                self?.updateStartAtLoginState()
            }
        }
    }

    private func refreshSnapshot(reason: String) {
        guard refreshInFlight == false else { return }
        refreshInFlight = true

        DispatchQueue.global(qos: .utility).async {
            let previous = self.statusStore.loadSnapshot()
            let nextSnapshot: NucleusStatusSnapshot
            do {
                let installedPackages = try self.bridge.fetchPackages()
                let outdatedPackages = try self.bridge.fetchOutdatedPackages()
                    .sorted(by: { $0.name < $1.name })
                nextSnapshot = NucleusStatusSnapshot(
                    installedCount: installedPackages.count,
                    hazardousPackageCount: Self.securityAlertCount(
                        installedPackages: installedPackages,
                        geigerAlertCount: self.geigerAlertCount()
                    ),
                    outdatedPackages: outdatedPackages,
                    refreshedAt: Date(),
                    lastError: nil,
                    remoteDatabaseRefreshState: previous.remoteDatabaseRefreshState
                )
            } catch {
                nextSnapshot = NucleusStatusSnapshot(
                    installedCount: previous.installedCount,
                    hazardousPackageCount: previous.hazardousPackageCount,
                    outdatedPackages: previous.outdatedPackages,
                    refreshedAt: previous.refreshedAt,
                    lastError: .init(
                        message: L10n.format(
                            "Refresh failed during %@: %@",
                            L10n.string(reason),
                            error.localizedDescription
                        ),
                        refreshedAt: Date()
                    ),
                    remoteDatabaseRefreshState: previous.remoteDatabaseRefreshState
                )
            }

            try? self.statusStore.saveSnapshot(nextSnapshot)
            DispatchQueue.main.async {
                self.refreshInFlight = false
                self.apply(snapshot: nextSnapshot)
            }
        }
    }

    private func apply(snapshot: NucleusStatusSnapshot) {
        self.snapshot = snapshot
        refreshedItem.title = L10n.format("Last Refresh: %@", refreshStatusText(for: snapshot))
        if let error = snapshot.lastError {
            errorItem.isHidden = false
            errorItem.title = error.message
        } else {
            errorItem.isHidden = true
            errorItem.title = ""
        }
        rebuildMenuItems()
        updateStartAtLoginState()
        applyButtonAppearance(
            outdatedCount: snapshot.flaggedOutdatedPackageCount,
            hazardousCount: snapshot.hazardousPackageCount
        )
    }

    private func packageItems(for snapshot: NucleusStatusSnapshot) -> [NSMenuItem] {
        guard snapshot.flaggedOutdatedPackageCount > 0 else {
            return [
                packageStatusItem(
                    name: L10n.string("Installed"),
                    detail: snapshot.installedCount == 1
                        ? L10n.string("1 package")
                        : L10n.format("%d packages", snapshot.installedCount)
                )
            ]
        }

        return snapshot.outdatedPackages.map { package in
            packageStatusItem(
                name: package.name,
                detail: "\(package.currentVersion) → \(package.latestVersion)"
            )
        }
    }

    private func packageStatusItem(
        name: String,
        detail: String
    ) -> NSMenuItem {
        let item = NSMenuItem(title: "", action: nil, keyEquivalent: "")
        item.view = PackageStatusMenuItemView(name: name, detail: detail)
        return item
    }

    private func applyButtonAppearance(outdatedCount: Int, hazardousCount: Int) {
        guard let button = statusItem.button else { return }
        let hasSecurityAlerts = hazardousCount > 0
        let shouldShowIndicator = hasSecurityAlerts
            || outdatedCount > 0
            || appUpdateSnapshot.updateAvailable
        let indicatorColor = Self.menuBarIndicatorColor(hasSecurityAlerts: hasSecurityAlerts)

        if let statusIcon {
            button.image = adjustedIcon(statusIcon, horizontalInset: 0, heightReduction: 0, offsetY: 0)
            button.imagePosition = .imageOnly
            button.imageScaling = .scaleNone
            button.attributedTitle = NSAttributedString(string: "")
            button.title = ""
        } else {
            let atom = NSAttributedString(
                string: "⚛︎",
                attributes: [
                    .font: NSFont.systemFont(ofSize: 18, weight: .medium),
                    .foregroundColor: NSColor.labelColor
                ]
            )
            button.image = nil
            button.attributedTitle = atom
        }
        updateMenuBarIndicator(
            on: button,
            isVisible: shouldShowIndicator,
            color: indicatorColor
        )
        button.toolTip = buttonTooltip(
            outdatedCount: outdatedCount,
            hazardousCount: hazardousCount
        )
    }

    private func updateMenuBarIndicator(
        on button: NSStatusBarButton,
        isVisible: Bool,
        color: NSColor
    ) {
        button.wantsLayer = true
        guard let buttonLayer = button.layer else { return }
        if menuBarIndicatorLayer.superlayer !== buttonLayer {
            menuBarIndicatorLayer.removeFromSuperlayer()
            buttonLayer.addSublayer(menuBarIndicatorLayer)
        }

        CATransaction.begin()
        CATransaction.setDisableActions(true)
        var indicatorColor = color.cgColor
        button.effectiveAppearance.performAsCurrentDrawingAppearance {
            indicatorColor = color.cgColor
        }
        menuBarIndicatorLayer.backgroundColor = indicatorColor
        menuBarIndicatorLayer.cornerRadius = MenuBarIndicatorMetrics.diameter / 2
        menuBarIndicatorLayer.contentsScale = button.window?.backingScaleFactor
            ?? NSScreen.main?.backingScaleFactor
            ?? 2
        menuBarIndicatorLayer.frame = menuBarIndicatorFrame(in: button)
        menuBarIndicatorLayer.isHidden = isVisible == false
        CATransaction.commit()

        DispatchQueue.main.async { [weak self, weak button] in
            guard let self, let button else { return }
            self.positionMenuBarIndicator(in: button)
        }
    }

    private func positionMenuBarIndicator(in button: NSStatusBarButton) {
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        menuBarIndicatorLayer.frame = menuBarIndicatorFrame(in: button)
        CATransaction.commit()
    }

    private func menuBarIndicatorFrame(in button: NSStatusBarButton) -> CGRect {
        let diameter = MenuBarIndicatorMetrics.diameter
        let bounds = button.bounds
        guard bounds.isEmpty == false else {
            return CGRect(x: 0, y: 0, width: diameter, height: diameter)
        }

        let imageRect = button.cell?.imageRect(forBounds: bounds) ?? .zero
        let referenceRect = imageRect.isEmpty
            ? bounds.insetBy(dx: max((bounds.width - 18) / 2, 0), dy: max((bounds.height - 18) / 2, 0))
            : imageRect
        let proposedX = referenceRect.maxX - diameter + MenuBarIndicatorMetrics.imageOverlap
        let proposedY = referenceRect.maxY - diameter + MenuBarIndicatorMetrics.imageOverlap
        let maxX = max(bounds.minX + MenuBarIndicatorMetrics.edgeInset, bounds.maxX - diameter - MenuBarIndicatorMetrics.edgeInset)
        let maxY = max(bounds.minY + MenuBarIndicatorMetrics.edgeInset, bounds.maxY - diameter - MenuBarIndicatorMetrics.edgeInset)
        let x = min(max(proposedX, bounds.minX + MenuBarIndicatorMetrics.edgeInset), maxX)
        let y = min(max(proposedY, bounds.minY + MenuBarIndicatorMetrics.edgeInset), maxY)
        return CGRect(x: floor(x), y: floor(y), width: diameter, height: diameter)
    }

    private static func menuBarIndicatorColor(hasSecurityAlerts: Bool) -> NSColor {
        hasSecurityAlerts ? .systemRed : neutralMenuBarIndicatorColor
    }

    private static func securityAlertCount(
        installedPackages: [PackageRecord],
        geigerAlertCount: Int
    ) -> Int {
        max(
            installedPackages.filter(\.hasMainWindowSecurityAlert).count,
            geigerAlertCount
        )
    }

    private func geigerAlertCount() -> Int {
        (try? bridge.fetchGeigerPackages(offset: 0, limit: 1).totalCount) ?? 0
    }

    private func buttonTooltip(outdatedCount: Int, hazardousCount: Int) -> String {
        var parts: [String] = []
        if hazardousCount > 0 {
            parts.append(
                hazardousCount == 1
                    ? L10n.string("1 hazardous package")
                    : L10n.format("%d hazardous packages", hazardousCount)
            )
        }
        if outdatedCount > 0 {
            parts.append(
                outdatedCount == 1
                    ? L10n.string("1 outdated package")
                    : L10n.format("%d outdated packages", outdatedCount)
            )
        }
        if appUpdateSnapshot.updateAvailable {
            parts.append(L10n.string("app update available"))
        }
        return parts.isEmpty ? "Automic Vault" : parts.joined(separator: ", ")
    }

    private func refreshStatusText(for snapshot: NucleusStatusSnapshot) -> String {
        switch snapshot.remoteDatabaseRefreshState {
        case .normal:
            return formattedRefresh(snapshot.refreshedAt)
        case .pendingHelperInstallation:
            return L10n.string("pending helper installation")
        }
    }

    private func adjustedIcon(
        _ image: NSImage,
        horizontalInset: CGFloat,
        heightReduction: CGFloat,
        offsetY: CGFloat
    ) -> NSImage {
        let height = max(image.size.height - heightReduction, 1)
        let scale = height / max(image.size.height, 1)
        let scaledWidth = image.size.width * scale
        let width = max(scaledWidth - (horizontalInset * 2), 1)
        let adjusted = NSImage(size: NSSize(width: width, height: height))
        adjusted.isTemplate = true
        adjusted.lockFocus()
        image.draw(
            in: NSRect(
                x: -horizontalInset,
                y: offsetY,
                width: scaledWidth,
                height: height
            ),
            from: NSRect(origin: .zero, size: image.size),
            operation: .sourceOver,
            fraction: 1
        )
        adjusted.unlockFocus()
        return adjusted
    }

    private func formattedRefresh(_ date: Date) -> String {
        guard date != .distantPast else {
            return L10n.string("never")
        }
        let elapsedSeconds = max(0, Int(Date().timeIntervalSince(date)))
        if elapsedSeconds < 60 {
            return L10n.string("just now")
        }

        let elapsedMinutes = elapsedSeconds / 60
        if elapsedMinutes < 60 {
            return elapsedMinutes == 1
                ? L10n.string("1 min. ago")
                : L10n.format("%d min. ago", elapsedMinutes)
        }

        let elapsedHours = elapsedMinutes / 60
        if elapsedHours < 24 {
            return elapsedHours == 1
                ? L10n.string("1 hr. ago")
                : L10n.format("%d hr. ago", elapsedHours)
        }

        let elapsedDays = elapsedHours / 24
        return elapsedDays == 1
            ? L10n.string("1 day ago")
            : L10n.format("%d days ago", elapsedDays)
    }

    private func mainApplicationURL() -> URL? {
        let url = Bundle.main.bundleURL
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        guard FileManager.default.fileExists(atPath: url.path) else {
            return nil
        }
        return url
    }
}

private final class PackageStatusMenuItemView: NSView {
    private enum Metrics {
        static let minimumWidth: CGFloat = 190
        static let maximumWidth: CGFloat = 420
        static let height: CGFloat = 24
        static let leadingInset: CGFloat = 24
        static let trailingInset: CGFloat = 16
        static let gap: CGFloat = 12
        static let maximumDetailWidth: CGFloat = 130
        static let measuredTextPadding: CGFloat = 3
    }

    private let nameLabel = NSTextField(labelWithString: "")
    private let detailLabel = NSTextField(labelWithString: "")
    private let rowWidth: CGFloat

    init(name: String, detail: String) {
        rowWidth = Self.preferredWidth(name: name, detail: detail)
        super.init(frame: NSRect(x: 0, y: 0, width: rowWidth, height: Metrics.height))

        configureLabel(nameLabel)
        configureLabel(detailLabel)

        nameLabel.stringValue = name
        nameLabel.font = .systemFont(ofSize: 12, weight: .semibold)
        nameLabel.textColor = .disabledControlTextColor
        nameLabel.lineBreakMode = .byTruncatingMiddle
        nameLabel.setContentHuggingPriority(.defaultHigh, for: .horizontal)
        nameLabel.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)

        detailLabel.stringValue = detail
        detailLabel.font = .monospacedDigitSystemFont(ofSize: 11, weight: .regular)
        detailLabel.textColor = .secondaryLabelColor
        detailLabel.alignment = .left
        detailLabel.lineBreakMode = .byTruncatingHead
        detailLabel.setContentHuggingPriority(.defaultHigh, for: .horizontal)
        detailLabel.setContentCompressionResistancePriority(.defaultHigh, for: .horizontal)

        [nameLabel, detailLabel].forEach(addSubview)
        activateConstraints()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        nil
    }

    override var intrinsicContentSize: NSSize {
        NSSize(width: rowWidth, height: Metrics.height)
    }

    private func configureLabel(_ label: NSTextField) {
        label.translatesAutoresizingMaskIntoConstraints = false
        label.isBezeled = false
        label.drawsBackground = false
        label.isEditable = false
        label.isSelectable = false
        label.maximumNumberOfLines = 1
    }

    private func activateConstraints() {
        NSLayoutConstraint.activate([
            nameLabel.leadingAnchor.constraint(
                equalTo: leadingAnchor,
                constant: Metrics.leadingInset
            ),
            nameLabel.centerYAnchor.constraint(equalTo: centerYAnchor),

            detailLabel.leadingAnchor.constraint(
                equalTo: nameLabel.trailingAnchor,
                constant: Metrics.gap
            ),
            detailLabel.trailingAnchor.constraint(
                lessThanOrEqualTo: trailingAnchor,
                constant: -Metrics.trailingInset
            ),
            detailLabel.widthAnchor.constraint(lessThanOrEqualToConstant: Metrics.maximumDetailWidth),
            detailLabel.centerYAnchor.constraint(equalTo: centerYAnchor),
        ])
    }

    private static func preferredWidth(name: String, detail: String) -> CGFloat {
        let detailWidth = min(
            measuredWidth(detail, font: .monospacedDigitSystemFont(ofSize: 11, weight: .regular)),
            Metrics.maximumDetailWidth
        )
        let width = Metrics.leadingInset
            + Metrics.trailingInset
            + measuredWidth(name, font: .systemFont(ofSize: 12, weight: .semibold))
            + Metrics.gap
            + detailWidth

        return min(max(ceil(width), Metrics.minimumWidth), Metrics.maximumWidth)
    }

    private static func measuredWidth(_ string: String, font: NSFont) -> CGFloat {
        (string as NSString).size(withAttributes: [.font: font]).width
            + Metrics.measuredTextPadding
    }
}

private final class MenuBarInlineNotification {
    private let panel: NSPanel
    private let label = NSTextField(labelWithString: "")
    private var hideWorkItem: DispatchWorkItem?

    init() {
        panel = NSPanel(
            contentRect: NSRect(x: 0, y: 0, width: 220, height: 24),
            styleMask: [.borderless, .nonactivatingPanel],
            backing: .buffered,
            defer: false
        )
        panel.level = .statusBar
        panel.collectionBehavior = [.canJoinAllSpaces, .transient, .ignoresCycle]
        panel.backgroundColor = .clear
        panel.isOpaque = false
        panel.hasShadow = true
        panel.ignoresMouseEvents = true

        let container = NSVisualEffectView()
        container.frame = NSRect(x: 0, y: 0, width: 220, height: 24)
        container.autoresizingMask = [.width, .height]
        container.material = .popover
        container.blendingMode = .behindWindow
        container.state = .active
        container.wantsLayer = true
        container.layer?.cornerRadius = 6
        container.layer?.masksToBounds = true
        container.layer?.borderWidth = 1
        container.layer?.borderColor = NSColor.separatorColor.withAlphaComponent(0.45).cgColor

        label.translatesAutoresizingMaskIntoConstraints = false
        label.font = .systemFont(ofSize: 12, weight: .semibold)
        label.textColor = .labelColor
        label.alignment = .center
        label.lineBreakMode = .byTruncatingTail
        label.maximumNumberOfLines = 1

        container.addSubview(label)
        panel.contentView = container

        NSLayoutConstraint.activate([
            label.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 10),
            label.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -10),
            label.centerYAnchor.constraint(equalTo: container.centerYAnchor),
        ])
    }

    func show(message: String, anchoredTo button: NSStatusBarButton) {
        guard let buttonWindow = button.window else { return }

        hideWorkItem?.cancel()
        label.stringValue = message

        let anchorFrame = buttonWindow.convertToScreen(button.convert(button.bounds, to: nil))
        let height = max(22, min(28, NSStatusBar.system.thickness))
        let width = preferredWidth(for: message)
        let screenFrame = buttonWindow.screen?.visibleFrame ?? NSScreen.main?.visibleFrame
        let preferredY = screenFrame.map { $0.maxY - height - 8 }
            ?? anchorFrame.minY - height - 8
        var frame = NSRect(
            x: anchorFrame.midX - width / 2,
            y: min(anchorFrame.minY - height - 8, preferredY),
            width: width,
            height: height
        )

        if let screenFrame {
            frame.origin.x = min(
                max(frame.minX, screenFrame.minX + 4),
                screenFrame.maxX - width - 4
            )
            frame.origin.y = max(frame.minY, screenFrame.minY + 4)
        }

        panel.setFrame(frame, display: false)
        panel.alphaValue = 0
        panel.orderFrontRegardless()

        NSAnimationContext.runAnimationGroup { context in
            context.duration = 0.12
            panel.animator().alphaValue = 1
        }

        let workItem = DispatchWorkItem { [weak self] in
            self?.hide()
        }
        hideWorkItem = workItem
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.4, execute: workItem)
    }

    private func hide() {
        NSAnimationContext.runAnimationGroup { [panel] context in
            context.duration = 0.18
            panel.animator().alphaValue = 0
        } completionHandler: { [panel] in
            panel.orderOut(nil)
        }
    }

    private func preferredWidth(for message: String) -> CGFloat {
        let font = label.font ?? .systemFont(ofSize: 12, weight: .semibold)
        let measured = (message as NSString).size(withAttributes: [.font: font]).width
        return min(max(ceil(measured) + 24, 178), 340)
    }
}
