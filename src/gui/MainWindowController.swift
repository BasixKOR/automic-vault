import AppKit
import Combine
import SwiftUI

@MainActor
final class MainWindowController: NSHostingController<MainWindowView> {
    private static let searchShortcutKeys: Set<String> = ["k", "l", "p"]

    private let model: MainWindowModel
    private let helperBridge = NukeHelperBridge()
    private var didStartModel = false
    private var searchShortcutMonitor: Any?
    private weak var mainToolbar: NSToolbar?
    private weak var searchToolbarItem: NSSearchToolbarItem?
    private var updateAllRequestCancellable: AnyCancellable?
    private var updateProgressViewController: UpdateProgressViewController?

    init() {
        let model = MainWindowModel()
        self.model = model
        super.init(rootView: MainWindowView(model: model))
        installUpdateAllRequestObserver()
    }

    @MainActor @preconcurrency required dynamic init?(coder: NSCoder) {
        let model = MainWindowModel()
        self.model = model
        super.init(coder: coder, rootView: MainWindowView(model: model))
        installUpdateAllRequestObserver()
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
        model.stop()
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

        let handleProgress: (NukeHelperProgressEvent) -> Void = { [weak progressController] event in
            progressController?.handle(event: event)
        }
        let handleCompletion: (Result<NukeHelperResult, Error>) -> Void = {
            [weak self, weak progressController] result in
            guard let self else { return }
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
            completion: handleCompletion
        )
    }

    private func presentUpdateProgressController() -> UpdateProgressViewController {
        if let updateProgressViewController {
            return updateProgressViewController
        }

        let controller = UpdateProgressViewController()
        controller.preferredContentSize = NSSize(width: 860, height: 760)
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
            title: debugPlayback ? "NUCLEUS UPDATE PLAYBACK" : "NUCLEUS UPDATE CHANNEL",
            awaitingClearance: debugPlayback
                ? "Debug update stream ready"
                : "Awaiting helper authorization",
            idleStatus: updateStatusText(packageCount: packageCount),
            successOperation: "Update Complete",
            failureOperation: "Update Halted"
        )
    }

    private func activationLog(packageCount: Int, debugPlayback: Bool) -> String {
        let countText = updateStatusText(packageCount: packageCount)
        return debugPlayback
            ? "Starting debug playback for \(countText)."
            : "Starting update all for \(countText)."
    }

    private func updateStatusText(packageCount: Int) -> String {
        packageCount == 1
            ? "1 outdated package"
            : "\(packageCount) outdated packages"
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
}
