import AppKit
import UserNotifications

final class MenuBarAppDelegate: NSObject, NSApplicationDelegate, NSMenuDelegate, UNUserNotificationCenterDelegate {
    private let bridge = NucleusBridge(compatibilityPolicy: .protocolOnly)
    private let homebrewUpdateChecker = HomebrewUpdateChecker()
    private let statusStore = NucleusStatusStore()
    private let hazardEffect = MenuBarHazardEffect()
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
    private let menu = NSMenu()
    private let refreshedItem = NSMenuItem(title: "Last Refresh: --", action: nil, keyEquivalent: "")
    private let errorItem = NSMenuItem(title: "", action: nil, keyEquivalent: "")
    private let startAtLoginItem = NSMenuItem(
        title: "Start at Login",
        action: #selector(toggleStartAtLogin(_:)),
        keyEquivalent: ""
    )
    private var refreshTimer: Timer?
    private var refreshObserver: NSObjectProtocol?
    private var startAtLoginObserver: NSObjectProtocol?
    private var appUpdateObserver: NSObjectProtocol?
    private var autoApprovedSecretObserver: NSObjectProtocol?
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
        refreshSnapshot(reason: "launch")
        startRefreshTimer()
        vaultDaemon.start()
    }

    func applicationWillTerminate(_ notification: Notification) {
        refreshTimer?.invalidate()
        if let refreshObserver {
            DistributedNotificationCenter.default().removeObserver(refreshObserver)
        }
        if let startAtLoginObserver {
            DistributedNotificationCenter.default().removeObserver(startAtLoginObserver)
        }
        if let appUpdateObserver {
            DistributedNotificationCenter.default().removeObserver(appUpdateObserver)
        }
        if let autoApprovedSecretObserver {
            DistributedNotificationCenter.default().removeObserver(autoApprovedSecretObserver)
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
        content.title = "Automic Vault Approval Needed"
        content.body = "A command is waiting for approval."
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
            withTitle: "Refresh",
            action: #selector(refreshFromMenu(_:)),
            keyEquivalent: "r"
        ).target = self
        menu.addItem(refreshedItem)
        menu.addItem(.separator())

        menu.addItem(errorItem)
        menu.addItem(
            withTitle: "Open Main Window",
            action: #selector(openMainWindow(_:)),
            keyEquivalent: ""
        ).target = self
        menu.addItem(.separator())
        startAtLoginItem.target = self
        menu.addItem(startAtLoginItem)
        menu.addItem(.separator())
        menu.addItem(
            withTitle: "Quit",
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
        alert.messageText = "Could Not Update Login Item"
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

    private func installRefreshObserverIfNeeded() {
        guard refreshObserver == nil else { return }
        refreshObserver = statusStore.observeRefreshRequests { [weak self] _ in
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
        guard autoApprovedSecretObserver == nil else { return }
        autoApprovedSecretObserver = DistributedNotificationCenter.default().addObserver(
            forName: IsotopeNotification.automaticApprovalGranted,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.showAutomaticSecretApprovalNotification()
        }
    }

    private func showAutomaticSecretApprovalNotification() {
        guard let button = statusItem.button else { return }
        automaticSecretApprovalToast.show(
            message: "Secret auto-approved",
            anchoredTo: button
        )
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
                let homebrewOutdatedPackages: [OutdatedPackageRecord]
                let lastError: NucleusStatusSnapshot.ErrorSnapshot?
                do {
                    homebrewOutdatedPackages = try self.homebrewUpdateChecker
                        .refreshOutdatedPackagesSync()
                    lastError = nil
                } catch {
                    homebrewOutdatedPackages = []
                    lastError = .init(
                        message: "Homebrew refresh failed during \(reason): " +
                            error.localizedDescription,
                        refreshedAt: Date()
                    )
                }
                nextSnapshot = NucleusStatusSnapshot(
                    installedCount: installedPackages.count,
                    hazardousPackageCount: installedPackages.filter {
                        $0.fallbackDetail.securityNotice != nil
                    }.count,
                    outdatedPackages: outdatedPackages,
                    homebrewOutdatedPackages: homebrewOutdatedPackages,
                    refreshedAt: Date(),
                    lastError: lastError,
                    remoteDatabaseRefreshState: previous.remoteDatabaseRefreshState
                )
            } catch {
                nextSnapshot = NucleusStatusSnapshot(
                    installedCount: previous.installedCount,
                    hazardousPackageCount: previous.hazardousPackageCount,
                    outdatedPackages: previous.outdatedPackages,
                    homebrewOutdatedPackages: [],
                    refreshedAt: previous.refreshedAt,
                    lastError: .init(
                        message: "Refresh failed during \(reason): \(error.localizedDescription)",
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
        refreshedItem.title = "Last Refresh: \(refreshStatusText(for: snapshot))"
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
                    name: "Installed",
                    detail: "\(snapshot.installedCount) packages"
                )
            ]
        }

        let nucleusItems = snapshot.outdatedPackages.map { package in
            packageStatusItem(
                name: package.name,
                detail: "\(package.currentVersion) -> \(package.latestVersion)"
            )
        }
        let homebrewItems = snapshot.homebrewOutdatedPackages.map { package in
            packageStatusItem(
                name: package.name,
                detail: "\(package.currentVersion) -> \(package.latestVersion)",
                source: "Homebrew"
            )
        }
        return nucleusItems + homebrewItems
    }

    private func packageStatusItem(
        name: String,
        detail: String,
        source: String? = nil
    ) -> NSMenuItem {
        let item = NSMenuItem(title: "", action: nil, keyEquivalent: "")
        item.view = PackageStatusMenuItemView(name: name, detail: detail, source: source)
        return item
    }

    private func applyButtonAppearance(outdatedCount: Int, hazardousCount: Int) {
        guard let button = statusItem.button else { return }
        let shouldShowIndicator = outdatedCount > 0 || appUpdateSnapshot.updateAvailable

        if let statusIcon {
            button.image = adjustedIcon(statusIcon, horizontalInset: 0, heightReduction: 0, offsetY: 0)
            button.imagePosition = shouldShowIndicator ? .imageLeading : .imageOnly
            button.imageScaling = .scaleNone
            if shouldShowIndicator {
                button.attributedTitle = NSAttributedString(
                    string: "●",
                    attributes: [
                        .font: NSFont.systemFont(ofSize: 9, weight: .bold),
                        .foregroundColor: NSColor.systemRed,
                        .baselineOffset: 5
                    ]
                )
            } else {
                button.attributedTitle = NSAttributedString(string: "")
                button.title = ""
            }
        } else {
            let atom = NSAttributedString(
                string: "⚛︎",
                attributes: [
                    .font: NSFont.systemFont(ofSize: 18, weight: .medium),
                    .foregroundColor: NSColor.labelColor
                ]
            )
            let title = NSMutableAttributedString(attributedString: atom)
            if shouldShowIndicator {
                title.append(
                    NSAttributedString(
                        string: " ●",
                        attributes: [
                            .font: NSFont.systemFont(ofSize: 10, weight: .bold),
                            .foregroundColor: NSColor.systemRed,
                            .baselineOffset: 6
                        ]
                    )
                )
            }
            button.image = nil
            button.attributedTitle = title
        }
        hazardEffect.install(in: button)
        hazardEffect.layout(in: button.bounds)
        hazardEffect.update(isActive: hazardousCount > 0)
        button.toolTip = buttonTooltip(
            outdatedCount: outdatedCount,
            hazardousCount: hazardousCount
        )
    }

    private func buttonTooltip(outdatedCount: Int, hazardousCount: Int) -> String {
        var parts: [String] = []
        if hazardousCount > 0 {
            parts.append("\(hazardousCount) hazardous packages")
        }
        if outdatedCount > 0 {
            parts.append("\(outdatedCount) outdated packages")
        }
        if appUpdateSnapshot.updateAvailable {
            parts.append("app update available")
        }
        return parts.isEmpty ? "Automic Vault" : parts.joined(separator: ", ")
    }

    private func refreshStatusText(for snapshot: NucleusStatusSnapshot) -> String {
        switch snapshot.remoteDatabaseRefreshState {
        case .normal:
            return formattedRefresh(snapshot.refreshedAt)
        case .pendingHelperInstallation:
            return "pending helper installation"
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
            return "never"
        }
        let elapsedSeconds = max(0, Int(Date().timeIntervalSince(date)))
        if elapsedSeconds < 60 {
            return "just now"
        }

        let elapsedMinutes = elapsedSeconds / 60
        if elapsedMinutes < 60 {
            return elapsedMinutes == 1 ? "1 min. ago" : "\(elapsedMinutes) min. ago"
        }

        let elapsedHours = elapsedMinutes / 60
        if elapsedHours < 24 {
            return elapsedHours == 1 ? "1 hr. ago" : "\(elapsedHours) hr. ago"
        }

        let elapsedDays = elapsedHours / 24
        return elapsedDays == 1 ? "1 day ago" : "\(elapsedDays) days ago"
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
        static let minimumWidth: CGFloat = 268
        static let maximumWidth: CGFloat = 360
        static let height: CGFloat = 24
        static let leadingInset: CGFloat = 24
        static let trailingInset: CGFloat = 16
        static let gap: CGFloat = 8
        static let minimumSourceWidth: CGFloat = 68
        static let maximumDetailWidth: CGFloat = 130
    }

    private let sourceLabel = NSTextField(labelWithString: "")
    private let nameLabel = NSTextField(labelWithString: "")
    private let detailLabel = NSTextField(labelWithString: "")
    private let rowWidth: CGFloat

    init(name: String, detail: String, source: String?) {
        rowWidth = Self.preferredWidth(name: name, detail: detail, source: source)
        super.init(frame: NSRect(x: 0, y: 0, width: rowWidth, height: Metrics.height))

        configureLabel(nameLabel)
        configureLabel(detailLabel)

        if let source {
            configureLabel(sourceLabel)
            sourceLabel.stringValue = source
            sourceLabel.font = .systemFont(ofSize: 10, weight: .medium)
            sourceLabel.textColor = .secondaryLabelColor
            sourceLabel.alignment = .left
            addSubview(sourceLabel)
        }

        nameLabel.stringValue = name
        nameLabel.font = .systemFont(ofSize: 12, weight: .semibold)
        nameLabel.textColor = .disabledControlTextColor
        nameLabel.lineBreakMode = .byTruncatingMiddle
        nameLabel.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)

        detailLabel.stringValue = detail
        detailLabel.font = .monospacedDigitSystemFont(ofSize: 11, weight: .regular)
        detailLabel.textColor = .secondaryLabelColor
        detailLabel.alignment = .right
        detailLabel.lineBreakMode = .byTruncatingHead
        detailLabel.setContentHuggingPriority(.defaultHigh, for: .horizontal)
        detailLabel.setContentCompressionResistancePriority(.defaultHigh, for: .horizontal)

        [nameLabel, detailLabel].forEach(addSubview)
        activateConstraints(source: source)
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

    private func activateConstraints(source: String?) {
        var constraints: [NSLayoutConstraint] = [
            detailLabel.trailingAnchor.constraint(
                equalTo: trailingAnchor,
                constant: -Metrics.trailingInset
            ),
            detailLabel.widthAnchor.constraint(lessThanOrEqualToConstant: Metrics.maximumDetailWidth),
            detailLabel.centerYAnchor.constraint(equalTo: centerYAnchor),

            nameLabel.centerYAnchor.constraint(equalTo: centerYAnchor),
            nameLabel.trailingAnchor.constraint(
                lessThanOrEqualTo: detailLabel.leadingAnchor,
                constant: -Metrics.gap
            ),
        ]

        if source != nil {
            constraints += [
                sourceLabel.leadingAnchor.constraint(
                    equalTo: leadingAnchor,
                    constant: Metrics.leadingInset
                ),
                sourceLabel.widthAnchor.constraint(
                    equalToConstant: Self.sourceWidth(source)
                ),
                sourceLabel.centerYAnchor.constraint(equalTo: centerYAnchor),
                nameLabel.leadingAnchor.constraint(
                    equalTo: sourceLabel.trailingAnchor,
                    constant: Metrics.gap
                ),
            ]
        } else {
            constraints.append(
                nameLabel.leadingAnchor.constraint(
                    equalTo: leadingAnchor,
                    constant: Metrics.leadingInset
                )
            )
        }

        NSLayoutConstraint.activate(constraints)
    }

    private static func preferredWidth(name: String, detail: String, source: String?) -> CGFloat {
        let detailWidth = min(
            measuredWidth(detail, font: .monospacedDigitSystemFont(ofSize: 11, weight: .regular)),
            Metrics.maximumDetailWidth
        )
        var width = Metrics.leadingInset
            + Metrics.trailingInset
            + measuredWidth(name, font: .systemFont(ofSize: 12, weight: .semibold))
            + Metrics.gap
            + detailWidth

        if let source {
            width += sourceWidth(source) + Metrics.gap
        }

        return min(max(ceil(width), Metrics.minimumWidth), Metrics.maximumWidth)
    }

    private static func sourceWidth(_ source: String?) -> CGFloat {
        guard let source else { return 0 }
        return max(
            Metrics.minimumSourceWidth,
            ceil(measuredWidth(source, font: .systemFont(ofSize: 10, weight: .medium)))
        )
    }

    private static func measuredWidth(_ string: String, font: NSFont) -> CGFloat {
        (string as NSString).size(withAttributes: [.font: font]).width
    }
}

private final class MenuBarInlineNotification {
    private let panel: NSPanel
    private let label = NSTextField(labelWithString: "")
    private var hideWorkItem: DispatchWorkItem?

    init() {
        panel = NSPanel(
            contentRect: NSRect(x: 0, y: 0, width: 178, height: 24),
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
        container.translatesAutoresizingMaskIntoConstraints = false
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
        let width: CGFloat = 178
        var frame = NSRect(
            x: anchorFrame.midX - width / 2,
            y: anchorFrame.minY - height - 5,
            width: width,
            height: height
        )

        if let screenFrame = buttonWindow.screen?.visibleFrame ?? NSScreen.main?.visibleFrame {
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
}
