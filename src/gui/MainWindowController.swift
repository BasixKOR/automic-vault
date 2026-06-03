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
    private let statusStore = NucleusStatusStore()
    private var didStartModel = false
    private var searchShortcutMonitor: Any?
    private weak var mainToolbar: NSToolbar?
    private weak var searchToolbarItem: NSSearchToolbarItem?
    private weak var appUpdateToolbarItem: NSToolbarItem?
    private weak var automicVaultCLTInstallToolbarItem: NSToolbarItem?
    private weak var helperMaintenanceToolbarItem: NSToolbarItem?
    private var updateAllRequestCancellable: AnyCancellable?
    private var packageOperationRequestCancellable: AnyCancellable?
    private var searchTextCancellable: AnyCancellable?
    private var searchDeactivationRequestCancellable: AnyCancellable?
    private var cltInstallToolbarStateCancellable: AnyCancellable?
    private var helperMaintenanceToolbarStateCancellable: AnyCancellable?
    private var updateProgressViewController: UpdateProgressViewController?
    private var helperNeedsMaintenance = false
    private var isRefreshingHelperMaintenanceState = false
    private var isUpdatingHelper = false {
        didSet {
            syncAutomicVaultCLTInstallToolbarItem()
            syncHelperMaintenanceToolbarItem()
        }
    }
    private var isAuthorizingPrivilegedOperation = false {
        didSet {
            syncAutomicVaultCLTInstallToolbarItem()
            updateHelperMaintenanceToolbarItemState()
        }
    }

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
        installHelperMaintenanceToolbarObserver()
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
        installHelperMaintenanceToolbarObserver()
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

    @objc private func helperMaintenanceToolbarItemPressed(_ sender: Any?) {
        beginHelperMaintenance()
    }

    func requestSearchFocus() {
        startModelIfNeeded()
        model.requestSearchFocus()
        searchToolbarItem?.beginSearchInteraction()
    }

    func requestPackageInstall(packageNames: [String]) {
        startModelIfNeeded()
        model.requestPackageInstall(packageNames: packageNames)
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
        helperMaintenanceToolbarStateCancellable?.cancel()
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

    private func installHelperMaintenanceToolbarObserver() {
        let snapshotChanges = model.$snapshot.map { _ in () }
        let operationChanges = model.$activePackageOperation.map { _ in () }
        let updateAllChanges = model.$isUpdatingAll.map { _ in () }
        helperMaintenanceToolbarStateCancellable = Publishers.Merge3(
            snapshotChanges,
            operationChanges,
            updateAllChanges
        )
        .sink { [weak self] _ in
            Task { @MainActor in
                self?.syncHelperMaintenanceToolbarItem()
                self?.refreshHelperMaintenanceState()
            }
        }
    }

    private func refreshHelperMaintenanceState() {
        guard isRefreshingHelperMaintenanceState == false else {
            return
        }
        isRefreshingHelperMaintenanceState = true
        helperBridge.helperNeedsInstallationOrUpdate { [weak self] result in
            guard let self else { return }
            self.isRefreshingHelperMaintenanceState = false
            switch result {
            case .success(let needsMaintenance):
                self.helperNeedsMaintenance = needsMaintenance
            case .failure:
                self.helperNeedsMaintenance =
                    self.model.snapshot.remoteDatabaseRefreshState == .pendingHelperInstallation
            }
            self.syncHelperMaintenanceToolbarItem()
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

    private func authorizePrivilegedHelperOperation(
        reason: String,
        completion: @escaping () -> Void
    ) {
        guard isAuthorizingPrivilegedOperation == false else {
            model.showTransientStatus(L10n.string("Authentication already in progress"))
            return
        }
        isAuthorizingPrivilegedOperation = true
        model.showTransientStatus(L10n.string("Waiting for Touch ID authorization"))
        helperBridge.authenticateBiometrics(reason: reason) { [weak self] result in
            guard let self else { return }
            self.isAuthorizingPrivilegedOperation = false
            switch result {
            case .success:
                completion()
            case .failure(let error):
                self.presentHelperError(error)
            }
        }
    }

    private func beginHelperMaintenance() {
        guard isUpdatingHelper == false,
              model.isPackageMutationInFlight == false else {
            model.showTransientStatus(L10n.string("Privileged operation already in progress"))
            return
        }

        authorizePrivilegedHelperOperation(
            reason: L10n.string("Authorize privileged helper update for Automic Vault.")
        ) { [weak self] in
            self?.installOrUpdateHelper()
        }
    }

    private func installOrUpdateHelper() {
        isUpdatingHelper = true
        helperNeedsMaintenance = true
        model.showTransientStatus(L10n.string("Updating privileged helper"))
        helperBridge.installOrUpdateHelper { [weak self] result in
            guard let self else { return }
            self.isUpdatingHelper = false
            switch result {
            case .success(let maintenanceResult):
                self.helperNeedsMaintenance = false
                try? self.statusStore.saveRemoteDatabaseRefreshState(.normal)
                self.statusStore.requestRefresh()
                switch maintenanceResult {
                case .completed(let updated):
                    self.model.showTransientStatus(
                        updated
                            ? L10n.string("Privileged helper updated")
                            : L10n.string("Privileged helper is current")
                    )
                case .pendingHelperInstallation:
                    self.helperNeedsMaintenance = true
                    self.model.showTransientStatus(
                        L10n.string("Privileged helper still needs installation")
                    )
                }
            case .failure(let error):
                self.helperNeedsMaintenance = true
                self.presentHelperError(error)
            }
            self.syncHelperMaintenanceToolbarItem()
            self.refreshHelperMaintenanceState()
        }
    }

    private func startUpdateAll(debugPlayback: Bool) {
        #if DEBUG
        if debugPlayback {
            startAuthorizedUpdateAll(debugPlayback: true)
            return
        }
        #endif

        authorizePrivilegedHelperOperation(
            reason: L10n.string("Authorize privileged package updates for Automic Vault.")
        ) { [weak self] in
            self?.startAuthorizedUpdateAll(debugPlayback: debugPlayback)
        }
    }

    private func startAuthorizedUpdateAll(debugPlayback: Bool) {
        guard !model.isUpdatingAll,
              isUpdatingHelper == false else {
            if isUpdatingHelper {
                model.showTransientStatus(
                    L10n.string("Privileged helper update already in progress")
                )
            }
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
            model.showTransientStatus(L10n.string("No outdated packages to update"))
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
                ? L10n.string("Playing debug update stream")
                : L10n.string("Awaiting helper authorization")
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
                    L10n.string("Bundled av command line tool was not staged for installation.")
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
                            : L10n.string("Update complete"),
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

    nonisolated private static func performIsotopeMigration(
        isotopeName: String
    ) throws -> NucleusBridge.IsotopeMigrationPlan {
        try NucleusBridge(daemonOwnership: .owner)
            .migrateIsotope(isotopeName: isotopeName)
    }

    private func startPackageOperation(_ request: PackageOperationRequest) {
        guard !model.isPackageMutationInFlight,
              isUpdatingHelper == false else {
            model.showTransientStatus(L10n.string("Privileged operation already in progress"))
            return
        }

        if request.isXcodeCLT, request.kind == .install {
            startXcodeCommandLineToolsInstall(request)
            return
        }

        authorizePrivilegedHelperOperation(
            reason: privilegedAuthorizationReason(for: request)
        ) { [weak self] in
            self?.startAuthorizedPackageOperation(request)
        }
    }

    private func startAuthorizedPackageOperation(_ request: PackageOperationRequest) {
        guard !model.isPackageMutationInFlight,
              isUpdatingHelper == false else {
            model.showTransientStatus(L10n.string("Privileged operation already in progress"))
            return
        }

        let progressController = presentUpdateProgressController()
        configure(progressController, request: request)
        model.beginPackageOperation(request)
        progressController.begin(
            packages: request.packageNames,
            activationLog: packageOperationActivationLog(request),
            initialOperation: L10n.string("Awaiting helper authorization")
        )

        var stagedCLTDirectory: URL?
        let handleProgress: (NukeHelperProgressEvent) -> Void = { [weak progressController] event in
            progressController?.handle(event: event)
        }
        let finishOperation: (Result<NukeHelperResult, Error>) -> Void = {
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
        let handleCompletion: (Result<NukeHelperResult, Error>) -> Void = {
            [weak progressController] result in
            guard case .harden = request.kind,
                  let migrationIsotopeName = request.migrationIsotopeName,
                  case .success(let helperResult) = result else {
                finishOperation(result)
                return
            }

            let migrationPackageName = request.packageNames.first ?? "isotope:\(migrationIsotopeName)"
            progressController?.handle(event: .log(
                package: migrationPackageName,
                message: L10n.string("migrating secrets")
            ))
            Task.detached(priority: .userInitiated) {
                let migrationResult = Result {
                    try Self.performIsotopeMigration(isotopeName: migrationIsotopeName)
                }
                await MainActor.run {
                    switch migrationResult {
                    case .success:
                        finishOperation(.success(NukeHelperResult(
                            message: L10n.string("Hardening complete"),
                            processedPackages: Self.mergedProcessedPackages(
                                helperResult.processedPackages,
                                request.packageNames
                            )
                        )))
                    case .failure(let error):
                        finishOperation(.failure(error))
                    }
                }
            }
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
        case .install, .harden:
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

    private func privilegedAuthorizationReason(
        for request: PackageOperationRequest
    ) -> String {
        if request.isAutomicVaultCLT {
            return L10n.string(
                "Authorize installation of Automic Vault command line tools into /usr/local/bin."
            )
        }

        switch request.kind {
        case .install:
            return L10n.format(
                "Authorize privileged package install for %@.",
                request.displayName
            )
        case .update:
            return L10n.format(
                "Authorize privileged package update for %@.",
                request.displayName
            )
        case .uninstall:
            return L10n.format(
                "Authorize privileged package uninstall for %@.",
                request.displayName
            )
        case .harden:
            return L10n.format(
                "Authorize privileged security hardening for %@.",
                request.displayName
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
                    message: L10n.string("Command Line Tools installer launched"),
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
            title: debugPlayback ? L10n.string("Update Playback") : L10n.string("Update All"),
            awaitingClearance: debugPlayback
                ? L10n.string("Ready to replay update progress")
                : L10n.string("Waiting for helper authorization"),
            idleStatus: updateStatusText(packageCount: packageCount),
            successOperation: L10n.string("Update Complete"),
            failureOperation: L10n.string("Update Halted"),
            activePrimaryTitle: L10n.string("Updating")
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
            awaitingClearance: L10n.string("Waiting for helper authorization"),
            idleStatus: packageOperationStatusText(request),
            successOperation: request.kind.successOperationTitle,
            failureOperation: request.kind.failureOperationTitle,
            activePrimaryTitle: request.kind.progressTitle
        )
    }

    private func activationLog(packageCount: Int, debugPlayback: Bool) -> String {
        let countText = updateStatusText(packageCount: packageCount)
        return debugPlayback
            ? L10n.format("Replaying update progress for %@.", countText)
            : L10n.format("Preparing updates for %@.", countText)
    }

    private func updateStatusText(packageCount: Int) -> String {
        packageCount == 1
            ? L10n.string("1 outdated package")
            : L10n.format("%d outdated packages", packageCount)
    }

    private func packageOperationStatusText(_ request: PackageOperationRequest) -> String {
        request.packageNames.count == 1
            ? request.displayName
            : L10n.format("%d packages", request.packageNames.count)
    }

    private func packageOperationActivationLog(_ request: PackageOperationRequest) -> String {
        L10n.format("%@ %@.", request.kind.progressTitle, packageOperationStatusText(request))
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
        syncHelperMaintenanceToolbarItem()
        refreshHelperMaintenanceState()
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
        let title = isInstalling
            ? L10n.string("Updating Automic Vault")
            : L10n.string("Update Automic Vault")
        let toolTip = isInstalling
            ? L10n.string("Installing the Automic Vault update")
            : L10n.string("Install the staged Automic Vault update and relaunch")

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
        let title = isInstalling
            ? L10n.string("Installing Automic Vault CLI")
            : L10n.string("Install Automic Vault CLI")
        let toolTip: String
        if isInstalling {
            toolTip = L10n.string("Installing the bundled av command line tool")
        } else if isAuthorizingPrivilegedOperation {
            toolTip = L10n.string("Complete Touch ID authorization before installing av")
        } else if isUpdatingHelper {
            toolTip = L10n.string("Finish the privileged helper update before installing av")
        } else if model.isPackageMutationInFlight {
            toolTip = L10n.string("Finish the current package operation before installing av")
        } else {
            toolTip = L10n.string("Install the bundled Automic Vault CLI to /usr/local/bin/av")
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
            && isAuthorizingPrivilegedOperation == false
            && isUpdatingHelper == false
        button.toolTip = toolTip
        button.sizeToFit()

        let fittingSize = button.fittingSize
        let size = NSSize(
            width: max(
                ceil(fittingSize.width + Self.appUpdateToolbarHorizontalPadding),
                206
            ),
            height: max(ceil(fittingSize.height), 28)
        )
        button.frame.size = size
        item.label = title
        item.paletteLabel = title
        item.toolTip = toolTip
    }

    private func syncHelperMaintenanceToolbarItem() {
        guard let toolbar = mainToolbar else {
            return
        }

        let itemIndex = toolbar.items.firstIndex {
            $0.itemIdentifier == .automicVaultHelperUpdate
        }
        let shouldShow = isUpdatingHelper
            || helperNeedsMaintenance
            || model.snapshot.remoteDatabaseRefreshState == .pendingHelperInstallation

        if shouldShow {
            if itemIndex == nil {
                let insertionIndex = toolbar.items.firstIndex {
                    $0.itemIdentifier == .automicVaultRefresh
                } ?? toolbar.items.count
                toolbar.insertItem(
                    withItemIdentifier: .automicVaultHelperUpdate,
                    at: insertionIndex
                )
            }
            updateHelperMaintenanceToolbarItemState()
        } else if let itemIndex {
            toolbar.removeItem(at: itemIndex)
            helperMaintenanceToolbarItem = nil
        }
    }

    private func updateHelperMaintenanceToolbarItemState() {
        guard let item = helperMaintenanceToolbarItem,
              let button = item.view as? NSButton else {
            return
        }

        let title = isUpdatingHelper
            ? L10n.string("Updating Helper")
            : L10n.string("Update Helper")
        let toolTip: String
        if isUpdatingHelper {
            toolTip = L10n.string("Installing the bundled privileged helper")
        } else if isAuthorizingPrivilegedOperation {
            toolTip = L10n.string("Complete Touch ID authorization before updating the helper")
        } else if model.isPackageMutationInFlight {
            toolTip = L10n.string("Finish the current package operation before updating the helper")
        } else {
            toolTip = L10n.string("Install the bundled privileged helper")
        }

        button.title = Self.appUpdateToolbarIconTitleSpacing
            + title
            + Self.appUpdateToolbarTrailingPadding
        button.image = NSImage(
            systemSymbolName: isUpdatingHelper
                ? "arrow.triangle.2.circlepath"
                : "lock.shield.fill",
            accessibilityDescription: title
        )
        button.isEnabled = isUpdatingHelper == false
            && isAuthorizingPrivilegedOperation == false
            && model.isPackageMutationInFlight == false
        button.toolTip = toolTip
        button.sizeToFit()

        let fittingSize = button.fittingSize
        let size = NSSize(
            width: max(
                ceil(fittingSize.width + Self.appUpdateToolbarHorizontalPadding),
                158
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
        alert.messageText = L10n.string("Update Automic Vault?")
        alert.informativeText = L10n.string(
            "Automic Vault will quit and relaunch after the update is installed."
        )
        alert.alertStyle = .informational
        alert.addButton(withTitle: L10n.string("Update Automic Vault"))
        alert.addButton(withTitle: L10n.string("Cancel"))
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
                    ?? .busy(L10n.string("Main window is unavailable."))
            },
            prepareForInstall: { [weak self] in
                self?.model.showTransientStatus(
                    L10n.string("Installing Automic Vault update")
                )
            }
        )
    }

    private func appUpdateInstallReadiness() -> AppUpdateCoordinator.InstallReadiness {
        if model.isPackageMutationInFlight {
            return .busy(
                L10n.string("Finish the current package operation before updating Automic Vault.")
            )
        }
        if isUpdatingHelper {
            return .busy(
                L10n.string("Finish the privileged helper update before updating Automic Vault.")
            )
        }
        if isAuthorizingPrivilegedOperation {
            return .busy(
                L10n.string("Complete Touch ID authorization before updating Automic Vault.")
            )
        }
        if view.window?.attachedSheet != nil {
            return .busy(
                L10n.string("Close the current sheet before updating Automic Vault.")
            )
        }
        return .ready
    }

    private func presentAppUpdateError(_ message: String) {
        let alert = NSAlert()
        alert.messageText = L10n.string("Could Not Update Automic Vault")
        alert.informativeText = message
        alert.alertStyle = .warning
        alert.addButton(withTitle: L10n.string("OK"))

        if let window = view.window, window.attachedSheet == nil {
            alert.beginSheetModal(for: window)
        } else {
            alert.runModal()
        }
    }

    private func presentHelperError(_ error: Error) {
        if case NukeHelperBridgeError.biometricCanceled = error {
            return
        }

        let alert = NSAlert()
        alert.messageText = L10n.string("Privileged Operation Failed")
        alert.informativeText = error.localizedDescription
        alert.alertStyle = .warning
        alert.addButton(withTitle: L10n.string("OK"))

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
            .automicVaultHelperUpdate,
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
            item.label = L10n.string("Search")
            item.paletteLabel = L10n.string("Search")
            item.toolTip = L10n.string("Search packages")
            item.preferredWidthForSearchField = 318
            item.resignsFirstResponderWithCancel = true
            configureSearchField(item.searchField)
            item.visibilityPriority = .high
            searchToolbarItem = item
            return item
        case .automicVaultRefresh:
            let item = NSToolbarItem(itemIdentifier: itemIdentifier)
            item.label = L10n.string("Refresh")
            item.paletteLabel = L10n.string("Refresh")
            item.toolTip = L10n.string("Refresh packages")
            item.image = NSImage(
                systemSymbolName: "arrow.clockwise",
                accessibilityDescription: L10n.string("Refresh packages")
            )
            item.target = self
            item.action = #selector(refreshToolbarItemPressed(_:))
            item.visibilityPriority = .high
            return item
        case .automicVaultAppUpdate:
            let item = NSToolbarItem(itemIdentifier: itemIdentifier)
            item.label = L10n.string("Update Automic Vault")
            item.paletteLabel = L10n.string("Update Automic Vault")
            item.visibilityPriority = .high

            let button = NSButton(
                title: L10n.string("Update Automic Vault"),
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
            item.label = L10n.string("Install Automic Vault CLI")
            item.paletteLabel = L10n.string("Install Automic Vault CLI")
            item.visibilityPriority = .high

            let button = NSButton(
                title: L10n.string("Install Automic Vault CLI"),
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
        case .automicVaultHelperUpdate:
            let item = NSToolbarItem(itemIdentifier: itemIdentifier)
            item.label = L10n.string("Update Helper")
            item.paletteLabel = L10n.string("Update Helper")
            item.visibilityPriority = .high

            let button = NSButton(
                title: L10n.string("Update Helper"),
                target: self,
                action: #selector(helperMaintenanceToolbarItemPressed(_:))
            )
            button.bezelStyle = .rounded
            button.controlSize = .small
            button.font = .systemFont(ofSize: 12, weight: .semibold)
            button.imagePosition = .imageLeading
            button.imageHugsTitle = true
            button.imageScaling = .scaleProportionallyDown
            button.setButtonType(.momentaryPushIn)
            item.view = button
            helperMaintenanceToolbarItem = item
            updateHelperMaintenanceToolbarItemState()
            return item
        default:
            return nil
        }
    }

    private func configureSearchField(_ searchField: NSSearchField) {
        searchField.placeholderString = L10n.string("Search")
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
    static let automicVaultHelperUpdate = NSToolbarItem.Identifier("AutomicVaultHelperUpdate")
}
