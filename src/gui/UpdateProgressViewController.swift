import AppKit
import SwiftUI

enum PackageStage: String {
    case queued = "Queued"
    case resolving = "Resolving"
    case downloading = "Downloading"
    case extracting = "Extracting"
    case installing = "Installing"
    case completed = "Complete"
    case failed = "Failed"

    var title: String {
        switch self {
        case .queued:
            return L10n.string("Queued")
        case .resolving:
            return L10n.string("Resolving")
        case .downloading:
            return L10n.string("Downloading")
        case .extracting:
            return L10n.string("Extracting")
        case .installing:
            return L10n.string("Installing")
        case .completed:
            return L10n.string("Complete")
        case .failed:
            return L10n.string("Failed")
        }
    }

    var systemImage: String {
        switch self {
        case .queued:
            return "circle.dotted"
        case .resolving:
            return "point.3.connected.trianglepath.dotted"
        case .downloading:
            return "arrow.down.circle"
        case .extracting:
            return "archivebox"
        case .installing:
            return "shippingbox"
        case .completed:
            return "checkmark.circle.fill"
        case .failed:
            return "exclamationmark.triangle.fill"
        }
    }

    var tint: Color {
        switch self {
        case .queued:
            return UpdateProgressPalette.quietText
        case .resolving:
            return UpdateProgressPalette.blue
        case .downloading, .extracting, .installing:
            return UpdateProgressPalette.cyan
        case .completed:
            return UpdateProgressPalette.green
        case .failed:
            return UpdateProgressPalette.red
        }
    }
}

private enum PackageDisplayLifetime {
    case planned
    case discovered
}

private struct PackageDisplayItem {
    let id: String
    let addedIndex: Int
    var lifetime: PackageDisplayLifetime
}

private struct PackageRuntimeState {
    var stage: PackageStage = .queued
    var progress: Double = 0.02
    var speed: String?
    var lastDownloadUpdateAt: Date?
    var lastDownloadProgress = 0.0
    var didLogDownloadStart = false
    var observedDownload = false
}

struct PackageProgressRowState: Identifiable, Equatable {
    let id: String
    var stage: PackageStage
    var progress: Double
    var speed: String?
}

struct ProgressLogEntry: Identifiable, Equatable {
    let id = UUID()
    let timestamp: String
    let message: String
}

@MainActor
final class UpdateProgressViewModel: ObservableObject {
    private enum ProgressLayout {
        static let queued = 0.02
        static let resolving = 0.04
        static let downloadFloor = 0.06
        static let downloadCeiling = 0.78
        static let extractFloor = 0.82
        static let extractCeiling = 0.92
        static let installFloor = 0.84
    }

    @Published var title = L10n.string("Update All")
    @Published var operation = L10n.string("Waiting for helper authorization")
    @Published var status = L10n.string("Ready")
    @Published var rows: [PackageProgressRowState] = []
    @Published var logs: [ProgressLogEntry] = []
    @Published var primaryTitle = L10n.string("Updating")
    @Published var primaryEnabled = false
    @Published var showSecondary = false
    @Published var isTerminalState = false
    @Published var terminalStage: PackageStage?

    private var displayItems: [String: PackageDisplayItem] = [:]
    private var nextDisplayIndex = 0
    private var acceptsDiscoveredDisplayItems = true
    private var packageStates: [String: PackageRuntimeState] = [:]
    private var successOperationTitle = L10n.string("Update Complete")
    private var failureOperationTitle = L10n.string("Update Halted")
    private var activePrimaryTitle = L10n.string("Updating")
    private var packageCountLabel: (Int) -> String =
        UpdateProgressViewModel.outdatedPackageCountText
    private var hasLoggedResolving = false
    private var lastActivePackage: String?

    var completedCount: Int {
        rows.filter { $0.stage == .completed }.count
    }

    var failedCount: Int {
        rows.filter { $0.stage == .failed }.count
    }

    var totalCount: Int {
        rows.count
    }

    var overallProgress: Double {
        guard rows.isEmpty == false else {
            return isTerminalState && terminalStage == .completed ? 1 : 0
        }
        return rows.reduce(0) { $0 + $1.progress } / Double(rows.count)
    }

    var progressSummary: String {
        if failedCount > 0 {
            return failedCount == 1
                ? L10n.string("1 failed")
                : L10n.format("%d failed", failedCount)
        }
        if totalCount == 0 {
            return L10n.string("Preparing package plan")
        }
        return L10n.format("%d of %d complete", completedCount, totalCount)
    }

    var statusTint: Color {
        if terminalStage == .failed {
            return UpdateProgressPalette.red
        }
        if terminalStage == .completed {
            return UpdateProgressPalette.green
        }
        return UpdateProgressPalette.cyan
    }

    var primarySystemImage: String {
        if primaryTitle == L10n.string("Retry") {
            return "arrow.clockwise"
        }
        if primaryTitle == L10n.string("Dismiss") {
            return "checkmark"
        }
        return "arrow.triangle.2.circlepath"
    }

    func configure(
        title: String,
        awaitingClearance: String,
        idleStatus: String,
        successOperation: String,
        failureOperation: String,
        activePrimaryTitle: String = L10n.string("Updating"),
        packageCountLabel: ((Int) -> String)? = nil
    ) {
        self.title = title
        operation = awaitingClearance
        status = idleStatus
        successOperationTitle = successOperation
        failureOperationTitle = failureOperation
        self.activePrimaryTitle = activePrimaryTitle
        self.packageCountLabel = packageCountLabel ?? Self.outdatedPackageCountText
        terminalStage = nil
    }

    func begin(
        packages: [String],
        activationLog: String,
        initialOperation: String?
    ) {
        displayItems = [:]
        nextDisplayIndex = 0
        acceptsDiscoveredDisplayItems = packages.isEmpty
        packageStates = [:]
        packages.forEach {
            addDisplayItem($0, lifetime: .planned)
            packageStates[$0] = packageStates[$0] ?? PackageRuntimeState()
        }
        hasLoggedResolving = false
        lastActivePackage = nil
        isTerminalState = false
        terminalStage = nil
        primaryTitle = activePrimaryTitle
        primaryEnabled = false
        showSecondary = false
        renderRows()
        logs = []
        status = packageCountText(packages.count)
        if let initialOperation {
            operation = initialOperation
        }
        appendLog(activationLog)
        if packages.isEmpty {
            appendLog(L10n.string("Waiting for the package plan."))
        }
    }

    func handle(event: NukeHelperProgressEvent) {
        guard isTerminalState == false else {
            return
        }

        switch event {
        case .resolving:
            operation = L10n.string("Resolving package graph")
            if hasLoggedResolving == false {
                appendLog(L10n.string("Resolving package graph"))
                hasLoggedResolving = true
            }
            displayItemIDs.forEach {
                updateRow(package: $0, stage: .resolving, progress: ProgressLayout.resolving)
            }
        case .downloading(let package, let bytesPerSecond, let progress):
            let displayPackage = displayPackageName(forProgressPackage: package)
            let rowPackage = displayPackage ?? package
            if let displayPackage {
                operation = L10n.format("Updating %@", displayPackage.progressDisplayName)
            }
            if shouldLogDownloadStart(for: rowPackage) {
                appendLog(L10n.format("Downloading %@", package))
            }
            guard displayPackage != nil,
                  shouldRenderDownloadUpdate(for: rowPackage, progress: Double(progress)) else {
                return
            }
            updateRow(
                package: rowPackage,
                stage: .downloading,
                progress: downloadProgress(for: Double(progress)),
                speed: Self.format(speed: bytesPerSecond)
            )
        case .installing(let package):
            let displayPackage = displayPackageName(forProgressPackage: package)
            let rowPackage = displayPackage ?? package
            let state = packageStates[rowPackage]
                ?? packageStates[package]
                ?? PackageRuntimeState()
            if state.observedDownload {
                appendLog(L10n.format("Extracting %@", package))
                guard let displayPackage else { return }
                operation = L10n.format("Extracting %@", displayPackage.progressDisplayName)
                updateRow(
                    package: rowPackage,
                    stage: .extracting,
                    progress: extractProgress(from: state.progress)
                )
            } else {
                appendLog(L10n.format("Installing %@", package))
                guard let displayPackage else { return }
                operation = L10n.format("Installing %@", displayPackage.progressDisplayName)
                updateRow(
                    package: rowPackage,
                    stage: .installing,
                    progress: ProgressLayout.installFloor
                )
            }
        case .log(let package, let message):
            if let displayPackage = displayPackageName(forProgressPackage: package) {
                lastActivePackage = displayPackage
                operation = Self.sentenceCase(message)
            }
            appendLog("\(package): \(message)")
        case .completed(let package):
            let displayPackage = displayPackageName(forProgressPackage: package)
            let rowPackage = displayPackage ?? package
            appendLog(L10n.format("Completed %@", package))
            guard let displayPackage else { return }
            operation = L10n.format("Finishing %@", displayPackage.progressDisplayName)
            updateRow(package: rowPackage, stage: .completed, progress: 1)
        case .error(let message):
            fail(message: message)
        }
    }

    func succeed(message: String, packages: [String]) {
        guard isTerminalState == false else {
            return
        }

        packages
            .compactMap { displayPackageName(forProgressPackage: $0) }
            .forEach { updateRow(package: $0, stage: .completed, progress: 1) }
        isTerminalState = true
        terminalStage = .completed
        operation = successOperationTitle
        status = message
        primaryTitle = L10n.string("Dismiss")
        primaryEnabled = true
        showSecondary = false
        appendLog(message)
    }

    func fail(message: String) {
        guard isTerminalState == false else {
            return
        }

        isTerminalState = true
        terminalStage = .failed
        operation = failureOperationTitle
        status = message
        appendLog(L10n.format("Failed: %@", message))
        if let package = lastActivePackage ?? displayItemIDs.last {
            updateRow(package: package, stage: .failed, progress: 1)
        }
        primaryTitle = L10n.string("Retry")
        primaryEnabled = true
        showSecondary = true
    }

    private var displayItemIDs: [String] {
        displayItems.values
            .sorted { left, right in
                if left.addedIndex == right.addedIndex {
                    return left.id < right.id
                }
                return left.addedIndex < right.addedIndex
            }
            .map(\.id)
    }

    @discardableResult
    private func addDisplayItem(
        _ package: String,
        lifetime: PackageDisplayLifetime
    ) -> Bool {
        let normalized = package.trimmingCharacters(in: .whitespacesAndNewlines)
        guard normalized.isEmpty == false else {
            return false
        }
        if var item = displayItems[normalized] {
            if item.lifetime == .discovered, lifetime == .planned {
                item.lifetime = .planned
                displayItems[normalized] = item
            }
            renderRows()
            return true
        }
        displayItems[normalized] = PackageDisplayItem(
            id: normalized,
            addedIndex: nextDisplayIndex,
            lifetime: lifetime
        )
        nextDisplayIndex += 1
        renderRows()
        return true
    }

    private func displayPackageName(forProgressPackage package: String) -> String? {
        if displayItems[package] != nil {
            return package
        }

        let progressOrderName = package.packageSearchOrderName
        let qualifiedCandidates = displayItemIDs.filter {
            $0.packageSearchOrderName == progressOrderName
        }
        guard qualifiedCandidates.count == 1 else {
            if acceptsDiscoveredDisplayItems {
                addDisplayItem(package, lifetime: .discovered)
                packageStates[package] = packageStates[package] ?? PackageRuntimeState()
                return package
            }
            packageStates[package] = packageStates[package] ?? PackageRuntimeState()
            return nil
        }
        let displayPackage = qualifiedCandidates[0]
        packageStates[displayPackage] = packageStates[displayPackage]
            ?? packageStates[package]
            ?? PackageRuntimeState()
        return displayPackage
    }

    private func updateRow(
        package: String,
        stage: PackageStage,
        progress: Double,
        speed: String? = nil
    ) {
        guard displayItems[package] != nil else {
            packageStates[package] = packageStates[package] ?? PackageRuntimeState()
            return
        }
        lastActivePackage = package
        var state = packageStates[package] ?? PackageRuntimeState()
        state.stage = stage
        state.progress = progress.clamped(to: 0 ... 1)
        state.speed = speed
        if stage == .downloading {
            state.observedDownload = true
        } else {
            state.didLogDownloadStart = false
            state.lastDownloadUpdateAt = nil
        }
        packageStates[package] = state
        renderRows()
    }

    private func renderRows() {
        rows = displayItemIDs.map { package in
            let state = packageStates[package] ?? PackageRuntimeState()
            return PackageProgressRowState(
                id: package,
                stage: state.stage,
                progress: state.progress,
                speed: state.speed
            )
        }
    }

    private func shouldLogDownloadStart(for package: String) -> Bool {
        var state = packageStates[package] ?? PackageRuntimeState()
        defer { packageStates[package] = state }
        if state.didLogDownloadStart {
            return false
        }
        state.didLogDownloadStart = true
        return true
    }

    private func shouldRenderDownloadUpdate(for package: String, progress: Double) -> Bool {
        let now = Date()
        var state = packageStates[package] ?? PackageRuntimeState()
        defer {
            state.lastDownloadUpdateAt = now
            state.lastDownloadProgress = progress
            packageStates[package] = state
        }
        let progressDelta = abs(progress - state.lastDownloadProgress)
        if progress >= 0.99 || progressDelta >= 0.015 {
            return true
        }
        guard let lastUpdate = state.lastDownloadUpdateAt else {
            return true
        }
        return now.timeIntervalSince(lastUpdate) >= 0.05
    }

    private func appendLog(_ line: String) {
        logs.append(ProgressLogEntry(
            timestamp: DateFormatter.localizedString(
                from: Date(),
                dateStyle: .none,
                timeStyle: .medium
            ),
            message: line
        ))
        if logs.count > 160 {
            logs.removeFirst(logs.count - 160)
        }
    }

    private func downloadProgress(for progress: Double) -> Double {
        let normalized = progress.clamped(to: 0 ... 1)
        return ProgressLayout.downloadFloor
            + normalized * (ProgressLayout.downloadCeiling - ProgressLayout.downloadFloor)
    }

    private func extractProgress(from currentProgress: Double) -> Double {
        max(currentProgress, ProgressLayout.extractFloor)
            .clamped(to: ProgressLayout.extractFloor ... ProgressLayout.extractCeiling)
    }

    private func packageCountText(_ count: Int) -> String {
        packageCountLabel(count)
    }

    private static func outdatedPackageCountText(_ count: Int) -> String {
        count == 1
            ? L10n.string("1 outdated package")
            : L10n.format("%d outdated packages", count)
    }

    private static func format(speed: UInt64) -> String {
        if speed >= 1_000_000 {
            return String(format: "%.1f MB/s", Double(speed) / 1_000_000)
        }
        if speed >= 1_000 {
            return String(format: "%.0f KB/s", Double(speed) / 1_000)
        }
        return "\(speed) B/s"
    }

    private static func sentenceCase(_ message: String) -> String {
        guard let first = message.first else { return message }
        return first.uppercased() + message.dropFirst()
    }
}

@MainActor
final class UpdateProgressViewController: NSViewController {
    var onRetry: (() -> Void)?
    var onDismiss: (() -> Void)?

    private let model = UpdateProgressViewModel()

    override func loadView() {
        view = NSHostingView(rootView: UpdateProgressSheetView(
            model: model,
            onPrimary: { [weak self] in self?.primaryAction() },
            onSecondary: { [weak self] in self?.secondaryAction() },
            onCancel: { [weak self] in self?.requestDismiss() }
        ))
    }

    func configure(
        title: String,
        awaitingClearance: String,
        idleStatus: String,
        successOperation: String,
        failureOperation: String,
        activePrimaryTitle: String = L10n.string("Updating"),
        packageCountLabel: ((Int) -> String)? = nil
    ) {
        model.configure(
            title: title,
            awaitingClearance: awaitingClearance,
            idleStatus: idleStatus,
            successOperation: successOperation,
            failureOperation: failureOperation,
            activePrimaryTitle: activePrimaryTitle,
            packageCountLabel: packageCountLabel
        )
    }

    func begin(
        packages: [String],
        activationLog: String,
        initialOperation: String? = nil
    ) {
        model.begin(
            packages: packages,
            activationLog: activationLog,
            initialOperation: initialOperation
        )
    }

    func handle(event: NukeHelperProgressEvent) {
        model.handle(event: event)
    }

    func succeed(message: String, packages: [String]) {
        model.succeed(message: message, packages: packages)
    }

    func fail(message: String) {
        model.fail(message: message)
    }

    private func primaryAction() {
        guard model.isTerminalState else {
            NSSound.beep()
            return
        }
        if model.primaryTitle == L10n.string("Retry") {
            onRetry?()
        } else {
            onDismiss?()
        }
    }

    private func secondaryAction() {
        guard model.isTerminalState else {
            NSSound.beep()
            return
        }
        onDismiss?()
    }

    private func requestDismiss() {
        guard model.isTerminalState else {
            NSSound.beep()
            return
        }
        onDismiss?()
    }
}

private struct UpdateProgressSheetView: View {
    @ObservedObject var model: UpdateProgressViewModel
    let onPrimary: () -> Void
    let onSecondary: () -> Void
    let onCancel: () -> Void

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
                .overlay(UpdateProgressPalette.hairline)
            content
            Divider()
                .overlay(UpdateProgressPalette.hairline)
            footer
        }
        .frame(width: 820, height: 700)
        .background {
            Rectangle()
                .fill(.ultraThinMaterial)
                .overlay(UpdateProgressPalette.windowTint)
        }
        .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
        .overlay {
            RoundedRectangle(cornerRadius: 8, style: .continuous)
                .stroke(UpdateProgressPalette.controlBorder.opacity(0.28), lineWidth: 1)
        }
        .preferredColorScheme(.dark)
        .onExitCommand(perform: onCancel)
    }

    private var header: some View {
        HStack(spacing: 14) {
            ZStack {
                Circle()
                    .fill(model.statusTint.opacity(0.16))
                Image(systemName: model.terminalStage?.systemImage ?? "arrow.triangle.2.circlepath")
                    .font(.system(size: 18, weight: .semibold))
                    .foregroundStyle(model.statusTint)
                    .symbolRenderingMode(.hierarchical)
            }
            .frame(width: 42, height: 42)

            VStack(alignment: .leading, spacing: 5) {
                Text(model.title)
                    .font(.system(size: 20, weight: .semibold))
                    .foregroundStyle(UpdateProgressPalette.primaryText)
                    .lineLimit(1)
                Text(model.operation)
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(UpdateProgressPalette.secondaryText)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }

            Spacer(minLength: 16)

            StatusPill(
                text: model.progressSummary,
                systemImage: model.terminalStage?.systemImage ?? "waveform.path.ecg",
                tint: model.statusTint
            )
        }
        .padding(.horizontal, 24)
        .frame(height: 82)
        .background {
            Rectangle()
                .fill(.thinMaterial)
                .overlay(UpdateProgressPalette.headerTint)
        }
    }

    private var content: some View {
        VStack(spacing: 16) {
            overallProgress
            HStack(alignment: .top, spacing: 16) {
                packagePanel
                    .frame(width: 464)
                activityPanel
                    .frame(maxWidth: .infinity)
            }
        }
        .padding(20)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var overallProgress: some View {
        VStack(alignment: .leading, spacing: 9) {
            HStack(alignment: .firstTextBaseline) {
                Text(model.status)
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(UpdateProgressPalette.primaryText)
                    .lineLimit(1)
                    .truncationMode(.middle)
                Spacer()
                Text("\(Int((model.overallProgress * 100).rounded()))%")
                    .font(.system(size: 13, weight: .semibold, design: .monospaced))
                    .foregroundStyle(UpdateProgressPalette.secondaryText)
                    .monospacedDigit()
            }
            ProgressView(value: model.overallProgress)
                .tint(model.statusTint)
        }
        .padding(14)
        .frame(height: 72)
        .background(UpdateProgressPalette.controlFill, in: RoundedRectangle(cornerRadius: 8))
        .overlay {
            RoundedRectangle(cornerRadius: 8)
                .stroke(UpdateProgressPalette.controlBorder.opacity(0.20), lineWidth: 1)
        }
    }

    private var packagePanel: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionLabel(L10n.string("Packages"))
            ScrollView {
                LazyVStack(spacing: 8) {
                    if model.rows.isEmpty {
                        EmptyProgressPlaceholder(text: L10n.string("Preparing package plan"))
                    } else {
                        ForEach(model.rows) { row in
                            PackageProgressRow(row: row)
                        }
                    }
                }
                .padding(10)
            }
            .scrollIndicators(.visible)
            .background(UpdateProgressPalette.panelFill, in: RoundedRectangle(cornerRadius: 8))
            .overlay {
                RoundedRectangle(cornerRadius: 8)
                    .stroke(UpdateProgressPalette.controlBorder.opacity(0.16), lineWidth: 1)
            }
        }
    }

    private var activityPanel: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionLabel(L10n.string("Activity"))
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 7) {
                        if model.logs.isEmpty {
                            EmptyProgressPlaceholder(text: L10n.string("Waiting for activity"))
                        } else {
                            ForEach(model.logs) { entry in
                                LogEntryRow(entry: entry)
                                    .id(entry.id)
                            }
                        }
                    }
                    .padding(12)
                }
                .scrollIndicators(.visible)
                .background(UpdateProgressPalette.panelFill, in: RoundedRectangle(cornerRadius: 8))
                .overlay {
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(UpdateProgressPalette.controlBorder.opacity(0.16), lineWidth: 1)
                }
                .onChange(of: model.logs.last?.id) { _, id in
                    guard let id else { return }
                    withAnimation(.easeOut(duration: 0.18)) {
                        proxy.scrollTo(id, anchor: .bottom)
                    }
                }
            }
        }
    }

    private var footer: some View {
        HStack(spacing: 12) {
            if model.isTerminalState == false {
                ProgressView()
                    .controlSize(.small)
                    .tint(UpdateProgressPalette.cyan)
                Text(L10n.string("Updating packages"))
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(UpdateProgressPalette.secondaryText)
            }

            Spacer()

            if model.showSecondary {
                Button(action: onSecondary) {
                    Text(L10n.string("Dismiss"))
                        .frame(minWidth: 82)
                }
                .buttonStyle(.glass)
                .tint(.clear)
            }

            Button(action: onPrimary) {
                Label(model.primaryTitle, systemImage: model.primarySystemImage)
                    .frame(minWidth: 94)
            }
            .buttonStyle(.glass)
            .tint(.clear)
            .disabled(model.primaryEnabled == false)
        }
        .padding(.horizontal, 24)
        .frame(height: 64)
        .background {
            Rectangle()
                .fill(.thinMaterial)
                .overlay(UpdateProgressPalette.footerTint)
        }
    }
}

private struct PackageProgressRow: View {
    let row: PackageProgressRowState

    var body: some View {
        HStack(spacing: 11) {
            Image(systemName: row.stage.systemImage)
                .font(.system(size: 15, weight: .semibold))
                .foregroundStyle(row.stage.tint)
                .frame(width: 20)
                .symbolRenderingMode(.hierarchical)

            VStack(alignment: .leading, spacing: 7) {
                HStack(alignment: .firstTextBaseline, spacing: 8) {
                    Text(row.id)
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(UpdateProgressPalette.primaryText)
                        .lineLimit(1)
                        .truncationMode(.middle)
                    Spacer(minLength: 8)
                    Text(row.stage.title)
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(row.stage.tint)
                        .lineLimit(1)
                    if let speed = row.speed {
                        Text(speed)
                            .font(.system(size: 11, weight: .medium, design: .monospaced))
                            .foregroundStyle(UpdateProgressPalette.quietText)
                            .lineLimit(1)
                    }
                }
                ProgressView(value: row.progress)
                    .tint(row.stage.tint)
            }
        }
        .padding(.horizontal, 11)
        .frame(height: 58)
        .background(UpdateProgressPalette.rowFill, in: RoundedRectangle(cornerRadius: 8))
    }
}

private struct LogEntryRow: View {
    let entry: ProgressLogEntry

    var body: some View {
        HStack(alignment: .top, spacing: 8) {
            Text(entry.timestamp)
                .font(.system(size: 11, weight: .medium, design: .monospaced))
                .foregroundStyle(UpdateProgressPalette.quietText)
                .frame(width: 76, alignment: .leading)
                .lineLimit(1)
            Text(entry.message)
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(UpdateProgressPalette.secondaryText)
                .lineLimit(3)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}

private struct StatusPill: View {
    let text: String
    let systemImage: String
    let tint: Color

    var body: some View {
        Label(text, systemImage: systemImage)
            .font(.system(size: 12, weight: .semibold))
            .foregroundStyle(tint)
            .lineLimit(1)
            .padding(.horizontal, 10)
            .frame(height: 28)
            .background(tint.opacity(0.13), in: Capsule())
            .overlay(Capsule().stroke(tint.opacity(0.26), lineWidth: 1))
    }
}

private struct SectionLabel: View {
    let title: String

    init(_ title: String) {
        self.title = title
    }

    var body: some View {
        Text(title)
            .font(.system(size: 11, weight: .semibold))
            .foregroundStyle(UpdateProgressPalette.quietText)
    }
}

private struct EmptyProgressPlaceholder: View {
    let text: String

    var body: some View {
        Text(text)
            .font(.system(size: 13, weight: .medium))
            .foregroundStyle(UpdateProgressPalette.quietText)
            .frame(maxWidth: .infinity, minHeight: 90)
    }
}

private enum UpdateProgressPalette {
    static let windowTint = Color.black.opacity(0.30)
    static let headerTint = Color.black.opacity(0.18)
    static let footerTint = Color.black.opacity(0.14)
    static let panelFill = Color.white.opacity(0.045)
    static let rowFill = Color.white.opacity(0.060)
    static let controlFill = Color.white.opacity(0.070)
    static let controlBorder = Color.white.opacity(0.22)
    static let hairline = Color.white.opacity(0.10)
    static let primaryText = Color.white.opacity(0.92)
    static let secondaryText = Color.white.opacity(0.66)
    static let quietText = Color.white.opacity(0.38)
    static let green = Color(red: 0.10, green: 0.86, blue: 0.58)
    static let cyan = Color(red: 0.10, green: 0.52, blue: 1.00)
    static let blue = Color(red: 0.55, green: 0.67, blue: 0.82)
    static let red = Color(red: 1.00, green: 0.45, blue: 0.45)
}

private extension Comparable {
    func clamped(to limits: ClosedRange<Self>) -> Self {
        min(max(self, limits.lowerBound), limits.upperBound)
    }
}

private extension String {
    var progressDisplayName: String {
        packageSearchOrderName
    }
}
