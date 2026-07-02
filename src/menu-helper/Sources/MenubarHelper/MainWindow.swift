import AppKit
import MenubarHelperCore
import Security
import SwiftUI

@MainActor
final class AutomicVaultMainWindowController: NSSplitViewController {
    private let model = DashboardModel()

    init() {
        super.init(nibName: nil, bundle: nil)
        splitView = NoDividerSplitView()
    }

    @MainActor @preconcurrency required dynamic init?(coder: NSCoder) {
        super.init(coder: coder)
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        splitView.isVertical = true
        addSplitViewItem(sidebarItem())
        addSplitViewItem(contentItem(DashboardListView(model: model), width: 300, minimumWidth: 260, maximumWidth: 360))
        addSplitViewItem(contentItem(DashboardDetailView(model: model), width: 470, minimumWidth: 320))
    }

    override func viewDidAppear() {
        super.viewDidAppear()
        model.reload()
    }

    func makeToolbar() -> NSToolbar {
        let toolbar = NSToolbar(identifier: "AutomicVaultToolbar")
        toolbar.displayMode = .iconOnly
        toolbar.delegate = self
        return toolbar
    }

    @objc func refresh(_ sender: Any?) {
        model.reload()
    }

    private func sidebarItem() -> NSSplitViewItem {
        let controller = NSHostingController(rootView: DashboardSidebarView(model: model))
        let item = NSSplitViewItem(sidebarWithViewController: controller)
        item.minimumThickness = 250
        item.maximumThickness = 250
        return item
    }

    private func contentItem<Content: View>(_ rootView: Content, width: CGFloat, minimumWidth: CGFloat, maximumWidth: CGFloat? = nil) -> NSSplitViewItem {
        let controller = NSHostingController(rootView: rootView)
        let item = NSSplitViewItem(viewController: controller)
        item.minimumThickness = minimumWidth
        if let maximumWidth {
            item.maximumThickness = maximumWidth
        }
        item.preferredThicknessFraction = 0
        controller.view.widthAnchor.constraint(greaterThanOrEqualToConstant: minimumWidth).isActive = true
        let widthConstraint = controller.view.widthAnchor.constraint(equalToConstant: width)
        widthConstraint.priority = maximumWidth == nil ? .defaultLow : .required
        widthConstraint.isActive = true
        return item
    }
}

extension AutomicVaultMainWindowController: NSToolbarDelegate {
    func toolbarAllowedItemIdentifiers(_ toolbar: NSToolbar) -> [NSToolbarItem.Identifier] {
        [.refresh, .flexibleSpace]
    }

    func toolbarDefaultItemIdentifiers(_ toolbar: NSToolbar) -> [NSToolbarItem.Identifier] {
        [.flexibleSpace, .refresh]
    }

    func toolbar(_ toolbar: NSToolbar, itemForItemIdentifier itemIdentifier: NSToolbarItem.Identifier, willBeInsertedIntoToolbar flag: Bool) -> NSToolbarItem? {
        guard itemIdentifier == .refresh else { return nil }
        let item = NSToolbarItem(itemIdentifier: itemIdentifier)
        item.image = NSImage(systemSymbolName: "arrow.clockwise", accessibilityDescription: "Refresh")
        item.label = "Refresh"
        item.target = self
        item.action = #selector(refresh(_:))
        return item
    }
}

private extension NSToolbarItem.Identifier {
    static let refresh = NSToolbarItem.Identifier("AutomicVaultRefreshToolbarItem")
}

private final class NoDividerSplitView: NSSplitView {
    override var dividerThickness: CGFloat { 0 }
    override func drawDivider(in rect: NSRect) {}
}

final class AutomicVaultWindow: NSWindow {
    override func cancelOperation(_ sender: Any?) {
        makeFirstResponder(nil)
    }

    override func sendEvent(_ event: NSEvent) {
        if event.type == .leftMouseDown, firstResponder is NSText {
            makeFirstResponder(nil)
        }
        super.sendEvent(event)
    }
}

@MainActor
final class DashboardModel: ObservableObject {
    @Published var selectedSection: DashboardSection = .detectors
    @Published private(set) var snapshot = DashboardSnapshot.empty
    @Published private(set) var isReloading = false
    @Published var isAddingSecret = false
    @Published var errorMessage: String?
    @Published var selectedItemID: String?

    private var reloadTask: Task<Void, Never>?

    var items: [DashboardItem] {
        switch selectedSection {
        case .detectors:
            detectorItems
        case .hardenedTools:
            snapshot.hardenedTools.map {
                DashboardItem(id: $0.stubPath, title: $0.name, subtitle: $0.targetPath ?? "target unknown", detail: $0.stubPath)
            }
        case .secretGates:
            snapshot.secretGates.map {
                let apps = $0.approvedApps.isEmpty ? "No approved apps recorded" : "Approved for: \($0.approvedApps.joined(separator: ", "))"
                return DashboardItem(id: $0.scriptPath + $0.target, title: URL(fileURLWithPath: $0.scriptPath).lastPathComponent, subtitle: $0.target, detail: "Keys: \($0.keys.joined(separator: ", "))\n\(apps)")
            }
        case .allSecrets:
            snapshot.secrets.map {
                DashboardItem(id: $0.account, title: $0.account, subtitle: "Keychain secret", detail: "Secret value is hidden.")
            }
        }
    }

    var selectedItem: DashboardItem? {
        if let selectedItemID, let item = items.first(where: { $0.id == selectedItemID }) {
            return item
        }
        return items.first
    }

    func count(for section: DashboardSection) -> Int {
        switch section {
        case .detectors: snapshot.flaggedDetectorCount
        case .hardenedTools: snapshot.hardenedTools.count
        case .secretGates: snapshot.secretGates.count
        case .allSecrets: snapshot.secrets.count
        }
    }

    func selectSection(_ section: DashboardSection) {
        selectedSection = section
        selectedItemID = nil
    }

    func select(_ item: DashboardItem) {
        selectedItemID = item.id
    }

    func reload() {
        reloadTask?.cancel()
        isReloading = true
        reloadTask = Task {
            let next = await Task.detached(priority: .background) {
                DashboardSnapshot.load()
            }.value
            guard !Task.isCancelled else { return }
            snapshot = next
            if selectedItemID.map({ id in !items.contains { $0.id == id } }) == true {
                selectedItemID = nil
            }
            isReloading = false
        }
    }

    func addSecret(account: String, value: String) {
        let account = account.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !account.isEmpty, !value.isEmpty else { return }
        let status = saveStoredSecret(account: account, value: value)
        if status == errSecSuccess {
            selectedSection = .allSecrets
            selectedItemID = account
            reload()
        } else {
            errorMessage = "Could not save \(account): \(status)"
        }
    }

    func deleteSelectedSecret() {
        guard selectedSection == .allSecrets, let account = selectedItem?.id else { return }
        let status = deleteStoredSecret(account: account)
        if status == errSecSuccess || status == errSecItemNotFound {
            selectedItemID = nil
            reload()
        } else {
            errorMessage = "Could not delete \(account): \(status)"
        }
    }

    private var detectorItems: [DashboardItem] {
        let findingsBySource = Dictionary(grouping: snapshot.detectorFindings, by: \.source)
        let detectors = snapshot.detectors.isEmpty
            ? findingsBySource.keys.map { DetectorMetadata(name: $0, homepage: "", docsURL: "") }
            : snapshot.detectors

        return detectors
            .map { detector in
                let findings = findingsBySource[detector.name] ?? []
                guard !findings.isEmpty else {
                    return DashboardItem(
                        id: detector.name,
                        title: detector.name,
                        subtitle: "No findings",
                        detail: detectorInfo(detector)
                    )
                }
                let severity = findings.map(\.severity).max() ?? "flagged"
                let affectedCount = findings.flatMap(\.affected).count
                let subtitle = affectedCount == 1 ? "1 affected file" : "\(affectedCount) affected files"
                return DashboardItem(
                    id: detector.name,
                    title: detector.name,
                    subtitle: "\(severity.uppercased()) - \(subtitle)",
                    detail: [
                        findings.first?.explanation ?? "Detector flagged this tool.",
                        findings.first?.solution,
                        detectorInfo(detector),
                    ].compactMap(\.self).joined(separator: "\n\n")
                )
            }
            .sorted { $0.title.localizedStandardCompare($1.title) == .orderedAscending }
    }

    private func detectorInfo(_ detector: DetectorMetadata) -> String {
        [
            detector.homepage.isEmpty ? nil : "Homepage: \(detector.homepage)",
            detector.docsURL.isEmpty ? nil : "Docs: \(detector.docsURL)",
        ].compactMap(\.self).joined(separator: "\n")
    }
}

enum DashboardSection: String, CaseIterable, Identifiable {
    case detectors
    case hardenedTools
    case secretGates
    case allSecrets

    var id: String { rawValue }

    var title: String {
        switch self {
        case .detectors: "Detectors"
        case .hardenedTools: "Hardened Tools"
        case .secretGates: "Secret Gates"
        case .allSecrets: "All Secrets"
        }
    }

    var systemImage: String {
        switch self {
        case .detectors: "sensor.tag.radiowaves.forward"
        case .hardenedTools: "hammer"
        case .secretGates: "lock.shield"
        case .allSecrets: "key"
        }
    }
}

struct DashboardItem: Identifiable, Equatable {
    let id: String
    let title: String
    let subtitle: String
    let detail: String
}

private struct DashboardSidebarView: View {
    @ObservedObject var model: DashboardModel

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("AUTOMIC VAULT")
                .font(.system(size: 11, weight: .medium))
                .foregroundStyle(GlassPalette.quietText)
                .tracking(0.5)
                .padding(.horizontal, 20)
                .padding(.top, 18)
                .padding(.bottom, 10)
            ForEach(DashboardSection.allCases) { section in
                sidebarRow(section)
            }
            Spacer()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(GlassSurface(tint: GlassPalette.sidebarTint).ignoresSafeArea())
        .preferredColorScheme(.dark)
    }

    private func sidebarRow(_ section: DashboardSection) -> some View {
        Button { model.selectSection(section) } label: {
            HStack(spacing: 12) {
                Image(systemName: section.systemImage)
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(.white)
                    .frame(width: 22, height: 22)
                    .background(iconFill(for: section), in: RoundedRectangle(cornerRadius: 6, style: .continuous))
                Text(section.title)
                    .font(.system(size: 14))
                    .lineLimit(1)
                Spacer(minLength: 6)
                let count = model.count(for: section)
                if count > 0 {
                    CountPill(count: count, isWarning: section == .detectors)
                }
            }
            .foregroundStyle(model.selectedSection == section ? GlassPalette.primaryText : GlassPalette.secondaryText)
            .padding(.horizontal, 10)
            .frame(maxWidth: .infinity, minHeight: 36, alignment: .leading)
            .background {
                if model.selectedSection == section {
                    RoundedRectangle(cornerRadius: 8, style: .continuous).fill(GlassPalette.sidebarSelectedFill)
                }
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .padding(.horizontal, 12)
    }

    private func iconFill(for section: DashboardSection) -> Color {
        switch section {
        case .detectors: GlassPalette.red
        case .hardenedTools: GlassPalette.blue
        case .secretGates: GlassPalette.gray
        case .allSecrets: GlassPalette.green
        }
    }
}

private struct DashboardListView: View {
    @ObservedObject var model: DashboardModel

    var body: some View {
        ZStack(alignment: .top) {
            ScrollView {
                LazyVStack(spacing: 0) {
                    if model.items.isEmpty && !model.isReloading {
                        EmptyListView(section: model.selectedSection)
                            .frame(maxWidth: .infinity, minHeight: 180)
                            .padding(.top, 43)
                    } else {
                        ForEach(model.items) { item in
                            DashboardRow(item: item, selected: model.selectedItem?.id == item.id) {
                                model.select(item)
                            }
                        }
                    }
                }
                .padding(.top, 43)
            }
            VStack(spacing: 0) {
                HStack {
                    Text(model.selectedSection.title)
                    Spacer()
                    if model.selectedSection == .allSecrets {
                        Button { model.isAddingSecret = true } label: {
                            Image(systemName: "plus")
                        }
                        .buttonStyle(.plain)
                        .help("Add Secret")
                    }
                    if model.isReloading { ProgressView().controlSize(.small) }
                }
                .font(.system(size: 13, weight: .bold))
                .foregroundStyle(GlassPalette.quietText)
                .padding(.horizontal, 12)
                .frame(height: 42)
                .background(GlassSurface(tint: GlassPalette.topBarTint).ignoresSafeArea(.container, edges: .top))
                hairline
            }
        }
        .ignoresSafeArea(.container, edges: .top)
        .background(GlassSurface(tint: GlassPalette.windowTint).ignoresSafeArea())
        .sheet(isPresented: $model.isAddingSecret) {
            AddSecretView(model: model)
        }
        .preferredColorScheme(.dark)
    }
}

private struct DashboardDetailView: View {
    @ObservedObject var model: DashboardModel

    var body: some View {
        ScrollView {
            if let item = model.selectedItem {
                VStack(alignment: .leading, spacing: 18) {
                    Text(item.title)
                        .font(.system(size: 24, weight: .semibold))
                        .foregroundStyle(GlassPalette.primaryText)
                        .lineLimit(3)
                    Text(item.subtitle)
                        .font(.system(size: 14))
                        .foregroundStyle(GlassPalette.secondaryText)
                    InfoBlock(title: model.selectedSection.title, text: item.detail)
                    if model.selectedSection == .allSecrets {
                        Button { model.deleteSelectedSecret() } label: {
                            Label("Delete Secret", systemImage: "trash")
                                .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(.borderedProminent)
                        .controlSize(.large)
                        .tint(.red)
                    }
                    if let error = model.errorMessage {
                        InfoBlock(title: "Error", text: error)
                    }
                }
                .padding(.horizontal, 22)
                .padding(.top, 32)
                .padding(.bottom, 28)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .ignoresSafeArea(.container, edges: .top)
        .background(GlassSurface(tint: GlassPalette.windowTint).ignoresSafeArea())
        .preferredColorScheme(.dark)
    }
}

private struct DashboardRow: View {
    let item: DashboardItem
    let selected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            VStack(alignment: .leading, spacing: 4) {
                Text(item.title)
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(GlassPalette.primaryText)
                    .lineLimit(1)
                Text(item.subtitle)
                    .font(.system(size: 12))
                    .foregroundStyle(GlassPalette.quietText)
                    .lineLimit(2)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 8)
            .padding(.vertical, 8)
            .frame(height: 58, alignment: .topLeading)
            .background {
                if selected {
                    RoundedRectangle(cornerRadius: 8, style: .continuous).fill(GlassPalette.packageSelectedFill)
                }
            }
            .padding(.horizontal, 2)
            .padding(.vertical, 2)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}

private struct EmptyListView: View {
    let section: DashboardSection

    var body: some View {
        Text(emptyText)
            .font(.system(size: 13, weight: .medium))
            .foregroundStyle(GlassPalette.quietText)
    }

    private var emptyText: String {
        switch section {
        case .detectors: "No flagged detectors"
        case .hardenedTools: "No hardened tools"
        case .secretGates: "No remembered gates"
        case .allSecrets: "No stored secrets"
        }
    }
}

private struct AddSecretView: View {
    @ObservedObject var model: DashboardModel
    @State private var account = ""
    @State private var value = ""
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Add Secret")
                .font(.system(size: 18, weight: .semibold))
                .foregroundStyle(GlassPalette.primaryText)
            TextField("Name", text: $account)
                .textFieldStyle(.roundedBorder)
            SecureField("Value", text: $value)
                .textFieldStyle(.roundedBorder)
            HStack {
                Spacer()
                Button("Cancel") { dismiss() }
                Button("Save") {
                    model.addSecret(account: account, value: value)
                    dismiss()
                }
                .keyboardShortcut(.defaultAction)
                .disabled(account.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || value.isEmpty)
            }
        }
        .padding(22)
        .frame(width: 360)
        .background(GlassSurface(tint: GlassPalette.windowTint))
        .preferredColorScheme(.dark)
    }
}

private struct CountPill: View {
    let count: Int
    let isWarning: Bool

    var body: some View {
        Text(count.formatted())
            .font(.system(size: 11, weight: .medium))
            .foregroundStyle(.white)
            .monospacedDigit()
            .padding(.horizontal, 8)
            .frame(height: 20)
            .background(isWarning ? GlassPalette.red : GlassPalette.controlFill, in: Capsule())
    }
}

private struct InfoBlock: View {
    let title: String
    let text: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title.uppercased())
                .font(.system(size: 11, weight: .bold))
                .foregroundStyle(GlassPalette.quietText)
                .tracking(0.7)
            Text(text)
                .font(.system(size: 12))
                .foregroundStyle(GlassPalette.secondaryText)
                .textSelection(.enabled)
        }
    }
}

private struct GlassSurface: View {
    let tint: Color

    var body: some View {
        Rectangle().fill(.ultraThinMaterial).overlay(tint)
    }
}

private var hairline: some View {
    Rectangle().fill(GlassPalette.hairline).frame(height: 1)
}

private enum GlassPalette {
    static let windowTint = Color(red: 0.05, green: 0.06, blue: 0.07).opacity(0.50)
    static let topBarTint = Color(red: 0.07, green: 0.08, blue: 0.09).opacity(0.36)
    static let sidebarTint = Color(red: 0.06, green: 0.07, blue: 0.07).opacity(0.72)
    static let primaryText = Color.white.opacity(0.92)
    static let secondaryText = Color.white.opacity(0.72)
    static let quietText = Color.white.opacity(0.42)
    static let hairline = Color.white.opacity(0.07)
    static let sidebarSelectedFill = Color(red: 0.00, green: 0.38, blue: 0.86)
    static let packageSelectedFill = Color.white.opacity(0.08)
    static let controlFill = Color.white.opacity(0.18)
    static let red = Color(red: 0.95, green: 0.18, blue: 0.16)
    static let blue = Color(red: 0.00, green: 0.48, blue: 1.00)
    static let green = Color(red: 0.18, green: 0.62, blue: 0.31)
    static let gray = Color(red: 0.46, green: 0.49, blue: 0.53)
}
