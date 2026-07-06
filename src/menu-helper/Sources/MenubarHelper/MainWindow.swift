import AppKit
import MenubarHelperCore
import Security
import SwiftUI
import UniformTypeIdentifiers

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
        addSplitViewItem(columnItem(DashboardListView(model: model), width: 280, minimumWidth: 168))
        addSplitViewItem(columnItem(DashboardDetailView(model: model), width: 320))
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

    private func columnItem<Content: View>(_ rootView: Content, width: CGFloat, minimumWidth: CGFloat? = nil) -> NSSplitViewItem {
        let minimumWidth = minimumWidth ?? width
        let controller = NSHostingController(rootView: rootView)
        let item = NSSplitViewItem(viewController: controller)
        item.minimumThickness = minimumWidth
        item.preferredThicknessFraction = 0
        controller.view.widthAnchor.constraint(greaterThanOrEqualToConstant: minimumWidth).isActive = true
        let widthConstraint = controller.view.widthAnchor.constraint(equalToConstant: width)
        widthConstraint.priority = .defaultLow
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
    override func performKeyEquivalent(with event: NSEvent) -> Bool {
        guard event.modifierFlags.intersection(.deviceIndependentFlagsMask) == .command,
              let key = event.charactersIgnoringModifiers?.lowercased()
        else {
            return super.performKeyEquivalent(with: event)
        }

        switch key {
        case "w":
            performClose(nil)
            return true
        case "h":
            NSApp.hide(nil)
            return true
        default:
            return super.performKeyEquivalent(with: event)
        }
    }

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
    @Published var isRenamingSecret = false
    @Published var errorMessage: String?
    @Published var selectedItemID: String?
    @Published var searchText = ""

    private var reloadTask: Task<Void, Never>?

    var items: [DashboardItem] {
        let base = switch selectedSection {
        case .detectors:
            detectorItems
        case .hardenedTools:
            snapshot.hardenedTools.map {
                DashboardItem(
                    id: $0.stubPath,
                    title: $0.name,
                    subtitle: $0.targetPath ?? "target unknown",
                    detail: [
                        "Stub: \($0.stubPath)",
                        $0.targetPath.map { "Target: \($0)" },
                    ].compactMap(\.self).joined(separator: "\n"),
                    documentation: $0.documentation
                )
            }
        case .secretGates:
            snapshot.secretGates.map {
                let secrets = $0.keys.count == 1 ? "1 secret" : "\($0.keys.count) secrets"
                let apps = $0.approvedApps.count == 1 ? "1 app" : "\($0.approvedApps.count) apps"
                return DashboardItem(
                    id: $0.id,
                    title: $0.scriptPath,
                    subtitle: "\(secrets) - \(apps)",
                    detail: [
                        "Script: \($0.scriptPath)",
                        "SHA: \($0.scriptChecksum)",
                        "Secrets: \($0.keys.joined(separator: ", "))",
                        "Target: \($0.target)",
                        "Calling apps: \($0.approvedApps.map(\.bundleIdentifier).joined(separator: ", "))",
                    ].joined(separator: "\n")
                )
            }
        case .allSecrets:
            snapshot.secrets.map {
                DashboardItem(id: $0.account, title: $0.account, subtitle: "Keychain secret", detail: "Secret value is hidden.")
            }
        }
        let query = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else { return base }
        return base.filter {
            $0.title.localizedCaseInsensitiveContains(query)
                || $0.subtitle.localizedCaseInsensitiveContains(query)
                || $0.detail.localizedCaseInsensitiveContains(query)
        }
    }

    var selectedItem: DashboardItem? {
        if let selectedItemID, let item = items.first(where: { $0.id == selectedItemID }) {
            return item
        }
        return items.first
    }

    var selectedSecretGate: SecretGate? {
        if let selectedItemID, let gate = snapshot.secretGates.first(where: { $0.id == selectedItemID }) {
            return gate
        }
        return snapshot.secretGates.first
    }

    func count(for section: DashboardSection) -> Int {
        switch section {
        case .detectors: snapshot.detectorDisplayCount
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

    func renameSelectedSecret(to newAccount: String) {
        guard selectedSection == .allSecrets, let account = selectedItem?.id else { return }
        let newAccount = newAccount.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !newAccount.isEmpty, newAccount != account else { return }
        let status = renameStoredSecret(account: account, to: newAccount)
        if status == errSecSuccess {
            errorMessage = nil
            selectedItemID = newAccount
            reload()
        } else {
            errorMessage = "Could not rename \(account): \(status)"
        }
    }

    func addApp(to gate: SecretGate) {
        let panel = NSOpenPanel()
        panel.title = "Allow Calling App"
        panel.prompt = "Allow"
        panel.directoryURL = URL(fileURLWithPath: "/Applications", isDirectory: true)
        panel.allowedContentTypes = [.applicationBundle]
        panel.canChooseFiles = true
        panel.canChooseDirectories = false
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        guard url.pathExtension == "app" else {
            errorMessage = "Choose a .app bundle."
            return
        }
        guard let requirement = appBundleSigning(url)?.requirement else {
            errorMessage = "Could not read code signing identity for \(url.lastPathComponent)."
            return
        }
        let status = rememberTrustedApp(requirement: requirement, for: gate)
        if status == errSecSuccess {
            errorMessage = nil
            reload()
        } else {
            errorMessage = "Could not allow \(url.lastPathComponent): \(status)"
        }
    }

    func remove(_ app: SecretGateApprovedApp, from gate: SecretGate) {
        let status = forgetTrustedApp(app, from: gate)
        if status == errSecSuccess {
            errorMessage = nil
            reload()
        } else {
            errorMessage = "Could not remove \(app.bundleIdentifier): \(status)"
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
                        detail: "",
                        documentation: detector.documentation
                    )
                }
                let severity = findings.map(\.severity).max() ?? "flagged"
                let affectedCount = findings.flatMap(\.affected).count
                let subtitle = affectedCount == 1 ? "1 affected file" : "\(affectedCount) affected files"
                return DashboardItem(
                    id: detector.name,
                    title: detector.name,
                    subtitle: subtitle,
                    detail: [
                        findings.first?.explanation ?? "Detector flagged this tool.",
                        findings.first?.solution,
                    ].compactMap(\.self).joined(separator: "\n\n"),
                    documentation: detector.documentation,
                    severity: severity.uppercased(),
                    isTriggered: true
                )
            }
            .sorted {
                if $0.isTriggered != $1.isTriggered { return $0.isTriggered }
                return $0.title.localizedStandardCompare($1.title) == .orderedAscending
            }
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
        case .allSecrets: "Secrets"
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
    let documentation: String
    let severity: String?
    let isTriggered: Bool

    init(id: String, title: String, subtitle: String, detail: String, documentation: String = "", severity: String? = nil, isTriggered: Bool = false) {
        self.id = id
        self.title = title
        self.subtitle = subtitle
        self.detail = detail
        self.documentation = documentation
        self.severity = severity
        self.isTriggered = isTriggered
    }
}

private struct DashboardSidebarView: View {
    @ObservedObject var model: DashboardModel

    var body: some View {
        VStack(spacing: 8) {
            searchField
                .padding(.horizontal, 12)
                .padding(.top, 8)
            List(selection: sectionSelection) {
                ForEach(DashboardSection.allCases) { section in
                    sidebarRow(section)
                        .tag(section)
                }
            }
            .listStyle(.sidebar)
        }
    }

    private var sectionSelection: Binding<DashboardSection?> {
        Binding {
            model.selectedSection
        } set: { section in
            if let section {
                model.selectSection(section)
            }
        }
    }

    private func sidebarRow(_ section: DashboardSection) -> some View {
        HStack(spacing: 12) {
            sidebarIcon(section)
            Text(section.title)
                .font(.system(size: 14, weight: .regular))
                .lineLimit(1)
            Spacer(minLength: 6)
            let count = model.count(for: section)
            if count > 0 {
                if section == .detectors, model.snapshot.flaggedDetectorCount > 0 {
                    DetectorCountPill(count: count)
                        .fixedSize()
                } else {
                    SidebarCountText(count: count)
                        .fixedSize()
                }
            }
        }
    }

    private var searchField: some View {
        TextField("Search", text: $model.searchText)
            .textFieldStyle(.roundedBorder)
            .font(.system(size: 13))
    }

    private func sidebarIcon(_ section: DashboardSection) -> some View {
        Image(systemName: section.systemImage)
            .font(.system(size: 14, weight: .semibold))
            .frame(width: 20, height: 20)
    }
}

private struct DashboardListView: View {
    @ObservedObject var model: DashboardModel

    var body: some View {
        Group {
            if model.items.isEmpty && !model.isReloading {
                EmptyListView(section: model.selectedSection)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                List(selection: itemSelection) {
                    rows(model.items)
                }
                .listStyle(.inset)
            }
        }
        .ignoresSafeArea()
        .sheet(isPresented: $model.isAddingSecret) {
            AddSecretView(model: model)
        }
    }

    private var itemSelection: Binding<String?> {
        Binding {
            model.selectedItem?.id
        } set: { id in
            model.selectedItemID = id
        }
    }

    private func rows(_ items: [DashboardItem]) -> some View {
        ForEach(items) { item in
            DashboardRow(item: item)
                .tag(item.id)
        }
    }
}

private struct DashboardDetailView: View {
    @ObservedObject var model: DashboardModel

    var body: some View {
        ScrollView {
            if model.selectedSection == .secretGates, let gate = model.selectedSecretGate {
                SecretGateDetailView(model: model, gate: gate)
                    .padding(.horizontal, 22)
                    .padding(.top, 32)
                    .padding(.bottom, 28)
                    .frame(maxWidth: .infinity, alignment: .leading)
            } else if model.selectedSection == .detectors, let item = model.selectedItem {
                ReferenceDetailView(
                    item: item,
                    summary: "Detector behavior and sensitive files checked by this rule.",
                    referenceTitle: "Detector Reference",
                    fallbackDocumentation: "No detector documentation is bundled for this item.",
                    badge: item.isTriggered
                        ? ReferenceBadge(title: "Flagged", color: .red)
                        : ReferenceBadge(title: "Ready", color: .green)
                )
                    .padding(.horizontal, 22)
                    .padding(.top, 32)
                    .padding(.bottom, 28)
                    .frame(maxWidth: .infinity, alignment: .leading)
            } else if model.selectedSection == .hardenedTools, let item = model.selectedItem {
                ReferenceDetailView(
                    item: item,
                    summary: "Installed hardening behavior and caveats for this tool.",
                    referenceTitle: "Hardener Reference",
                    fallbackDocumentation: "No hardener documentation is bundled for this item.",
                    badge: ReferenceBadge(title: "Hardened", color: .blue)
                )
                    .padding(.horizontal, 22)
                    .padding(.top, 32)
                    .padding(.bottom, 28)
                    .frame(maxWidth: .infinity, alignment: .leading)
            } else if let item = model.selectedItem {
                VStack(alignment: .leading, spacing: 18) {
                    Text(item.title)
                        .font(.system(size: 24, weight: .semibold))
                        .foregroundStyle(.primary)
                        .lineLimit(3)
                    Text(item.subtitle)
                        .font(.system(size: 14))
                        .foregroundStyle(.secondary)
                    InfoBlock(title: model.selectedSection.title, text: item.detail)
                    if model.selectedSection == .allSecrets {
                        HStack {
                            Button { model.isRenamingSecret = true } label: {
                                Label("Rename Secret", systemImage: "pencil")
                                    .frame(maxWidth: .infinity)
                            }
                            .buttonStyle(.borderedProminent)
                            .controlSize(.large)
                            Button { model.deleteSelectedSecret() } label: {
                                Label("Delete Secret", systemImage: "trash")
                                    .frame(maxWidth: .infinity)
                            }
                            .buttonStyle(.borderedProminent)
                            .controlSize(.large)
                            .tint(.red)
                        }
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
        .background(.ultraThinMaterial)
        .sheet(isPresented: $model.isRenamingSecret) {
            if let account = model.selectedItem?.id {
                RenameSecretView(model: model, account: account)
            }
        }
    }
}

private struct DashboardRow: View {
    let item: DashboardItem

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack(spacing: 6) {
                Text(item.title)
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(.primary)
                    .lineLimit(1)
                if let severity = item.severity {
                    Text(severity)
                        .font(.system(size: 10, weight: .bold))
                        .padding(.horizontal, 6)
                        .frame(height: 18)
                        .outlinedPill()
                }
            }
            Text(item.subtitle)
                .font(.system(size: 12))
                .foregroundStyle(.secondary)
                .lineLimit(2)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.vertical, 4)
        .frame(height: 54, alignment: .topLeading)
    }
}

private struct EmptyListView: View {
    let section: DashboardSection

    var body: some View {
        Text(emptyText)
            .font(.system(size: 13, weight: .medium))
            .foregroundStyle(.secondary)
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
                .foregroundStyle(.primary)
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
        .background(.ultraThinMaterial)
    }
}

private struct RenameSecretView: View {
    @ObservedObject var model: DashboardModel
    @State private var account: String
    @Environment(\.dismiss) private var dismiss

    init(model: DashboardModel, account: String) {
        self.model = model
        _account = State(initialValue: account)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Rename Secret")
                .font(.system(size: 18, weight: .semibold))
                .foregroundStyle(.primary)
            TextField("Name", text: $account)
                .textFieldStyle(.roundedBorder)
            HStack {
                Spacer()
                Button("Cancel") { dismiss() }
                Button("Rename") {
                    model.renameSelectedSecret(to: account)
                    dismiss()
                }
                .keyboardShortcut(.defaultAction)
                .disabled(account.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }
        .padding(22)
        .frame(width: 360)
        .background(.ultraThinMaterial)
    }
}

private enum SidebarCountMetrics {
    static let columnWidth: CGFloat = 18
    static let pillHorizontalPadding: CGFloat = 8
}

private struct SidebarCountText: View {
    let count: Int

    var body: some View {
        Text(count.formatted())
            .font(.system(size: 11, weight: .regular))
            .foregroundStyle(.secondary)
            .monospacedDigit()
            .lineLimit(1)
            .frame(minWidth: SidebarCountMetrics.columnWidth, alignment: .trailing)
    }
}

private struct DetectorCountPill: View {
    let count: Int

    var body: some View {
        Text(count.formatted())
            .font(.system(size: 11, weight: .medium))
            .monospacedDigit()
            .padding(.horizontal, 8)
            .frame(height: 20)
            .outlinedPill()
            .padding(.trailing, -SidebarCountMetrics.pillHorizontalPadding)
    }
}

private struct InfoBlock: View {
    let title: String
    let text: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title.uppercased())
                .font(.system(size: 11, weight: .bold))
                .foregroundStyle(.secondary)
                .tracking(0.7)
            Text(text)
                .font(.system(size: 12))
                .foregroundStyle(.secondary)
                .textSelection(.enabled)
        }
    }
}

private struct ReferenceBadge {
    let title: String
    let color: Color
}

private struct ReferenceDetailView: View {
    let item: DashboardItem
    let summary: String
    let referenceTitle: String
    let fallbackDocumentation: String
    let badge: ReferenceBadge

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            VStack(alignment: .leading, spacing: 8) {
                HStack(alignment: .firstTextBaseline, spacing: 10) {
                    Text(item.title)
                        .font(.system(size: 26, weight: .semibold))
                        .foregroundStyle(.primary)
                        .lineLimit(3)
                    referenceBadge
                }
                Text(summary)
                    .font(.system(size: 13))
                    .foregroundStyle(.secondary)
            }

            if !item.detail.isEmpty {
                InfoBlock(title: "Current Result", text: item.detail)
                    .padding(14)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background {
                        RoundedRectangle(cornerRadius: 8, style: .continuous)
                            .fill(Color(nsColor: .controlBackgroundColor))
                    }
            }

            VStack(alignment: .leading, spacing: 14) {
                Text(referenceTitle)
                    .font(.system(size: 12, weight: .bold))
                    .foregroundStyle(.secondary)
                    .tracking(0.7)
                RenderedMarkdown(markdown: item.documentation.isEmpty ? fallbackDocumentation : item.documentation)
                    .font(.system(size: 13))
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .padding(16)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background {
                RoundedRectangle(cornerRadius: 8, style: .continuous)
                    .fill(Color(nsColor: .controlBackgroundColor))
            }
            .overlay {
                RoundedRectangle(cornerRadius: 8, style: .continuous)
                    .stroke(Color(nsColor: .separatorColor))
            }
        }
    }

    private var referenceBadge: some View {
        Text(badge.title)
            .font(.system(size: 11, weight: .semibold))
            .padding(.horizontal, 8)
            .frame(height: 20)
            .outlinedPill(badge.color)
    }
}

private struct SecretGateDetailView: View {
    @ObservedObject var model: DashboardModel
    let gate: SecretGate

    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            VStack(alignment: .leading, spacing: 6) {
                Text(URL(fileURLWithPath: gate.scriptPath).lastPathComponent)
                    .font(.system(size: 24, weight: .semibold))
                    .foregroundStyle(.primary)
                    .lineLimit(3)
                Text("\(countLabel(gate.keys.count, "secret")) allowed for \(countLabel(gate.approvedApps.count, "calling app"))")
                    .font(.system(size: 13))
                    .foregroundStyle(.secondary)
            }

            VStack(alignment: .leading, spacing: 10) {
                SecretGateField("Script", gate.scriptPath)
                SecretGateField("SHA", gate.scriptChecksum, monospaced: true)
                SecretGateField("Secrets", gate.keys.joined(separator: ", "))
                SecretGateField("Target", gate.target)
                SecretGateField("Replace Existing Env", gate.replaceExistingEnv ? "Yes" : "No")
                SecretGateField("Allow Missing Keys", gate.allowMissingKeys ? "Yes" : "No")
            }

            VStack(alignment: .leading, spacing: 10) {
                HStack {
                    Text("Always Approved Apps")
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(.primary)
                    Spacer()
                    Button { model.addApp(to: gate) } label: {
                        Image(systemName: "plus")
                            .frame(width: 20, height: 20)
                    }
                    .buttonStyle(.plain)
                    .help("Add Calling App")
                }

                if gate.approvedApps.isEmpty {
                    Text("No apps are always approved for this gate.")
                        .font(.system(size: 12))
                        .foregroundStyle(.secondary)
                } else {
                    VStack(spacing: 0) {
                        ForEach(gate.approvedApps, id: \.requirement) { app in
                            ApprovedAppRow(app: app) {
                                model.remove(app, from: gate)
                            }
                            if app.requirement != gate.approvedApps.last?.requirement {
                                hairline
                            }
                        }
                    }
                }
            }

            if let error = model.errorMessage {
                InfoBlock(title: "Error", text: error)
            }
        }
    }

    private func countLabel(_ count: Int, _ singular: String) -> String {
        count == 1 ? "1 \(singular)" : "\(count) \(singular)s"
    }
}

private struct SecretGateField: View {
    let label: String
    let value: String
    let monospaced: Bool

    init(_ label: String, _ value: String, monospaced: Bool = false) {
        self.label = label
        self.value = value
        self.monospaced = monospaced
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(label.uppercased())
                .font(.system(size: 10, weight: .bold))
                .foregroundStyle(.secondary)
            Text(value)
                .font(monospaced ? .system(size: 12, design: .monospaced) : .system(size: 12))
                .foregroundStyle(.secondary)
                .textSelection(.enabled)
        }
    }
}

private struct ApprovedAppRow: View {
    let app: SecretGateApprovedApp
    let remove: () -> Void

    private var display: ApprovedAppDisplay {
        ApprovedAppDisplay(app)
    }

    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Image(nsImage: display.icon)
                .resizable()
                .frame(width: 34, height: 34)
            VStack(alignment: .leading, spacing: 4) {
                Text(display.name)
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(.primary)
                    .lineLimit(1)
                Text(display.bundleIdentifier)
                    .font(.system(size: 12))
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
                Text(display.signingSummary)
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
                    .textSelection(.enabled)
            }
            Spacer(minLength: 8)
            Button(action: remove) {
                Image(systemName: "minus")
                    .frame(width: 20, height: 20)
            }
            .buttonStyle(.plain)
            .foregroundStyle(.secondary)
            .help("Remove Calling App")
        }
        .padding(.vertical, 10)
    }
}

private struct ApprovedAppDisplay {
    let name: String
    let bundleIdentifier: String
    let icon: NSImage
    let signingSummary: String

    init(_ app: SecretGateApprovedApp) {
        let url = NSWorkspace.shared.urlForApplication(withBundleIdentifier: app.bundleIdentifier)
        let bundle = url.flatMap(Bundle.init(url:))
        name = bundle?.object(forInfoDictionaryKey: "CFBundleDisplayName") as? String
            ?? bundle?.object(forInfoDictionaryKey: "CFBundleName") as? String
            ?? url?.deletingPathExtension().lastPathComponent
            ?? app.bundleIdentifier
        bundleIdentifier = app.bundleIdentifier
        icon = url.map { NSWorkspace.shared.icon(forFile: $0.path) } ?? NSImage(systemSymbolName: "app", accessibilityDescription: nil) ?? NSImage()
        if let signing = url.flatMap(appBundleSigning) {
            signingSummary = "identifier \(signing.identifier) / team \(signing.teamIdentifier)\n\(app.requirement)"
        } else {
            signingSummary = app.requirement
        }
    }
}

private struct AppBundleSigning {
    let identifier: String
    let teamIdentifier: String
    let requirement: String
}

private func appBundleSigning(_ url: URL) -> AppBundleSigning? {
    var staticCode: SecStaticCode?
    guard SecStaticCodeCreateWithPath(url as CFURL, [], &staticCode) == errSecSuccess,
          let staticCode
    else {
        return nil
    }

    var info: CFDictionary?
    let flags = SecCSFlags(rawValue: kSecCSSigningInformation | kSecCSRequirementInformation)
    guard SecCodeCopySigningInformation(staticCode, flags, &info) == errSecSuccess,
          let dictionary = info as? [CFString: Any],
          let requirementValue = dictionary[kSecCodeInfoDesignatedRequirement]
    else {
        return nil
    }
    let requirement = requirementValue as! SecRequirement
    guard let requirementText = requirementText(requirement) else {
        return nil
    }
    return AppBundleSigning(
        identifier: dictionary[kSecCodeInfoIdentifier] as? String ?? "unknown",
        teamIdentifier: dictionary[kSecCodeInfoTeamIdentifier] as? String ?? "unknown",
        requirement: requirementText
    )
}

private func requirementText(_ requirement: SecRequirement) -> String? {
    var text: CFString?
    guard SecRequirementCopyString(requirement, [], &text) == errSecSuccess,
          let text
    else {
        return nil
    }
    return text as String
}

private extension View {
    func outlinedPill(_ color: Color = .red) -> some View {
        foregroundStyle(color)
            .background(color.opacity(0.12), in: Capsule())
            .overlay {
                Capsule().stroke(color, lineWidth: 1)
            }
    }
}

private var hairline: some View {
    Rectangle().fill(Color(nsColor: .separatorColor)).frame(height: 1)
}
