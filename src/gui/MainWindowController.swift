import AppKit
import SwiftUI

@MainActor
final class MainWindowController: NSHostingController<MainWindowView> {
    private let model: MainWindowModel

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
        model.start()
    }

    func requestRefresh() {
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
}
