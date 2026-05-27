import AppKit
import SwiftUI

@MainActor
final class MainWindowController: NSHostingController<MainWindowView> {
    private let model: MainWindowModel
    private var didStartModel = false
    private var searchShortcutMonitor: Any?

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
