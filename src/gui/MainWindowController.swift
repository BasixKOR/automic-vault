import AppKit
import Combine
import SwiftUI

@MainActor
final class MainWindowController: NSHostingController<MainWindowView> {
    private static let searchShortcutKeys: Set<String> = ["k", "l", "p"]
    private static let appUpdateToolbarHorizontalPadding: CGFloat = 18
    private static let appUpdateToolbarIconTitleSpacing = ""
    private static let appUpdateToolbarTrailingPadding = "  "

    private let model: MainWindowModel
    private let appUpdateCoordinator: AppUpdateCoordinator
    private let helperBridge = NukeHelperBridge()
    private var didStartModel = false
    private var searchShortcutMonitor: Any?
    private weak var mainToolbar: NSToolbar?
    private weak var searchToolbarItem: NSSearchToolbarItem?
    private weak var appUpdateToolbarItem: NSToolbarItem?
    private weak var automicVaultCLTInstallToolbarItem: NSToolbarItem?
    private var updateAllRequestCancellable: AnyCancellable?
    private var packageOperationRequestCancellable: AnyCancellable?
    private var searchTextCancellable: AnyCancellable?
    private var searchDeactivationRequestCancellable: AnyCancellable?
    private var cltInstallToolbarStateCancellable: AnyCancellable?
    private var updateProgressViewController: UpdateProgressViewController?

    init(appUpdateCoordinator: AppUpdateCoordinator) {
        let model = MainWindowModel()
        self.model = model
        self.appUpdateCoordinator = appUpdateCoordinator
        super.init(rootView: MainWindowView(model: model))
        installUpdateAllRequestObserver()
        installPackageOperationRequestObserver()
        installSearchTextObserver()
        installSearchDeactivationObserver()
        installAutomicVaultCLTInstallToolbarObserver()
        installAppUpdateCallbacks()
    }

    @MainActor @preconcurrency required dynamic init?(coder: NSCoder) {
        let model = MainWindowModel()
        self.model = model
        self.appUpdateCoordinator = AppUpdateCoordinator(statusStore: NucleusStatusStore())
        super.init(coder: coder, rootView: MainWindowView(model: model))
        installUpdateAllRequestObserver()
        installPackageOperationRequestObserver()
        installSearchTextObserver()
        installSearchDeactivationObserver()
        installAutomicVaultCLTInstallToolbarObserver()
        installAppUpdateCallbacks()
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        startModelIfNeeded()
    }

    override func viewWillAppear() {
        super.viewWillAppear()
        startModelIfNeeded()
    }

    override func viewDidAppear() {
        super.viewDidAppear()
        installToolbarIfNeeded()
        installSearchShortcutMonitorIfNeeded()
    }

    override func viewDidDisappear() {
        super.viewDidDisappear()
        removeSearchShortcutMonitor()
    }

    func requestRefresh() {
        startModelIfNeeded()
        model.reloadPackages()
    }

    @objc private func refreshToolbarItemPressed(_ sender: Any?) {
        requestRefresh()
    }

    @objc private func appUpdateToolbarItemPressed(_ sender: Any?) {
        guard appUpdateCoordinator.hasAvailableUpdate else {
            appUpdateCoordinator.checkForUpdates()
            return
        }
        confirmAndInstallAppUpdate()
    }

    @objc private func automicVaultCLTInstallToolbarItemPressed(_ sender: Any?) {
        model.requestAutomicVaultCLTInstall()
    }

    func requestSearchFocus() {
        startModelIfNeeded()
        model.requestSearchFocus()
        searchToolbarItem?.beginSearchInteraction()
    }

    @objc private func searchToolbarItemChanged(_ sender: NSSearchField) {
        updateSearchText(sender.stringValue)
    }

    #if DEBUG
    func runDebugFakeUpdate() {
        startUpdateAll(debugPlayback: true)
    }
    #endif

    func applicationWillTerminate() {
        updateAllRequestCancellable?.cancel()
        packageOperationRequestCancellable?.cancel()
        searchTextCancellable?.cancel()
        searchDeactivationRequestCancellable?.cancel()
        cltInstallToolbarStateCancellable?.cancel()
        model.stop()
    }

    private func installAppUpdateCallbacks() {
        appUpdateCoordinator.onStateChange = { [weak self] in
            self?.syncAppUpdateToolbarItem()
        }
        appUpdateCoordinator.onError = { [weak self] message in
            self?.presentAppUpdateError(message)
        }
    }

    private func installUpdateAllRequestObserver() {
        updateAllRequestCancellable = model.$updateAllRequestID
            .dropFirst()
            .sink { [weak self] _ in
                Task { @MainActor in
                    self?.startUpdateAll(debugPlayback: false)
                }
            }
    }

    private func installPackageOperationRequestObserver() {
        packageOperationRequestCancellable = model.$packageOperationRequest
            .compactMap { $0 }
            .sink { [weak self] request in
                Task { @MainActor in
                    self?.startPackageOperation(request)
                }
            }
    }

    private func installSearchTextObserver() {
        searchTextCancellable = model.$searchText
            .removeDuplicates()
            .sink { [weak self] text in
                Task { @MainActor in
                    self?.syncSearchFieldText(text)
                }
            }
    }

    private func syncSearchFieldText(_ text: String) {
        guard let searchField = searchToolbarItem?.searchField else {
            return
        }
        if searchField.stringValue != text {
            searchField.stringValue = text
        }
        if let editor = searchField.currentEditor(), editor.string != text {
            editor.string = text
        }
    }

    private func installSearchDeactivationObserver() {
        searchDeactivationRequestCancellable = model.$searchDeactivationRequestID
            .dropFirst()
            .sink { [weak self] _ in
                Task { @MainActor in
                    self?.deactivateSearchField()
                }
            }
    }

    private func installAutomicVaultCLTInstallToolbarObserver() {
        let recommendationChanges = model.$automicVaultCLTRecommendation.map { _ in () }
        let operationChanges = model.$activePackageOperation.map { _ in () }
        let updateAllChanges = model.$isUpdatingAll.map { _ in () }
        cltInstallToolbarStateCancellable = Publishers.Merge3(
            recommendationChanges,
            operationChanges,
            updateAllChanges
        )
        .sink { [weak self] _ in
            Task { @MainActor in
                self?.syncAutomicVaultCLTInstallToolbarItem()
            }
        }
    }

    private func deactivateSearchField() {
        guard let searchField = searchToolbarItem?.searchField else {
            return
        }
        syncSearchFieldText(model.searchText)
        searchField.abortEditing()
        searchField.window?.makeFirstResponder(nil)
    }

    private func startUpdateAll(debugPlayback: Bool) {
        guard !model.isUpdatingAll else {
            return
        }

        #if DEBUG
        let packageNames = debugPlayback
            ? NukeHelperBridge.debugFakeUpdatePackages
            : model.outdatedUpdatePackageNames
        #else
        let packageNames = model.outdatedUpdatePackageNames
        #endif
        let updatesAutomicVaultCLT =
            !debugPlayback && model.shouldUpdateAutomicVaultCLTWithUpdateAll

        guard !packageNames.isEmpty else {
            model.showTransientStatus("No outdated packages to update")
            return
        }

        let progressController = presentUpdateProgressController()
        configure(
            progressController,
            packageCount: packageNames.count,
            debugPlayback: debugPlayback
        )
        model.beginOutdatedUpdateAll(packageCount: packageNames.count)
        progressController.begin(
            packages: packageNames,
            activationLog: activationLog(
                packageCount: packageNames.count,
                debugPlayback: debugPlayback
            ),
            initialOperation: debugPlayback
                ? "Playing debug update stream"
                : "Awaiting helper authorization"
        )

        var stagedCLTDirectory: URL?
        let handleProgress: (NukeHelperProgressEvent) -> Void = { [weak progressController] event in
            progressController?.handle(event: event)
        }
        let handleCompletion: (Result<NukeHelperResult, Error>) -> Void = {
            [weak self, weak progressController] result in
            guard let self else { return }
            if let stagedCLTDirectory {
                try? FileManager.default.removeItem(at: stagedCLTDirectory)
            }
            switch result {
            case .success(let helperResult):
                let completedPackages = helperResult.processedPackages.isEmpty
                    ? packageNames
                    : helperResult.processedPackages
                progressController?.succeed(
                    message: helperResult.message,
                    packages: completedPackages
                )
            case .failure(let error):
                progressController?.fail(message: error.localizedDescription)
            }
            self.model.finishOutdatedUpdateAll(
                result,
                refreshAfterSuccess: !debugPlayback
            )
        }

        if updatesAutomicVaultCLT {
            do {
                stagedCLTDirectory = try NucleusBridge().exportBundledCLTForHelperInstall()
            } catch {
                handleCompletion(.failure(error))
                return
            }
        }

        #if DEBUG
        if debugPlayback {
            helperBridge.debugFakeUpdate(
                progress: handleProgress,
                completion: handleCompletion
            )
            return
        }
        #endif

        helperBridge.updateAll(
            progress: handleProgress,
            completion: { [weak self] result in
                guard let self,
                      updatesAutomicVaultCLT else {
                    handleCompletion(result)
                    return
                }
                self.finishUpdateAll(
                    result,
                    byInstallingAutomicVaultCLTFrom: stagedCLTDirectory,
                    progress: handleProgress,
                    completion: handleCompletion
                )
            }
        )
    }

    private func finishUpdateAll(
        _ updateAllResult: Result<NukeHelperResult, Error>,
        byInstallingAutomicVaultCLTFrom stagedCLTDirectory: URL?,
        progress: @escaping (NukeHelperProgressEvent) -> Void,
        completion: @escaping (Result<NukeHelperResult, Error>) -> Void
    ) {
        switch updateAllResult {
        case .failure:
            completion(updateAllResult)
        case .success(let updateAllSuccess):
            guard let stagedCLTDirectory else {
                completion(.failure(NukeHelperBridgeError.operationFailed(
                    "Bundled av command line tool was not staged for installation."
                )))
                return
            }
            helperBridge.installAv(
                sourcePath: stagedCLTDirectory.path,
                progress: progress
            ) { avResult in
                switch avResult {
                case .failure:
                    completion(avResult)
                case .success(let avSuccess):
                    completion(.success(NukeHelperResult(
                        message: updateAllSuccess.processedPackages.isEmpty
                            ? avSuccess.message
                            : "Update complete",
                        processedPackages: Self.mergedProcessedPackages(
                            updateAllSuccess.processedPackages,
                            avSuccess.processedPackages
                        )
                    )))
                }
            }
        }
    }

    private static func mergedProcessedPackages(
        _ left: [String],
        _ right: [String]
    ) -> [String] {
        var seen = Set<String>()
        var result: [String] = []
        for package in left + right {
            guard seen.insert(package).inserted else {
                continue
            }
            result.append(package)
        }
        return result
    }

    private func startPackageOperation(_ request: PackageOperationRequest) {
        guard !model.isPackageMutationInFlight else {
            model.showTransientStatus("Package operation already in progress")
            return
        }

        if request.isXcodeCLT, request.kind == .install {
            startXcodeCommandLineToolsInstall(request)
            return
        }

        let progressController = presentUpdateProgressController()
        configure(progressController, request: request)
        model.beginPackageOperation(request)
        progressController.begin(
            packages: request.packageNames,
            activationLog: packageOperationActivationLog(request),
            initialOperation: "Awaiting helper authorization"
        )

        var stagedCLTDirectory: URL?
        let handleProgress: (NukeHelperProgressEvent) -> Void = { [weak progressController] event in
            progressController?.handle(event: event)
        }
        let handleCompletion: (Result<NukeHelperResult, Error>) -> Void = {
            [weak self, weak progressController] result in
            guard let self else { return }
            if let stagedCLTDirectory {
                try? FileManager.default.removeItem(at: stagedCLTDirectory)
            }
            switch result {
            case .success(let helperResult):
                let completedPackages = helperResult.processedPackages.isEmpty
                    ? request.packageNames
                    : helperResult.processedPackages
                progressController?.succeed(
                    message: helperResult.message,
                    packages: completedPackages
                )
            case .failure(let error):
                progressController?.fail(message: error.localizedDescription)
            }
            self.model.finishPackageOperation(
                request,
                result,
                refreshAfterSuccess: true
            )
        }

        if request.isAutomicVaultCLT,
           request.kind == .install || request.kind == .update {
            do {
                stagedCLTDirectory = try NucleusBridge().exportBundledCLTForHelperInstall()
                helperBridge.installAv(
                    sourcePath: stagedCLTDirectory?.path ?? "",
                    progress: handleProgress,
                    completion: handleCompletion
                )
            } catch {
                handleCompletion(.failure(error))
            }
            return
        }

        let packageSpecs = request.packageNames.map { AVPackageSpec(name: $0) }
        switch request.kind {
        case .install:
            helperBridge.install(
                packages: packageSpecs,
                progress: handleProgress,
                completion: handleCompletion
            )
        case .update:
            helperBridge.update(
                packages: packageSpecs,
                progress: handleProgress,
                completion: handleCompletion
            )
        case .uninstall:
            helperBridge.uninstall(
                packages: packageSpecs,
                progress: handleProgress,
                completion: handleCompletion
            )
        }
    }

    private func startXcodeCommandLineToolsInstall(_ request: PackageOperationRequest) {
        model.beginPackageOperation(request)
        do {
            let process = Process()
            process.executableURL = URL(fileURLWithPath: "/usr/bin/xcode-select")
            process.arguments = ["--install"]
            try process.run()
            model.finishPackageOperation(
                request,
                .success(NukeHelperResult(
                    message: "Command Line Tools installer launched",
                    processedPackages: request.packageNames
                )),
                refreshAfterSuccess: false
            )
        } catch {
            model.finishPackageOperation(
                request,
                .failure(error),
                refreshAfterSuccess: false
            )
        }
    }

    private func presentUpdateProgressController() -> UpdateProgressViewController {
        if let updateProgressViewController {
            return updateProgressViewController
        }

        let controller = UpdateProgressViewController()
        controller.preferredContentSize = NSSize(width: 820, height: 700)
        controller.onRetry = { [weak self] in
            self?.startUpdateAll(debugPlayback: false)
        }
        controller.onDismiss = { [weak self] in
            self?.dismissUpdateProgressController()
        }
        presentAsSheet(controller)
        updateProgressViewController = controller
        return controller
    }

    private func dismissUpdateProgressController() {
        guard let controller = updateProgressViewController else {
            return
        }
        dismiss(controller)
        updateProgressViewController = nil
    }

    private func configure(
        _ progressController: UpdateProgressViewController,
        packageCount: Int,
        debugPlayback: Bool
    ) {
        progressController.onRetry = { [weak self] in
            self?.startUpdateAll(debugPlayback: debugPlayback)
        }
        progressController.configure(
            title: debugPlayback ? "Update Playback" : "Update All",
            awaitingClearance: debugPlayback
                ? "Ready to replay update progress"
                : "Waiting for helper authorization",
            idleStatus: updateStatusText(packageCount: packageCount),
            successOperation: "Update Complete",
            failureOperation: "Update Halted",
            activePrimaryTitle: "Updating"
        )
    }

    private func configure(
        _ progressController: UpdateProgressViewController,
        request: PackageOperationRequest
    ) {
        progressController.onRetry = { [weak self] in
            self?.startPackageOperation(request)
        }
        progressController.configure(
            title: request.kind.progressSheetTitle,
            awaitingClearance: "Waiting for helper authorization",
            idleStatus: packageOperationStatusText(request),
            successOperation: request.kind.successOperationTitle,
            failureOperation: request.kind.failureOperationTitle,
            activePrimaryTitle: request.kind.progressTitle
        )
    }

    private func activationLog(packageCount: Int, debugPlayback: Bool) -> String {
        let countText = updateStatusText(packageCount: packageCount)
        return debugPlayback
            ? "Replaying update progress for \(countText)."
            : "Preparing updates for \(countText)."
    }

    private func updateStatusText(packageCount: Int) -> String {
        packageCount == 1
            ? "1 outdated package"
            : "\(packageCount) outdated packages"
    }

    private func packageOperationStatusText(_ request: PackageOperationRequest) -> String {
        request.packageNames.count == 1
            ? request.displayName
            : "\(request.packageNames.count) packages"
    }

    private func packageOperationActivationLog(_ request: PackageOperationRequest) -> String {
        "\(request.kind.progressTitle) \(packageOperationStatusText(request))."
    }

    private func startModelIfNeeded() {
        guard didStartModel == false else {
            return
        }
        didStartModel = true
        model.start()
    }

    private func installToolbarIfNeeded() {
        guard let window = view.window else {
            return
        }
        if window.toolbar == mainToolbar, mainToolbar != nil {
            return
        }

        let toolbar = NSToolbar(identifier: .automicVaultMain)
        toolbar.delegate = self
        toolbar.displayMode = .iconOnly
        toolbar.allowsUserCustomization = false
        toolbar.centeredItemIdentifiers = [.automicVaultSearch]
        window.toolbar = toolbar
        window.toolbarStyle = .unified
        window.titlebarSeparatorStyle = .none
        mainToolbar = toolbar
        syncAppUpdateToolbarItem()
        syncAutomicVaultCLTInstallToolbarItem()
    }

    private func syncAppUpdateToolbarItem() {
        guard let toolbar = mainToolbar else {
            return
        }

        let itemIndex = toolbar.items.firstIndex {
            $0.itemIdentifier == .automicVaultAppUpdate
        }
        let shouldShowUpdate =
            appUpdateCoordinator.hasAvailableUpdate
            || appUpdateCoordinator.isInstalling

        if shouldShowUpdate {
            if itemIndex == nil {
                let insertionIndex = toolbar.items.firstIndex {
                    $0.itemIdentifier == .automicVaultRefresh
                } ?? toolbar.items.count
                toolbar.insertItem(
                    withItemIdentifier: .automicVaultAppUpdate,
                    at: insertionIndex
                )
            }
            updateAppUpdateToolbarItemState()
        } else if let itemIndex {
            toolbar.removeItem(at: itemIndex)
            appUpdateToolbarItem = nil
        }
    }

    private func updateAppUpdateToolbarItemState() {
        guard let item = appUpdateToolbarItem,
              let button = item.view as? NSButton else {
            return
        }

        let isInstalling = appUpdateCoordinator.isInstalling
        let title = isInstalling ? "Updating Automic Vault" : "Update Automic Vault"
        let toolTip = isInstalling
            ? "Installing the Automic Vault update"
            : "Install the staged Automic Vault update and relaunch"

        button.title = Self.appUpdateToolbarIconTitleSpacing
            + title
            + Self.appUpdateToolbarTrailingPadding
        button.image = NSImage(
            systemSymbolName: isInstalling
                ? "arrow.triangle.2.circlepath"
                : "arrow.down.circle.fill",
            accessibilityDescription: title
        )
        button.isEnabled = !isInstalling
        button.toolTip = toolTip
        button.sizeToFit()

        let fittingSize = button.fittingSize
        let size = NSSize(
            width: max(
                ceil(fittingSize.width + Self.appUpdateToolbarHorizontalPadding),
                182
            ),
            height: max(ceil(fittingSize.height), 28)
        )
        button.frame.size = size
        item.label = title
        item.paletteLabel = title
        item.toolTip = toolTip
    }

    private func syncAutomicVaultCLTInstallToolbarItem() {
        guard let toolbar = mainToolbar else {
            return
        }

        let itemIndex = toolbar.items.firstIndex {
            $0.itemIdentifier == .automicVaultCLTInstall
        }

        if model.shouldShowAutomicVaultCLTInstallButton {
            if itemIndex == nil {
                let insertionIndex = toolbar.items.firstIndex {
                    $0.itemIdentifier == .automicVaultRefresh
                } ?? toolbar.items.count
                toolbar.insertItem(
                    withItemIdentifier: .automicVaultCLTInstall,
                    at: insertionIndex
                )
            }
            updateAutomicVaultCLTInstallToolbarItemState()
        } else if let itemIndex {
            toolbar.removeItem(at: itemIndex)
            automicVaultCLTInstallToolbarItem = nil
        }
    }

    private func updateAutomicVaultCLTInstallToolbarItemState() {
        guard let item = automicVaultCLTInstallToolbarItem,
              let button = item.view as? NSButton else {
            return
        }

        let isInstalling = model.isInstallingAutomicVaultCLT
        let title = isInstalling ? "Installing av" : "Install av"
        let toolTip: String
        if isInstalling {
            toolTip = "Installing the bundled av command line tool"
        } else if model.isPackageMutationInFlight {
            toolTip = "Finish the current package operation before installing av"
        } else {
            toolTip = "Install the bundled av command line tool to /usr/local/bin/av"
        }

        button.title = Self.appUpdateToolbarIconTitleSpacing
            + title
            + Self.appUpdateToolbarTrailingPadding
        button.image = NSImage(
            systemSymbolName: isInstalling
                ? "arrow.triangle.2.circlepath"
                : "terminal.fill",
            accessibilityDescription: title
        )
        button.isEnabled = model.canRequestAutomicVaultCLTInstall
        button.toolTip = toolTip
        button.sizeToFit()

        let fittingSize = button.fittingSize
        let size = NSSize(
            width: max(
                ceil(fittingSize.width + Self.appUpdateToolbarHorizontalPadding),
                116
            ),
            height: max(ceil(fittingSize.height), 28)
        )
        button.frame.size = size
        item.label = title
        item.paletteLabel = title
        item.toolTip = toolTip
    }

    private func confirmAndInstallAppUpdate() {
        guard let window = view.window else {
            installAppUpdateIfReady()
            return
        }

        let alert = NSAlert()
        alert.messageText = "Update Automic Vault?"
        alert.informativeText = "Automic Vault will quit and relaunch after the update is installed."
        alert.alertStyle = .informational
        alert.addButton(withTitle: "Update Automic Vault")
        alert.addButton(withTitle: "Cancel")
        alert.beginSheetModal(for: window) { [weak self] response in
            guard response == .alertFirstButtonReturn else {
                return
            }
            DispatchQueue.main.async {
                self?.installAppUpdateIfReady()
            }
        }
    }

    private func installAppUpdateIfReady() {
        appUpdateCoordinator.installWhenReady(
            readiness: { [weak self] in
                self?.appUpdateInstallReadiness()
                    ?? .busy("Main window is unavailable.")
            },
            prepareForInstall: { [weak self] in
                self?.model.showTransientStatus("Installing Automic Vault update")
            }
        )
    }

    private func appUpdateInstallReadiness() -> AppUpdateCoordinator.InstallReadiness {
        if model.isPackageMutationInFlight {
            return .busy(
                "Finish the current package operation before updating Automic Vault."
            )
        }
        if view.window?.attachedSheet != nil {
            return .busy(
                "Close the current sheet before updating Automic Vault."
            )
        }
        return .ready
    }

    private func presentAppUpdateError(_ message: String) {
        let alert = NSAlert()
        alert.messageText = "Could Not Update Automic Vault"
        alert.informativeText = message
        alert.alertStyle = .warning
        alert.addButton(withTitle: "OK")

        if let window = view.window, window.attachedSheet == nil {
            alert.beginSheetModal(for: window)
        } else {
            alert.runModal()
        }
    }

    private func installSearchShortcutMonitorIfNeeded() {
        guard searchShortcutMonitor == nil else {
            return
        }
        searchShortcutMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            guard let self,
                  event.window === self.view.window,
                  self.isSearchShortcut(event) else {
                return event
            }
            self.requestSearchFocus()
            return nil
        }
    }

    private func removeSearchShortcutMonitor() {
        guard let searchShortcutMonitor else {
            return
        }
        NSEvent.removeMonitor(searchShortcutMonitor)
        self.searchShortcutMonitor = nil
    }

    private func isSearchShortcut(_ event: NSEvent) -> Bool {
        let modifiers = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        let hasOnlyCommand = modifiers.contains(.command)
            && modifiers.isDisjoint(with: [.control, .option, .shift])
        let key = event.charactersIgnoringModifiers?.lowercased()
        return hasOnlyCommand
            && key.map(Self.searchShortcutKeys.contains) == true
    }
}

extension MainWindowController: NSToolbarDelegate {
    func toolbarAllowedItemIdentifiers(_ toolbar: NSToolbar) -> [NSToolbarItem.Identifier] {
        [
            .flexibleSpace,
            .automicVaultSearch,
            .automicVaultRefresh,
            .automicVaultAppUpdate,
            .automicVaultCLTInstall,
        ]
    }

    func toolbarDefaultItemIdentifiers(_ toolbar: NSToolbar) -> [NSToolbarItem.Identifier] {
        [
            .flexibleSpace,
            .automicVaultSearch,
            .flexibleSpace,
            .automicVaultRefresh,
        ]
    }

    func toolbar(
        _ toolbar: NSToolbar,
        itemForItemIdentifier itemIdentifier: NSToolbarItem.Identifier,
        willBeInsertedIntoToolbar flag: Bool
    ) -> NSToolbarItem? {
        switch itemIdentifier {
        case .automicVaultSearch:
            let item = NSSearchToolbarItem(itemIdentifier: itemIdentifier)
            item.label = "Search"
            item.paletteLabel = "Search"
            item.toolTip = "Search packages"
            item.preferredWidthForSearchField = 318
            item.resignsFirstResponderWithCancel = true
            configureSearchField(item.searchField)
            item.visibilityPriority = .high
            searchToolbarItem = item
            return item
        case .automicVaultRefresh:
            let item = NSToolbarItem(itemIdentifier: itemIdentifier)
            item.label = "Refresh"
            item.paletteLabel = "Refresh"
            item.toolTip = "Refresh packages"
            item.image = NSImage(
                systemSymbolName: "arrow.clockwise",
                accessibilityDescription: "Refresh packages"
            )
            item.target = self
            item.action = #selector(refreshToolbarItemPressed(_:))
            item.visibilityPriority = .high
            return item
        case .automicVaultAppUpdate:
            let item = NSToolbarItem(itemIdentifier: itemIdentifier)
            item.label = "Update Automic Vault"
            item.paletteLabel = "Update Automic Vault"
            item.visibilityPriority = .high

            let button = NSButton(
                title: "Update Automic Vault",
                target: self,
                action: #selector(appUpdateToolbarItemPressed(_:))
            )
            button.bezelStyle = .rounded
            button.controlSize = .small
            button.font = .systemFont(ofSize: 12, weight: .semibold)
            button.imagePosition = .imageLeading
            button.imageHugsTitle = true
            button.imageScaling = .scaleProportionallyDown
            button.setButtonType(.momentaryPushIn)
            item.view = button
            appUpdateToolbarItem = item
            updateAppUpdateToolbarItemState()
            return item
        case .automicVaultCLTInstall:
            let item = NSToolbarItem(itemIdentifier: itemIdentifier)
            item.label = "Install av"
            item.paletteLabel = "Install av"
            item.visibilityPriority = .high

            let button = NSButton(
                title: "Install av",
                target: self,
                action: #selector(automicVaultCLTInstallToolbarItemPressed(_:))
            )
            button.bezelStyle = .rounded
            button.controlSize = .small
            button.font = .systemFont(ofSize: 12, weight: .semibold)
            button.imagePosition = .imageLeading
            button.imageHugsTitle = true
            button.imageScaling = .scaleProportionallyDown
            button.setButtonType(.momentaryPushIn)
            item.view = button
            automicVaultCLTInstallToolbarItem = item
            updateAutomicVaultCLTInstallToolbarItemState()
            return item
        default:
            return nil
        }
    }

    private func configureSearchField(_ searchField: NSSearchField) {
        searchField.placeholderString = "Search"
        searchField.stringValue = model.searchText
        searchField.font = .systemFont(ofSize: 13, weight: .regular)
        searchField.delegate = self
        searchField.target = self
        searchField.action = #selector(searchToolbarItemChanged(_:))
        searchField.sendsSearchStringImmediately = true
        searchField.sendsWholeSearchString = false
    }

    private func updateSearchText(_ text: String) {
        guard model.searchText != text else {
            return
        }
        model.searchText = text
    }
}

extension MainWindowController: NSSearchFieldDelegate {
    func controlTextDidChange(_ notification: Notification) {
        guard let searchField = notification.object as? NSSearchField else {
            return
        }
        updateSearchText(searchField.stringValue)
    }

    func control(
        _ control: NSControl,
        textView: NSTextView,
        doCommandBy commandSelector: Selector
    ) -> Bool {
        guard commandSelector == #selector(NSResponder.cancelOperation(_:)),
              let searchField = control as? NSSearchField else {
            return false
        }
        clearAndDeactivateSearch(searchField, fieldEditor: textView)
        return true
    }

    private func clearAndDeactivateSearch(
        _ searchField: NSSearchField,
        fieldEditor: NSTextView
    ) {
        searchField.stringValue = ""
        fieldEditor.string = ""
        updateSearchText("")
        searchField.abortEditing()
        searchField.window?.makeFirstResponder(nil)
    }
}

private extension NSToolbar.Identifier {
    static let automicVaultMain = NSToolbar.Identifier("AutomicVaultMainToolbar")
}

private extension NSToolbarItem.Identifier {
    static let automicVaultSearch = NSToolbarItem.Identifier("AutomicVaultSearch")
    static let automicVaultRefresh = NSToolbarItem.Identifier("AutomicVaultRefresh")
    static let automicVaultAppUpdate = NSToolbarItem.Identifier("AutomicVaultAppUpdate")
    static let automicVaultCLTInstall = NSToolbarItem.Identifier("AutomicVaultCLTInstall")
}
