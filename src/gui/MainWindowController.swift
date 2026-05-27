import AppKit
import SwiftUI

@MainActor
final class MainWindowController: NSHostingController<MainWindowView> {
    private let model: MainWindowModel
    private var didStartModel = false
    private var searchShortcutMonitor: Any?
    private weak var mainToolbar: NSToolbar?

    init() {
        let model = MainWindowModel()
        self.model = model
        super.init(rootView: MainWindowView(model: model))
    }

    @MainActor @preconcurrency required dynamic init?(coder: NSCoder) {
        let model = MainWindowModel()
        self.model = model
        super.init(coder: coder, rootView: MainWindowView(model: model))
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
    }

    #if DEBUG
    func runDebugFakeUpdate() {
        model.showTransientStatus("Debug update playback is not part of the Liquid Glass mockup.")
    }
    #endif

    func applicationWillTerminate() {
        model.stop()
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
        return hasOnlyCommand
            && event.charactersIgnoringModifiers?.lowercased() == "k"
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
            let item = NSToolbarItem(itemIdentifier: itemIdentifier)
            let view = NSHostingView(rootView: MainWindowToolbarSearch(model: model))
            constrain(view, width: 318, height: 34)
            item.view = view
            item.visibilityPriority = .high
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

    private func constrain(_ view: NSView, width: CGFloat, height: CGFloat) {
        view.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            view.widthAnchor.constraint(equalToConstant: width),
            view.heightAnchor.constraint(equalToConstant: height),
        ])
    }
}

private extension NSToolbar.Identifier {
    static let automicVaultMain = NSToolbar.Identifier("AutomicVaultMainToolbar")
}

private extension NSToolbarItem.Identifier {
    static let automicVaultSearch = NSToolbarItem.Identifier("AutomicVaultSearch")
    static let automicVaultRefresh = NSToolbarItem.Identifier("AutomicVaultRefresh")
}
