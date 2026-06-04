import AppKit
import SwiftUI

@MainActor
final class PackWindowController: NSWindowController {
    private let model: PackWindowModel
    private let helperBridge = NukeHelperBridge()
    private let onInstallFinished: () -> Void
    private var progressController: UpdateProgressViewController?

    init(pack: PackagePack, onInstallFinished: @escaping () -> Void) {
        self.model = PackWindowModel(pack: pack)
        self.onInstallFinished = onInstallFinished
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 980, height: 720),
            styleMask: [
                .titled,
                .closable,
                .miniaturizable,
                .resizable,
                .fullSizeContentView
            ],
            backing: .buffered,
            defer: false
        )
        super.init(window: window)
        window.center()
        window.title = pack.title
        window.backgroundColor = .clear
        window.isOpaque = false
        window.appearance = NSAppearance(named: .darkAqua)
        window.isReleasedWhenClosed = false
        window.titleVisibility = .hidden
        window.titlebarAppearsTransparent = true
        window.isMovableByWindowBackground = true
        window.minSize = NSSize(width: 880, height: 640)
        window.contentViewController = NSHostingController(
            rootView: PackWindowView(
                model: model,
                onInstallAll: { [weak self] in
                    self?.installAll()
                }
            )
        )
    }

    @MainActor @preconcurrency required dynamic init?(coder: NSCoder) {
        return nil
    }

    private func installAll() {
        guard model.canInstallAll else {
            NSSound.beep()
            return
        }

        model.lastErrorMessage = nil
        model.statusMessage = L10n.string("Waiting for Touch ID authorization")
        model.isAuthorizing = true
        helperBridge.authenticateBiometrics(
            reason: L10n.format(
                "Authorize privileged package install for %@.",
                model.pack.title
            )
        ) { [weak self] result in
            guard let self else { return }
            self.model.isAuthorizing = false
            switch result {
            case .success:
                self.startAuthorizedInstallAll()
            case .failure(let error):
                self.model.statusMessage = nil
                self.presentHelperError(error)
            }
        }
    }

    private func startAuthorizedInstallAll() {
        guard model.isInstalling == false else {
            return
        }
        let packageNames = model.pack.installPackageNames
        guard packageNames.isEmpty == false else {
            NSSound.beep()
            return
        }

        let progressController = presentProgressController()
        configure(progressController, packageCount: packageNames.count)
        model.isInstalling = true
        model.statusMessage = L10n.string("Installing")
        progressController.begin(
            packages: packageNames,
            activationLog: L10n.format(
                "%@ %@.",
                L10n.string("Installing"),
                model.packageCountText
            ),
            initialOperation: L10n.string("Awaiting helper authorization")
        )

        helperBridge.install(
            packages: packageNames.map { AVPackageSpec(name: $0) },
            progress: { [weak progressController] event in
                progressController?.handle(event: event)
            },
            completion: { [weak self, weak progressController] result in
                guard let self else { return }
                self.model.isInstalling = false
                switch result {
                case .success(let helperResult):
                    let completedPackages = helperResult.processedPackages.isEmpty
                        ? packageNames
                        : helperResult.processedPackages
                    self.model.statusMessage = L10n.string("Install Complete")
                    progressController?.succeed(
                        message: helperResult.message,
                        packages: completedPackages
                    )
                    self.onInstallFinished()
                case .failure(let error):
                    self.model.statusMessage = nil
                    self.model.lastErrorMessage = error.localizedDescription
                    progressController?.fail(message: error.localizedDescription)
                }
            }
        )
    }

    private func presentProgressController() -> UpdateProgressViewController {
        if let progressController {
            return progressController
        }

        let controller = UpdateProgressViewController()
        controller.preferredContentSize = NSSize(width: 820, height: 700)
        controller.onRetry = { [weak self] in
            self?.installAll()
        }
        controller.onDismiss = { [weak self] in
            self?.dismissProgressController()
        }
        window?.contentViewController?.presentAsSheet(controller)
        progressController = controller
        return controller
    }

    private func dismissProgressController() {
        guard let controller = progressController else {
            return
        }
        window?.contentViewController?.dismiss(controller)
        progressController = nil
    }

    private func configure(
        _ progressController: UpdateProgressViewController,
        packageCount: Int
    ) {
        progressController.onRetry = { [weak self] in
            self?.installAll()
        }
        progressController.configure(
            title: L10n.format("%@ %@", L10n.string("Install"), model.pack.title),
            awaitingClearance: L10n.string("Waiting for helper authorization"),
            idleStatus: PackWindowModel.packageCountText(packageCount),
            successOperation: L10n.string("Install Complete"),
            failureOperation: L10n.string("Install Halted"),
            activePrimaryTitle: L10n.string("Installing"),
            packageCountLabel: PackWindowModel.packageCountText(_:)
        )
    }

    private func presentHelperError(_ error: Error) {
        if case NukeHelperBridgeError.biometricCanceled = error {
            return
        }

        model.lastErrorMessage = error.localizedDescription
        let alert = NSAlert()
        alert.messageText = L10n.string("Privileged Operation Failed")
        alert.informativeText = error.localizedDescription
        alert.alertStyle = .warning
        alert.addButton(withTitle: L10n.string("OK"))

        if let window, window.attachedSheet == nil {
            alert.beginSheetModal(for: window)
        } else {
            alert.runModal()
        }
    }
}

@MainActor
final class PackWindowModel: ObservableObject {
    let pack: PackagePack
    @Published var isAuthorizing = false
    @Published var isInstalling = false
    @Published var statusMessage: String?
    @Published var lastErrorMessage: String?

    init(pack: PackagePack) {
        self.pack = pack
    }

    var canInstallAll: Bool {
        !isAuthorizing && !isInstalling && pack.installPackageNames.isEmpty == false
    }

    var packageCountText: String {
        Self.packageCountText(pack.installPackageNames.count)
    }

    nonisolated static func packageCountText(_ count: Int) -> String {
        count == 1
            ? L10n.string("1 package")
            : L10n.format("%d packages", count)
    }
}

private struct PackWindowView: View {
    @ObservedObject var model: PackWindowModel
    let onInstallAll: () -> Void

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
                .overlay(PackWindowPalette.hairline)
            content
        }
        .frame(minWidth: 820, minHeight: 640)
        .background {
            Rectangle()
                .fill(.ultraThinMaterial)
                .overlay(PackWindowPalette.windowTint)
                .backgroundExtensionEffect()
        }
        .preferredColorScheme(.dark)
    }

    private var header: some View {
        HStack(spacing: 14) {
            ZStack {
                Circle()
                    .fill(PackWindowPalette.accent.opacity(0.16))
                Image(systemName: model.pack.systemImage)
                    .font(.system(size: 18, weight: .semibold))
                    .foregroundStyle(PackWindowPalette.accent)
                    .symbolRenderingMode(.hierarchical)
            }
            .frame(width: 42, height: 42)

            VStack(alignment: .leading, spacing: 5) {
                Text(model.pack.title)
                    .font(.system(size: 20, weight: .semibold))
                    .foregroundStyle(PackWindowPalette.primaryText)
                    .lineLimit(1)
                Text(model.statusMessage ?? model.packageCountText)
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(PackWindowPalette.secondaryText)
                    .lineLimit(1)
            }

            Spacer(minLength: 16)

            installAllButton
        }
        .padding(.horizontal, 24)
        .frame(height: 82)
        .background {
            Rectangle()
                .fill(.thinMaterial)
                .overlay(PackWindowPalette.headerTint)
        }
    }

    private var installAllButton: some View {
        Button(action: onInstallAll) {
            HStack(spacing: 6) {
                if model.isAuthorizing || model.isInstalling {
                    ProgressView()
                        .controlSize(.small)
                        .frame(width: 13, height: 13)
                } else {
                    Image(systemName: "arrow.down.circle")
                        .font(.system(size: 11, weight: .regular))
                        .symbolRenderingMode(.hierarchical)
                }
                Text(model.isInstalling
                    ? L10n.string("Installing")
                    : L10n.string("Install All"))
                    .font(.system(size: 11, weight: .regular))
                    .lineLimit(1)
                    .minimumScaleFactor(0.84)
            }
            .foregroundStyle(PackWindowPalette.secondaryText)
            .frame(minWidth: 106)
            .frame(height: 18, alignment: .center)
            .contentShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
        }
        .buttonStyle(.glass)
        .tint(.clear)
        .disabled(!model.canInstallAll)
        .opacity(model.canInstallAll ? 1 : 0.42)
    }

    private var content: some View {
        GeometryReader { proxy in
            let overviewWidth = min(
                PackWindowLayout.maximumOverviewWidth,
                max(
                    PackWindowLayout.minimumOverviewWidth,
                    proxy.size.width * PackWindowLayout.overviewWidthRatio
                )
            )

            HStack(spacing: 0) {
                overview
                    .frame(width: overviewWidth)

                Divider()
                    .overlay(PackWindowPalette.hairline)

                packagePanel
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
    }

    private var overview: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                packImage
                    .aspectRatio(16.0 / 9.0, contentMode: .fit)
                    .frame(maxWidth: .infinity)
                    .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                    .overlay {
                        RoundedRectangle(cornerRadius: 8, style: .continuous)
                            .stroke(
                                PackWindowPalette.controlBorder.opacity(0.22),
                                lineWidth: 1
                            )
                    }

                Text(model.pack.summary)
                    .font(.system(size: 14, weight: .regular))
                    .foregroundStyle(PackWindowPalette.secondaryText)
                    .lineSpacing(3)
                    .lineLimit(nil)
                    .fixedSize(horizontal: false, vertical: true)
                    .frame(maxWidth: .infinity, alignment: .leading)

                if let lastErrorMessage = model.lastErrorMessage {
                    Text(lastErrorMessage)
                        .font(.system(size: 12, weight: .medium))
                        .foregroundStyle(PackWindowPalette.red)
                        .lineSpacing(2)
                        .fixedSize(horizontal: false, vertical: true)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
            .padding(24)
            .frame(maxWidth: .infinity, alignment: .topLeading)
        }
        .scrollIndicators(.visible)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background {
            Rectangle()
                .fill(.thinMaterial)
                .overlay(PackWindowPalette.sidebarTint)
        }
        .clipped()
    }

    @ViewBuilder
    private var packImage: some View {
        ZStack {
            PackWindowPalette.panelFill
            Image(systemName: model.pack.systemImage)
                .font(.system(size: 58, weight: .semibold))
                .foregroundStyle(PackWindowPalette.accent)
        }
    }

    private var packagePanel: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Text(L10n.string("Packages"))
                    .font(.system(size: 13, weight: .bold))
                    .foregroundStyle(PackWindowPalette.quietText)
                    .tracking(0.6)
                Spacer()
                Text(model.packageCountText)
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(PackWindowPalette.quietText)
            }

            ScrollView {
                LazyVGrid(
                    columns: PackWindowLayout.packageGridColumns,
                    alignment: .leading,
                    spacing: 10
                ) {
                    ForEach(
                        Array(model.pack.packageNames.enumerated()),
                        id: \.offset
                    ) { index, packageName in
                        PackPackageCell(
                            packageName: packageName,
                            installPackageName: model.pack.installPackageNames[index]
                        )
                    }
                }
                .padding(.bottom, 18)
            }
            .scrollIndicators(.visible)
        }
        .padding(.horizontal, 24)
        .padding(.top, 20)
        .padding(.bottom, 24)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }
}

private struct PackPackageCell: View {
    let packageName: String
    let installPackageName: String

    var body: some View {
        HStack(spacing: 9) {
            Image(systemName: sourceSystemImage)
                .font(.system(size: 13, weight: .semibold))
                .foregroundStyle(PackWindowPalette.quietText)
                .frame(width: 18)

            VStack(alignment: .leading, spacing: 4) {
                Text(packageName)
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(PackWindowPalette.primaryText)
                    .lineLimit(1)
                    .truncationMode(.middle)
                Text(sourceTitle)
                    .font(.system(size: 10, weight: .semibold))
                    .foregroundStyle(PackWindowPalette.quietText)
                    .tracking(0.6)
                    .lineLimit(1)
            }

            Spacer(minLength: 0)
        }
        .padding(.horizontal, 12)
        .frame(height: 58)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(PackWindowPalette.rowFill, in: RoundedRectangle(cornerRadius: 8))
        .overlay {
            RoundedRectangle(cornerRadius: 8)
                .stroke(PackWindowPalette.controlBorder.opacity(0.16), lineWidth: 1)
        }
    }

    private var sourceTitle: String {
        if installPackageName.hasPrefix("cask:") {
            return "CASK"
        }
        if installPackageName.hasPrefix("brew:") {
            return "BREW"
        }
        return installPackageName
    }

    private var sourceSystemImage: String {
        installPackageName.hasPrefix("cask:") ? "app.dashed" : "terminal"
    }
}

private enum PackWindowPalette {
    static let windowTint = Color.black.opacity(0.26)
    static let headerTint = Color.black.opacity(0.18)
    static let sidebarTint = Color(red: 0.025, green: 0.050, blue: 0.075).opacity(0.26)
    static let panelFill = Color.white.opacity(0.045)
    static let rowFill = Color.white.opacity(0.060)
    static let controlBorder = Color.white.opacity(0.22)
    static let hairline = Color.white.opacity(0.10)
    static let primaryText = Color.white.opacity(0.92)
    static let secondaryText = Color.white.opacity(0.66)
    static let quietText = Color.white.opacity(0.38)
    static let accent = Color(red: 0.10, green: 0.86, blue: 0.58)
    static let red = Color(red: 1.00, green: 0.45, blue: 0.45)
}

private enum PackWindowLayout {
    static let minimumOverviewWidth: CGFloat = 330
    static let maximumOverviewWidth: CGFloat = 410
    static let overviewWidthRatio: CGFloat = 0.38
    static let packageGridColumns = [
        GridItem(.flexible(minimum: 180), spacing: 10),
        GridItem(.flexible(minimum: 180), spacing: 10)
    ]
}
