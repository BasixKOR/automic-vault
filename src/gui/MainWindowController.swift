import AppKit
import SwiftUI

@MainActor
final class MainWindowController: NSHostingController<MainWindowView> {
    private let model: MainWindowModel
    private var didStartModel = false
    private var searchShortcutMonitor: Any?
    private weak var mainToolbar: NSToolbar?
    private weak var searchToolbarItem: NSSearchToolbarItem?

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
        searchToolbarItem?.beginSearchInteraction()
    }

    @objc private func searchToolbarItemChanged(_ sender: NSSearchField) {
        updateSearchText(sender.stringValue)
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
            let item = NSSearchToolbarItem(itemIdentifier: itemIdentifier)
            let searchField = CenteredPlaceholderSearchField(frame: .zero)
            item.label = "Search"
            item.paletteLabel = "Search"
            item.toolTip = "Search packages"
            item.preferredWidthForSearchField = 318
            item.resignsFirstResponderWithCancel = true
            item.searchField = searchField
            configureSearchField(searchField)
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
        searchField.placeholderString = "Search Open Source"
        searchField.stringValue = model.searchText
        searchField.font = .systemFont(ofSize: 13, weight: .regular)
        searchField.delegate = self
        searchField.target = self
        searchField.action = #selector(searchToolbarItemChanged(_:))
        searchField.sendsSearchStringImmediately = true
        searchField.sendsWholeSearchString = false
        (searchField as? CenteredPlaceholderSearchField)?.syncCentering(animated: false)
    }

    private func updateSearchText(_ text: String) {
        guard model.searchText != text else {
            return
        }
        model.searchText = text
    }
}

extension MainWindowController: NSSearchFieldDelegate {
    func controlTextDidBeginEditing(_ notification: Notification) {
        syncSearchFieldCentering(from: notification, animated: true)
    }

    func controlTextDidChange(_ notification: Notification) {
        guard let searchField = notification.object as? NSSearchField else {
            return
        }
        updateSearchText(searchField.stringValue)
        syncSearchFieldCentering(searchField, animated: true)
    }

    func controlTextDidEndEditing(_ notification: Notification) {
        syncSearchFieldCentering(from: notification, animated: true)
    }

    private func syncSearchFieldCentering(from notification: Notification, animated: Bool) {
        guard let searchField = notification.object as? NSSearchField else {
            return
        }
        syncSearchFieldCentering(searchField, animated: animated)
    }

    private func syncSearchFieldCentering(_ searchField: NSSearchField, animated: Bool) {
        (searchField as? CenteredPlaceholderSearchField)?.syncCentering(animated: animated)
    }
}

private final class CenteredPlaceholderSearchField: NSSearchField {
    fileprivate var centeringProgress: CGFloat = 1 {
        didSet {
            needsDisplay = true
        }
    }

    private var centeringAnimationTimer: Timer?

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        installCenteredPlaceholderCell()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        installCenteredPlaceholderCell()
    }

    deinit {
        centeringAnimationTimer?.invalidate()
    }

    override func becomeFirstResponder() -> Bool {
        let didBecomeFirstResponder = super.becomeFirstResponder()
        syncCentering(animated: true)
        return didBecomeFirstResponder
    }

    override func resignFirstResponder() -> Bool {
        let didResignFirstResponder = super.resignFirstResponder()
        syncCentering(animated: true)
        return didResignFirstResponder
    }

    fileprivate func syncCentering(animated: Bool) {
        animateCentering(to: shouldCenterPlaceholder ? 1 : 0, animated: animated)
    }

    fileprivate var shouldCenterPlaceholder: Bool {
        stringValue.isEmpty && currentEditor() == nil
    }

    private func installCenteredPlaceholderCell() {
        cell = CenteredPlaceholderSearchFieldCell(textCell: "")
    }

    private func animateCentering(to target: CGFloat, animated: Bool) {
        centeringAnimationTimer?.invalidate()
        centeringAnimationTimer = nil

        let start = centeringProgress
        guard animated, abs(start - target) > 0.001 else {
            centeringProgress = target
            return
        }

        let startTime = Date.timeIntervalSinceReferenceDate
        let duration = 0.16
        let timer = Timer(timeInterval: 1 / 60, repeats: true) { [weak self] timer in
            guard let self else {
                timer.invalidate()
                return
            }

            let elapsed = Date.timeIntervalSinceReferenceDate - startTime
            let progress = min(1, elapsed / duration)
            let easedProgress = 1 - pow(1 - progress, 3)
            centeringProgress = start + (target - start) * CGFloat(easedProgress)

            if progress >= 1 {
                timer.invalidate()
                centeringAnimationTimer = nil
            }
        }

        centeringAnimationTimer = timer
        RunLoop.main.add(timer, forMode: .common)
    }
}

private final class CenteredPlaceholderSearchFieldCell: NSSearchFieldCell {
    override func titleRect(forBounds rect: NSRect) -> NSRect {
        centeredSearchTextRect(forBounds: rect)
    }

    override func drawingRect(forBounds rect: NSRect) -> NSRect {
        centeredSearchTextRect(forBounds: rect)
    }

    override func searchButtonRect(forBounds rect: NSRect) -> NSRect {
        super.searchButtonRect(forBounds: rect)
            .offsetBy(dx: centeringOffset(forBounds: rect), dy: 0)
    }

    override func searchTextRect(forBounds rect: NSRect) -> NSRect {
        centeredSearchTextRect(forBounds: rect)
    }

    private func centeredSearchTextRect(forBounds rect: NSRect) -> NSRect {
        super.searchTextRect(forBounds: rect)
            .offsetBy(dx: centeringOffset(forBounds: rect), dy: 0)
    }

    private func centeringOffset(forBounds rect: NSRect) -> CGFloat {
        guard let searchField = controlView as? CenteredPlaceholderSearchField,
              searchField.centeringProgress > 0,
              searchField.stringValue.isEmpty,
              let placeholderString,
              placeholderString.isEmpty == false else {
            return 0
        }

        let searchButtonRect = super.searchButtonRect(forBounds: rect)
        let textRect = super.searchTextRect(forBounds: rect)
        let font = font ?? NSFont.systemFont(ofSize: NSFont.systemFontSize)
        let textWidth = ceil((placeholderString as NSString).size(
            withAttributes: [.font: font]
        ).width)
        let spacing = max(4, textRect.minX - searchButtonRect.maxX)
        let groupWidth = searchButtonRect.width + spacing + textWidth
        let centeredMinX = floor(rect.midX - groupWidth / 2)
        let offset = max(0, centeredMinX - searchButtonRect.minX)

        return offset * searchField.centeringProgress
    }
}

private extension NSToolbar.Identifier {
    static let automicVaultMain = NSToolbar.Identifier("AutomicVaultMainToolbar")
}

private extension NSToolbarItem.Identifier {
    static let automicVaultSearch = NSToolbarItem.Identifier("AutomicVaultSearch")
    static let automicVaultRefresh = NSToolbarItem.Identifier("AutomicVaultRefresh")
}
