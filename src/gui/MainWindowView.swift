import SwiftUI
import WebKit

struct MainWindowView: View {
    @ObservedObject var model: MainWindowModel
    @State private var linkTab: MainWindowLinkTab = .homepage
    @State private var browserCommand: BrowserCommand?

    var body: some View {
        ZStack {
            background
            mainContent
            titleBarBackdrop
        }
        .frame(minWidth: 1380, minHeight: 760)
        .background(Color.clear)
    }

    private var background: some View {
        LiquidGlassSurface(
            material: .ultraThinMaterial,
            tint: AVGlassPalette.windowTint
        )
        .backgroundExtensionEffect()
        .ignoresSafeArea()
    }

    private var titleBarBackdrop: some View {
        GeometryReader { _ in
            VStack(spacing: 0) {
                LiquidGlassSurface(
                    material: .ultraThinMaterial,
                    tint: AVGlassPalette.topBarTint
                )
                .frame(height: 56)
                .overlay(alignment: .bottom) {
                    Rectangle()
                        .fill(AVGlassPalette.titleBarSeparator)
                        .frame(height: 1)
                }
                .overlay(alignment: .bottom) {
                    LinearGradient(
                        colors: [
                            AVGlassPalette.titleBarShadow,
                            .clear,
                        ],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                    .frame(height: 14)
                    .offset(y: 14)
                }

                Spacer(minLength: 0)
            }
            .ignoresSafeArea(edges: .top)
        }
        .allowsHitTesting(false)
        .accessibilityHidden(true)
    }

    private var mainContent: some View {
        GeometryReader { proxy in
            let width = proxy.size.width
            let sidebarWidth = min(290, max(270, width * 0.19))
            let packageWidth = min(380, max(340, width * 0.28))
            let dossierWidth = min(340, max(320, width * 0.24))
            let linksWidth = max(width - sidebarWidth - packageWidth - dossierWidth - 3, 360)

            HStack(spacing: 0) {
                sidebar
                    .frame(width: sidebarWidth)
                verticalHairline
                packageList
                    .frame(width: packageWidth)
                verticalHairline
                dossierPanel
                    .frame(width: dossierWidth)
                verticalHairline
                linksPanel
                    .frame(width: linksWidth)
            }
        }
    }

    private static let pulseDateFormatter = ISO8601DateFormatter()

    private static let pulseRelativeFormatter: RelativeDateTimeFormatter = {
        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .full
        return formatter
    }()

    private var sidebar: some View {
        VStack(alignment: .leading, spacing: 0) {
            sidebarHeader("AUTOMIC VAULT")
                .kerning(1.2)
                .padding(.top, 26)
            ForEach(MainWindowSection.librarySections) { section in
                sidebarRow(section)
            }

            sidebarHeader("CATEGORIES")
                .padding(.top, 22)
                .kerning(1.2)
            ForEach(MainWindowSection.categorySections) { section in
                sidebarRow(section)
            }

            Spacer(minLength: 24)

            VStack(spacing: 0) {
                ForEach(MainWindowSection.utilitySections) { section in
                    sidebarRow(section)
                }
            }
            .padding(.bottom, 18)
        }
        .padding(.horizontal, 18)
        .background {
            LiquidGlassSurface(
                material: .ultraThinMaterial,
                tint: AVGlassPalette.sidebarTint
            )
        }
    }

    private func sidebarHeader(_ text: String) -> some View {
        Text(text)
            .font(.system(size: 11, weight: .semibold))
            .foregroundStyle(AVGlassPalette.quietText)
            .tracking(0.5)
            .padding(.bottom, 6)
    }

    private func sidebarRow(_ section: MainWindowSection) -> some View {
        Button {
            model.selectedSection = section
        } label: {
            HStack(spacing: 12) {
                Image(systemName: section.systemImage)
                    .font(.system(size: 14, weight: .semibold))
                    .frame(width: 17)
                Text(section.title)
                    .font(.system(size: 14, weight: .regular))
                    .lineLimit(1)
                    .layoutPriority(1)
                Spacer(minLength: 6)
                if let count = model.count(for: section) {
                    if section == .geigerCounter && count > 0 {
                        CountPill(count: count, prominence: .critical)
                            .fixedSize()
                    } else {
                        SidebarCountText(count: count)
                            .fixedSize()
                    }
                }
            }
            .foregroundStyle(
                model.selectedSection == section
                    ? AVGlassPalette.primaryText
                    : AVGlassPalette.secondaryText
            )
            .padding(.horizontal, 7)
            .frame(height: 32)
            .contentShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
            .background {
                if model.selectedSection == section {
                    RoundedRectangle(cornerRadius: 8, style: .continuous)
                        .fill(AVGlassPalette.sidebarSelectedFill)
                }
            }
        }
        .buttonStyle(.plain)
    }

    private var packageList: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Package")
                Image(systemName: "arrow.up.arrow.down")
                    .font(.system(size: 11, weight: .bold))
                Spacer()
                if model.selectedSection == .outdated {
                    Button {
                        model.requestOutdatedUpdateAll()
                    } label: {
                        UpdateAllHeaderButtonLabel(
                            title: model.isUpdatingAll ? "Updating" : "Update All"
                        )
                    }
                    .buttonStyle(.glass)
                    .tint(.clear)
                    .disabled(!model.canUpdateAllOutdated)
                    .opacity(model.canUpdateAllOutdated ? 1 : 0.42)
                    .help(updateAllHelpText)
                    .offset(y: 2)
                }
                if model.isReloading
                    || model.isSearching
                    || model.isLoadingSectionPage
                    || model.isUpdatingAll {
                    ProgressView()
                        .controlSize(.small)
                }
            }
            .font(.system(size: 13, weight: .bold))
            .foregroundStyle(AVGlassPalette.quietText)
            .padding(.leading, 18)
            .padding(.trailing, 7)
            .frame(height: 42)

            hairline

            ScrollView {
                LazyVStack(spacing: 0) {
                    ForEach(model.displayedPackages, id: \.selectionID) { package in
                        PackageRow(
                            package: package,
                            title: model.displayName(for: package),
                            description: model.packageDescription(for: package),
                            version: packageRowVersion(for: package),
                            inlineBadges: model.packageInlineBadges(for: package),
                            badges: packageRowBadges(for: package),
                            selected: model.selectedItemID == package.selectionID
                        ) {
                            model.select(package)
                        }
                        hairline
                    }

                }
            }
            .scrollIndicators(.visible)
        }
        .background {
            LiquidGlassSurface(
                material: .thinMaterial,
                tint: AVGlassPalette.packageTint
            )
        }
    }

    private var updateAllHelpText: String {
        let count = model.outdatedUpdatePackageNames.count
        guard count > 0 else {
            return "No outdated packages to update"
        }
        return count == 1
            ? "Update 1 outdated package"
            : "Update \(count) outdated packages"
    }

    private func packageRowVersion(for package: PackagePresentation) -> String {
        if model.selectedSection == .newUpdated,
           case .available(let result) = package.item {
            if isNewPulseResult(result) {
                return ""
            }
            return pulseVersionText(for: result)
        }
        return model.versionText(for: package)
    }

    private func packageRowBadges(for package: PackagePresentation) -> [MainWindowPackageBadge] {
        var badges = model.packageListBadges(for: package)
        if model.selectedSection == .newUpdated,
           case .available(let result) = package.item,
           isNewPulseResult(result) {
            badges.append(.new)
        }
        return badges
    }

    private func isNewPulseResult(_ result: PackageSearchResult) -> Bool {
        let pulseKind = result.pulseKind?
            .trimmingCharacters(in: .whitespacesAndNewlines)
        return pulseKind?.localizedCaseInsensitiveCompare("new") == .orderedSame
    }

    private func pulseVersionText(for result: PackageSearchResult) -> String {
        guard let raw = result.lastUpdatedAt,
              let date = Self.pulseDateFormatter.date(from: raw) else {
            return "Updated recently"
        }
        return "Updated \(Self.pulseRelativeFormatter.localizedString(for: date, relativeTo: Date()))"
    }

    private var dossierPanel: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                if let detail = model.selectedDetail,
                   let package = model.selectedPackage {
                    dossierHeader(detail: detail, package: package)
                    executableSection(detail: detail)
                    permissionsSection(detail: detail, package: package)
                    notesSection(detail: detail, package: package)
                    lastUpdatedSection(detail: detail)
                } else {
                    Color.clear
                        .frame(maxWidth: .infinity, minHeight: 1)
                }
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 22)
        }
        .scrollIndicators(.hidden)
        .background {
            LiquidGlassSurface(
                material: .regularMaterial,
                tint: AVGlassPalette.dossierTint
            )
        }
    }

    private func dossierHeader(
        detail: PackageDetail,
        package: PackagePresentation
    ) -> some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text(model.displayName(for: package))
                    .font(.system(size: 22, weight: .semibold))
                    .foregroundStyle(AVGlassPalette.primaryText)
                    .lineLimit(1)
                Text(model.versionText(for: package))
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(AVGlassPalette.quietText)
                    .lineLimit(1)
            }

            if let badge = model.packageBadge(for: package) {
                PackageBadgeBanner(badge: badge)
            }

            if model.isLoadingDetail {
                HStack(spacing: 8) {
                    ProgressView()
                        .controlSize(.small)
                    Text("Refreshing dossier")
                        .font(.system(size: 12, weight: .medium))
                        .foregroundStyle(AVGlassPalette.quietText)
                }
            }
        }
    }

    private func executableSection(detail: PackageDetail) -> some View {
        InfoSection(title: "EXECUTABLES") {
            let paths = detail.executablePaths.isEmpty
                ? ["/usr/local/bin/\(detail.helperPackageName.split(separator: ":").last.map(String.init) ?? detail.packageName)"]
                : detail.executablePaths
            VStack(alignment: .leading, spacing: 8) {
                ForEach(paths.prefix(2), id: \.self) { path in
                    VStack(alignment: .leading, spacing: 3) {
                        Text(URL(fileURLWithPath: path).lastPathComponent)
                            .font(.system(size: 13, weight: .bold))
                            .foregroundStyle(AVGlassPalette.primaryText)
                        Text(path)
                            .font(.system(size: 12, weight: .medium, design: .monospaced))
                            .foregroundStyle(AVGlassPalette.quietText)
                            .lineLimit(1)
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(10)
                    .background(AVGlassPalette.controlFill, in: RoundedRectangle(cornerRadius: 8))
                    .overlay(alignment: .topTrailing) {
                        Text("Sandboxed")
                            .font(.system(size: 10, weight: .semibold))
                            .foregroundStyle(AVGlassPalette.blue)
                            .padding(.horizontal, 7)
                            .padding(.vertical, 3)
                            .background(AVGlassPalette.blue.opacity(0.16), in: Capsule())
                            .padding(7)
                    }
                }
                if paths.count > 2 {
                    Text("\(paths.count - 2) more")
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(AVGlassPalette.secondaryText)
                }
            }
        }
    }

    private func permissionsSection(
        detail: PackageDetail,
        package: PackagePresentation
    ) -> some View {
        InfoSection(title: "PERMISSIONS") {
            VStack(spacing: 8) {
                PermissionRow(icon: "network", title: "Network Access", allowed: true)
                PermissionRow(icon: "folder", title: "File System", allowed: true)
                PermissionRow(icon: "point.3.connected.trianglepath.dotted", title: "Process Spawning", allowed: true)
                PermissionRow(icon: "key", title: "Secrets Access", allowed: model.isHardened(package) || detail.securityNotice != nil)
            }
        }
    }

    private func notesSection(
        detail: PackageDetail,
        package: PackagePresentation
    ) -> some View {
        InfoSection(title: "NOTES") {
            Text(noteText(detail: detail, package: package))
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(AVGlassPalette.secondaryText)
                .lineSpacing(3)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(12)
                .background(AVGlassPalette.controlFill, in: RoundedRectangle(cornerRadius: 8))
        }
    }

    private func lastUpdatedSection(detail: PackageDetail) -> some View {
        InfoSection(title: "LAST UPDATED") {
            Text(model.relativeLastUpdatedText(for: detail))
                .font(.system(size: 16, weight: .semibold))
                .foregroundStyle(AVGlassPalette.secondaryText)
        }
    }

    private func noteText(
        detail: PackageDetail,
        package: PackagePresentation
    ) -> String {
        if let notice = detail.securityNotice {
            return notice.body
        }
        if model.isHardened(package) {
            return "This package is hardened. Binary execution is sandboxed and secret access is restricted."
        }
        return detail.primaryDescription
    }

    private var linksPanel: some View {
        VStack(spacing: 0) {
            linksToolbar
            hairline
            linkBrowser
        }
        .background {
            LiquidGlassSurface(
                material: .thinMaterial,
                tint: AVGlassPalette.linksTint
            )
        }
    }

    private var linksToolbar: some View {
        VStack(spacing: 10) {
            HStack(spacing: 10) {
                HStack(spacing: 6) {
                    ForEach(MainWindowLinkTab.allCases) { tab in
                        Button {
                            linkTab = tab
                        } label: {
                            Text(tab.title)
                                .font(.system(size: 12, weight: .semibold))
                                .foregroundStyle(
                                    linkTab == tab
                                        ? AVGlassPalette.primaryText
                                        : AVGlassPalette.quietText
                                )
                                .lineLimit(1)
                                .minimumScaleFactor(0.82)
                                .padding(.horizontal, 10)
                                .frame(height: 30)
                        }
                        .buttonStyle(.plain)
                        .background {
                            if linkTab == tab {
                                RoundedRectangle(cornerRadius: 9, style: .continuous)
                                    .fill(AVGlassPalette.selectedFill)
                            }
                        }
                        .overlay {
                            if linkTab == tab {
                                RoundedRectangle(cornerRadius: 9, style: .continuous)
                                    .stroke(AVGlassPalette.controlBorder.opacity(0.28), lineWidth: 1)
                            }
                        }
                    }
                }
                .layoutPriority(1)

                Spacer()

                Button {
                    model.open(url: model.selectedURL(for: linkTab))
                } label: {
                    Image(systemName: "arrow.up.right.square")
                }
                .buttonStyle(.glass)
                .tint(.clear)
                .disabled(model.selectedURL(for: linkTab) == nil)
                .help("Open externally")
            }

            HStack(spacing: 8) {
                BrowserToolbarButton(systemName: "chevron.left") {
                    browserCommand = .back(UUID())
                }
                BrowserToolbarButton(systemName: "chevron.right") {
                    browserCommand = .forward(UUID())
                }
                BrowserToolbarButton(systemName: "arrow.clockwise") {
                    browserCommand = .reload(UUID())
                }
                Text(model.selectedURL(for: linkTab)?.absoluteString ?? "")
                    .font(.system(size: 13, weight: .semibold, design: .monospaced))
                    .foregroundStyle(
                        model.selectedURL(for: linkTab) == nil
                            ? AVGlassPalette.quietText
                            : AVGlassPalette.secondaryText
                    )
                    .lineLimit(1)
                    .padding(.horizontal, 12)
                    .frame(height: 32)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .glassEffect(.regular, in: RoundedRectangle(cornerRadius: 10, style: .continuous))
                Button {
                    model.open(url: model.selectedURL(for: linkTab))
                } label: {
                    Image(systemName: "arrow.up.right")
                }
                .buttonStyle(.glass)
                .tint(.clear)
                .disabled(model.selectedURL(for: linkTab) == nil)
            }
        }
        .padding(.horizontal, 20)
        .padding(.vertical, 12)
    }

    @ViewBuilder
    private var linkBrowser: some View {
        if let url = model.selectedURL(for: linkTab) {
            PackageWebView(url: url, command: browserCommand)
                .id(url)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(Color.white.opacity(0.96))
        } else {
            Color.clear
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }

    private var hairline: some View {
        Rectangle()
            .fill(AVGlassPalette.hairline)
            .frame(height: 1)
    }

    private var verticalHairline: some View {
        Rectangle()
            .fill(AVGlassPalette.hairline)
            .frame(width: 1)
    }
}

private enum BrowserCommand {
    case back(UUID)
    case forward(UUID)
    case reload(UUID)

    var id: UUID {
        switch self {
        case .back(let id), .forward(let id), .reload(let id):
            return id
        }
    }
}

private struct PackageWebView: NSViewRepresentable {
    let url: URL
    let command: BrowserCommand?

    func makeNSView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .default()
        let webView = WKWebView(frame: .zero, configuration: configuration)
        webView.allowsBackForwardNavigationGestures = true
        webView.setValue(false, forKey: "drawsBackground")
        webView.navigationDelegate = context.coordinator
        return webView
    }

    func updateNSView(_ webView: WKWebView, context: Context) {
        if context.coordinator.currentURL != url {
            context.coordinator.currentURL = url
            webView.load(URLRequest(url: url))
        }

        guard let command,
              context.coordinator.lastCommandID != command.id else {
            return
        }
        context.coordinator.lastCommandID = command.id
        switch command {
        case .back:
            if webView.canGoBack {
                webView.goBack()
            }
        case .forward:
            if webView.canGoForward {
                webView.goForward()
            }
        case .reload:
            webView.reload()
        }
    }

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    final class Coordinator: NSObject, WKNavigationDelegate {
        var currentURL: URL?
        var lastCommandID: UUID?

        func webView(
            _ webView: WKWebView,
            decidePolicyFor navigationAction: WKNavigationAction,
            decisionHandler: @escaping (WKNavigationActionPolicy) -> Void
        ) {
            if navigationAction.navigationType == .linkActivated,
               let url = navigationAction.request.url,
               url.host != currentURL?.host {
                NSWorkspace.shared.open(url)
                decisionHandler(.cancel)
            } else {
                decisionHandler(.allow)
            }
        }
    }
}

private struct SidebarCountText: View {
    let count: Int

    var body: some View {
        Text(count.formatted())
            .font(.system(size: 12, weight: .regular))
            .monospacedDigit()
            .foregroundStyle(AVGlassPalette.secondaryText)
            .lineLimit(1)
            .frame(minWidth: 18, alignment: .trailing)
    }
}

private struct UpdateAllHeaderButtonLabel: View {
    let title: String

    var body: some View {
        HStack(spacing: 6) {
            Image(systemName: "arrow.triangle.2.circlepath")
                .font(.system(size: 10, weight: .regular))
                .symbolRenderingMode(.hierarchical)
            Text(title)
                .font(.system(size: 10, weight: .regular))
                .lineLimit(1)
                .minimumScaleFactor(0.84)
        }
        .foregroundStyle(AVGlassPalette.secondaryText)
        .padding(.horizontal, 4)
        .frame(height: 16, alignment: .center)
        .contentShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
    }
}

private struct CountPill: View {
    enum Prominence {
        case normal
        case critical
    }

    let count: Int
    let prominence: Prominence

    var body: some View {
        Text(count.formatted())
            .font(.system(size: 11, weight: .semibold))
            .foregroundStyle(foreground)
            .padding(.horizontal, 8)
            .frame(height: 20)
            .background(background, in: Capsule())
            .overlay(Capsule().stroke(border, lineWidth: borderWidth))
            .shadow(color: shadow, radius: 5)
    }

    private var foreground: Color {
        switch prominence {
        case .normal:
            return AVGlassPalette.secondaryText
        case .critical:
            return AVGlassPalette.vulnerableText
        }
    }

    private var background: Color {
        switch prominence {
        case .normal:
            return AVGlassPalette.controlFill
        case .critical:
            return AVGlassPalette.vulnerableFill
        }
    }

    private var border: Color {
        switch prominence {
        case .normal:
            return .clear
        case .critical:
            return AVGlassPalette.vulnerableBorder
        }
    }

    private var borderWidth: CGFloat {
        switch prominence {
        case .normal:
            return 0
        case .critical:
            return 1.15
        }
    }

    private var shadow: Color {
        switch prominence {
        case .normal:
            return .clear
        case .critical:
            return AVGlassPalette.vulnerableBorder.opacity(0.28)
        }
    }
}

private struct PackageRow: View {
    let package: PackagePresentation
    let title: String
    let description: String
    let version: String
    let inlineBadges: [MainWindowPackageBadge]
    let badges: [MainWindowPackageBadge]
    let selected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(alignment: .top, spacing: 10) {
                VStack(alignment: .leading, spacing: 4) {
                    HStack(alignment: .firstTextBaseline, spacing: 6) {
                        Text(title)
                            .font(.system(size: 14, weight: .semibold))
                            .foregroundStyle(AVGlassPalette.primaryText)
                            .lineLimit(1)
                            .truncationMode(.tail)
                            .layoutPriority(1)
                        if version.isEmpty == false {
                            Text(version)
                                .font(.system(size: 12, weight: .regular))
                                .foregroundStyle(AVGlassPalette.quietText.opacity(0.74))
                                .lineLimit(1)
                                .truncationMode(.middle)
                        }
                        ForEach(inlineBadges, id: \.self) { badge in
                            PackageBadgePill(badge: badge, size: .inline)
                        }
                    }
                    Text(description)
                        .font(.system(size: 12, weight: .regular))
                        .foregroundStyle(AVGlassPalette.quietText)
                        .lineLimit(2)
                }

                Spacer(minLength: 8)

                if badges.isEmpty == false {
                    VStack(alignment: .trailing, spacing: 4) {
                        ForEach(badges, id: \.self) { badge in
                            PackageBadgePill(badge: badge)
                        }
                    }
                }
            }
            .padding(.horizontal, 18)
            .padding(.vertical, 11)
            .frame(minHeight: 76)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(selected ? AVGlassPalette.selectedFill : .clear)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}

private struct PackageBadgePill: View {
    enum Size {
        case regular
        case inline
    }

    let badge: MainWindowPackageBadge
    var size: Size = .regular

    var body: some View {
        Text(title)
            .font(.system(size: fontSize, weight: fontWeight))
            .foregroundStyle(foreground)
            .padding(.horizontal, horizontalPadding)
            .frame(height: height)
            .background(background, in: Capsule())
            .overlay(Capsule().stroke(border, lineWidth: 1))
            .fixedSize(horizontal: true, vertical: false)
    }

    private var title: String {
        switch badge {
        case .new:
            return "New"
        case .vulnerable:
            return "Vulnerable"
        case .hardened:
            return "Hardened"
        case .immutable:
            return "Immutable"
        case .outdated:
            return "Outdated"
        }
    }

    private var foreground: Color {
        switch badge {
        case .new:
            return AVGlassPalette.orange
        case .vulnerable:
            return AVGlassPalette.vulnerableText
        case .hardened:
            return AVGlassPalette.green
        case .immutable:
            return AVGlassPalette.cyan
        case .outdated:
            return AVGlassPalette.orange
        }
    }

    private var background: Color {
        switch badge {
        case .new:
            return AVGlassPalette.orange.opacity(orangeFillOpacity)
        case .vulnerable:
            return AVGlassPalette.vulnerableFill
        case .hardened:
            return AVGlassPalette.green.opacity(0.14)
        case .immutable:
            return AVGlassPalette.cyan.opacity(0.14)
        case .outdated:
            return AVGlassPalette.orange.opacity(orangeFillOpacity)
        }
    }

    private var border: Color {
        switch badge {
        case .new:
            return AVGlassPalette.orange.opacity(orangeBorderOpacity)
        case .vulnerable:
            return AVGlassPalette.vulnerableBorder
        case .hardened:
            return AVGlassPalette.green.opacity(0.22)
        case .immutable:
            return AVGlassPalette.cyan.opacity(0.24)
        case .outdated:
            return AVGlassPalette.orange.opacity(orangeBorderOpacity)
        }
    }

    private var fontSize: CGFloat {
        switch size {
        case .regular:
            return 11
        case .inline:
            return 10
        }
    }

    private var fontWeight: Font.Weight {
        switch size {
        case .regular:
            return .semibold
        case .inline:
            return .bold
        }
    }

    private var horizontalPadding: CGFloat {
        switch size {
        case .regular:
            return 8
        case .inline:
            return 6
        }
    }

    private var height: CGFloat {
        switch size {
        case .regular:
            return 22
        case .inline:
            return 18
        }
    }

    private var orangeFillOpacity: Double {
        switch size {
        case .regular:
            return 0.14
        case .inline:
            return 0.24
        }
    }

    private var orangeBorderOpacity: Double {
        switch size {
        case .regular:
            return 0.28
        case .inline:
            return 0.55
        }
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
            .foregroundStyle(AVGlassPalette.quietText)
            .tracking(0.6)
    }
}

private struct PackageBadgeBanner: View {
    let badge: MainWindowPackageBadge

    var body: some View {
        Label(title, systemImage: systemImage)
        .font(.system(size: 14, weight: .bold))
        .foregroundStyle(foreground)
        .padding(.horizontal, 10)
        .frame(height: 32)
        .background(background, in: RoundedRectangle(cornerRadius: 10))
        .overlay(RoundedRectangle(cornerRadius: 10).stroke(border, lineWidth: 1))
    }

    private var title: String {
        switch badge {
        case .new:
            return "New"
        case .vulnerable:
            return "Vulnerable"
        case .hardened:
            return "Hardened"
        case .immutable:
            return "Immutable"
        case .outdated:
            return "Outdated"
        }
    }

    private var systemImage: String {
        switch badge {
        case .new:
            return "sparkles"
        case .vulnerable:
            return "exclamationmark.shield.fill"
        case .hardened:
            return "shield.fill"
        case .immutable:
            return "lock.fill"
        case .outdated:
            return "arrow.up.circle.fill"
        }
    }

    private var foreground: Color {
        switch badge {
        case .new:
            return AVGlassPalette.orange
        case .vulnerable:
            return AVGlassPalette.vulnerableText
        case .hardened:
            return AVGlassPalette.green
        case .immutable:
            return AVGlassPalette.cyan
        case .outdated:
            return AVGlassPalette.orange
        }
    }

    private var background: Color {
        switch badge {
        case .new:
            return AVGlassPalette.orange.opacity(0.14)
        case .vulnerable:
            return AVGlassPalette.vulnerableFill
        case .hardened:
            return AVGlassPalette.green.opacity(0.14)
        case .immutable:
            return AVGlassPalette.cyan.opacity(0.14)
        case .outdated:
            return AVGlassPalette.orange.opacity(0.14)
        }
    }

    private var border: Color {
        switch badge {
        case .new:
            return AVGlassPalette.orange.opacity(0.28)
        case .vulnerable:
            return AVGlassPalette.vulnerableBorder
        case .hardened:
            return AVGlassPalette.green.opacity(0.22)
        case .immutable:
            return AVGlassPalette.cyan.opacity(0.24)
        case .outdated:
            return AVGlassPalette.orange.opacity(0.28)
        }
    }
}

private struct InfoSection<Content: View>: View {
    let title: String
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 9) {
            SectionLabel(title)
            content
        }
    }
}

private struct PermissionRow: View {
    let icon: String
    let title: String
    let allowed: Bool

    var body: some View {
        HStack(spacing: 9) {
            Image(systemName: icon)
                .font(.system(size: 13, weight: .semibold))
                .frame(width: 18)
                .foregroundStyle(AVGlassPalette.quietText)
            Text(title)
                .font(.system(size: 13, weight: .regular))
                .foregroundStyle(AVGlassPalette.secondaryText)
            Spacer()
            Image(systemName: allowed ? "checkmark.circle" : "minus.circle")
                .font(.system(size: 14, weight: .bold))
                .foregroundStyle(allowed ? AVGlassPalette.green : AVGlassPalette.quietText)
        }
        .frame(height: 24)
    }
}

private struct BrowserToolbarButton: View {
    let systemName: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: systemName)
                .font(.system(size: 13, weight: .bold))
                .foregroundStyle(AVGlassPalette.quietText)
                .frame(width: 26, height: 26)
        }
        .buttonStyle(.plain)
        .contentShape(Rectangle())
    }
}

private struct LiquidGlassSurface: View {
    let material: Material
    let tint: Color

    var body: some View {
        Rectangle()
            .fill(material)
            .overlay(tint)
    }
}

private enum AVGlassPalette {
    static let windowTint = Color.black.opacity(0.18)
    static let topBarTint = Color.black.opacity(0.28)
    static let titleBarSeparator = Color.white.opacity(0.13)
    static let titleBarShadow = Color.black.opacity(0.12)
    static let sidebarTint = Color(red: 0.025, green: 0.050, blue: 0.075).opacity(0.28)
    static let packageTint = Color.black.opacity(0.20)
    static let dossierTint = Color.black.opacity(0.24)
    static let linksTint = Color.black.opacity(0.18)
    static let controlFill = Color.white.opacity(0.075)
    static let controlBorder = Color.white.opacity(0.22)
    static let selectedFill = Color.white.opacity(0.095)
    static let sidebarSelectedFill = AVGlassPalette.selectedFill
    static let hairline = Color.white.opacity(0.10)
    static let primaryText = Color.white.opacity(0.92)
    static let secondaryText = Color.white.opacity(0.64)
    static let quietText = Color.white.opacity(0.36)
    static let green = Color(red: 0.10, green: 0.86, blue: 0.58)
    static let orange = Color(red: 0.95, green: 0.58, blue: 0.25)
    static let red = Color(red: 1.00, green: 0.45, blue: 0.45)
    static let vulnerableRed = Color(red: 1.00, green: 0.13, blue: 0.18)
    static let vulnerableText = Color(red: 1.00, green: 0.18, blue: 0.22)
    static let vulnerableFill = Color(red: 1.00, green: 0.10, blue: 0.14).opacity(0.14)
    static let vulnerableBorder = Color(red: 1.00, green: 0.00, blue: 0.04).opacity(0.78)
    static let blue = Color(red: 0.55, green: 0.67, blue: 0.82)
    static let cyan = Color(red: 0.10, green: 0.52, blue: 1.00)
    static let purple = Color(red: 0.44, green: 0.10, blue: 0.48)
}
