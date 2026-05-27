import AppKit
import SwiftUI

@MainActor
final class MainWindowController: NSHostingController<MainWindowView> {
    private let model: MainWindowModel
    private var didStartModel = false

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

    func requestRefresh() {
        startModelIfNeeded()
        model.reloadPackages()
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
}
