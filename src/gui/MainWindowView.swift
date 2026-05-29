import SwiftUI
import WebKit

struct MainWindowView: View {
    @ObservedObject var model: MainWindowModel
    @State private var linkTab: MainWindowLinkTab = .homepage

    var body: some View {
        ZStack {
            background
            mainContent
            titleBarBackdrop
        }
        .frame(minWidth: 1380, minHeight: 760)
        .background(Color.clear)
        .preferredColorScheme(.dark)
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
            model.selectSection(section)
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
                model.activeSidebarSection == section
                    ? AVGlassPalette.primaryText
                    : AVGlassPalette.secondaryText
            )
            .padding(.horizontal, 7)
            .frame(height: 32)
            .contentShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
            .background {
                if model.activeSidebarSection == section {
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
                if model.activeSidebarSection == .outdated {
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
        if !model.isSearchActive,
           model.selectedSection == .newUpdated,
           case .available(let result) = package.item {
            return model.pulseListTimestampText(for: result)
        }
        return model.versionText(for: package)
    }

    private func packageRowBadges(for package: PackagePresentation) -> [MainWindowPackageBadge] {
        var badges = model.packageListBadges(for: package)
        if !model.isSearchActive,
           model.selectedSection == .newUpdated,
           case .available(let result) = package.item,
           result.isNewPulse {
            badges.append(.new)
        }
        return badges
    }

    private var dossierPanel: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                if let detail = model.selectedDetail,
                   let package = model.selectedPackage {
                    let warning = DossierSecurityWarningContent(detail: detail)

                    dossierHeader(
                        detail: detail,
                        package: package,
                        showsHazardWarning: warning != nil
                    )
                    if let warning {
                        securityWarningSection(warning: warning)
                    }
                    executableSection(detail: detail, package: package)
                    permissionsSection(detail: detail, package: package)
                    if warning == nil {
                        notesSection(detail: detail, package: package)
                    }
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
        package: PackagePresentation,
        showsHazardWarning: Bool
    ) -> some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline, spacing: 12) {
                HStack(alignment: .firstTextBaseline, spacing: 8) {
                    Text(model.displayName(for: package))
                        .font(.system(size: 22, weight: .semibold))
                        .foregroundStyle(AVGlassPalette.primaryText)
                        .lineLimit(1)
                        .truncationMode(.tail)
                    Text(model.versionText(for: package))
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(AVGlassPalette.quietText)
                        .lineLimit(1)
                        .truncationMode(.middle)
                    if model.isLoadingDetail {
                        ProgressView()
                            .controlSize(.small)
                            .frame(width: 14, height: 14)
                            .alignmentGuide(.firstTextBaseline) { dimensions in
                                dimensions[VerticalAlignment.bottom] - 2
                            }
                            .help("Refreshing dossier")
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)

                if let primaryAction = model.dossierPrimaryPackageAction(for: detail) {
                    dossierActionButton(
                        action: primaryAction,
                        detail: detail,
                        package: package
                    )
                    .fixedSize()
                }
            }

            if let badge = model.packageBadge(for: package),
               !(badge == .vulnerable && showsHazardWarning) {
                PackageBadgeBanner(badge: badge)
            }
        }
    }

    private func dossierActionButton(
        action primaryAction: PackageOperationKind,
        detail: PackageDetail,
        package: PackagePresentation
    ) -> some View {
        let primaryEnabled = model.canRequestDossierPackageAction(primaryAction, detail: detail)
        let isPrimaryActive = model.activePackageOperation?.kind == primaryAction
            && model.activePackageOperation?.displayName == model.displayName(for: package)

        return Button {
            model.requestDossierPackageAction(
                primaryAction,
                detail: detail,
                package: package
            )
        } label: {
            PackageDossierActionButtonLabel(
                action: primaryAction,
                isActive: isPrimaryActive
            )
        }
        .buttonStyle(.glass)
        .tint(.clear)
        .overlay {
            if primaryAction == .harden {
                HardenTraceStroke()
            }
        }
        .disabled(!primaryEnabled)
        .opacity(primaryEnabled || isPrimaryActive ? 1 : 0.42)
        .help(primaryAction.title)
    }

    private func executableSection(
        detail: PackageDetail,
        package: PackagePresentation
    ) -> some View {
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
                        if model.isHardened(package, detail: detail) {
                            Text("Hardened")
                                .font(.system(size: 10, weight: .semibold))
                                .foregroundStyle(AVGlassPalette.green)
                                .padding(.horizontal, 7)
                                .padding(.vertical, 3)
                                .background(AVGlassPalette.green.opacity(0.14), in: Capsule())
                                .padding(7)
                        }
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

    private func securityWarningSection(
        warning: DossierSecurityWarningContent
    ) -> some View {
        DossierSecurityWarningCard(warning: warning)
    }

    private func lastUpdatedSection(detail: PackageDetail) -> some View {
        InfoSection(title: "LAST UPDATED") {
            Text(model.relativeLastUpdatedText(for: detail))
                .font(.system(size: 13, weight: .regular))
                .foregroundStyle(AVGlassPalette.secondaryText)
        }
    }

    private func noteText(
        detail: PackageDetail,
        package: PackagePresentation
    ) -> String {
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
        HStack(spacing: 10) {
            LinkTabBar(selection: $linkTab)
                .frame(minWidth: 150, idealWidth: 162, maxWidth: 180)
                .layoutPriority(3)

            LinkURLBar(url: model.selectedURL(for: linkTab)) {
                model.open(url: model.selectedURL(for: linkTab))
            }
            .layoutPriority(1)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }

    @ViewBuilder
    private var linkBrowser: some View {
        if let url = model.selectedURL(for: linkTab) {
            PackageWebView(url: url)
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

private struct LinkTabBar: View {
    @Binding var selection: MainWindowLinkTab

    var body: some View {
        HStack(spacing: 0) {
            ForEach(MainWindowLinkTab.allCases) { tab in
                Button {
                    withAnimation(.easeInOut(duration: 0.14)) {
                        selection = tab
                    }
                } label: {
                    Text(title(for: tab))
                        .font(.system(size: 12, weight: .semibold))
                        .foregroundStyle(
                            selection == tab
                                ? AVGlassPalette.primaryText
                                : AVGlassPalette.secondaryText
                        )
                        .lineLimit(1)
                        .minimumScaleFactor(0.82)
                        .frame(maxWidth: .infinity)
                        .frame(height: 23)
                        .contentShape(RoundedRectangle(cornerRadius: 5, style: .continuous))
                }
                .buttonStyle(.plain)
                .background {
                    if selection == tab {
                        RoundedRectangle(cornerRadius: 5, style: .continuous)
                            .fill(AVGlassPalette.tabBarSelectedFill)
                    }
                }
                .accessibilityLabel(tab.title)
                .accessibilityAddTraits(selection == tab ? .isSelected : [])
            }
        }
        .padding(2)
        .frame(height: 27)
        .background(
            AVGlassPalette.tabBarFill,
            in: RoundedRectangle(cornerRadius: 7, style: .continuous)
        )
        .overlay(
            RoundedRectangle(cornerRadius: 7, style: .continuous)
                .stroke(AVGlassPalette.controlBorder.opacity(0.14), lineWidth: 1)
        )
    }

    private func title(for tab: MainWindowLinkTab) -> String {
        switch tab {
        case .homepage:
            return "Home"
        case .repository:
            return "Repo"
        case .documentation:
            return "Docs"
        }
    }
}

private struct PackageWebView: NSViewRepresentable {
    let url: URL

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
    }

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    final class Coordinator: NSObject, WKNavigationDelegate {
        var currentURL: URL?

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
        .frame(height: 15, alignment: .center)
        .contentShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
    }
}

private struct PackageDossierActionButtonLabel: View {
    let action: PackageOperationKind
    let isActive: Bool

    var body: some View {
        HStack(spacing: 7) {
            if isActive {
                ProgressView()
                    .controlSize(.small)
                    .frame(width: 14, height: 14)
            } else if action == .harden {
                Image(systemName: "shield.fill")
                    .font(.system(size: 11, weight: .semibold))
                    .symbolRenderingMode(.hierarchical)
            }
            Text(isActive ? action.progressTitle : action.title)
                .font(.system(size: 12, weight: .semibold))
                .lineLimit(1)
                .minimumScaleFactor(0.86)
        }
        .foregroundStyle(foreground)
        .frame(height: 28)
        .contentShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
    }

    private var foreground: Color {
        switch action {
        case .harden:
            return AVGlassPalette.green
        case .update:
            return AVGlassPalette.orange
        case .install, .uninstall:
            return AVGlassPalette.secondaryText
        }
    }
}

private struct HardenTraceStroke: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    private let cornerRadius: CGFloat = 8
    private let cycleDuration: TimeInterval = 1.90
    private let minimumTraceLength: CGFloat = 0.04
    private let maximumTraceLength: CGFloat = 0.29
    private let growDuration: CGFloat = 0.80
    private let holdDuration: CGFloat = 0.10
    private let pulseOffsetPerCycle: CGFloat = 0.08
    private let baseStrokeOpacity = 0.42
    private let traceStrokeWidth = 1.8
    private let shadowRadius = 4.0
    private let shadowOpacity = 0.76

    var body: some View {
        Group {
            if reduceMotion {
                strokeContent(travelPhase: 0.08, lengthPhase: 0.08)
            } else {
                TimelineView(.animation) { context in
                    let cycles = context.date.timeIntervalSinceReferenceDate / cycleDuration
                    let travelPhase = CGFloat(cycles.truncatingRemainder(dividingBy: 1))
                    let lengthPhase = CGFloat(
                        (cycles * (1 + Double(pulseOffsetPerCycle)))
                            .truncatingRemainder(dividingBy: 1)
                    )
                    strokeContent(travelPhase: travelPhase, lengthPhase: lengthPhase)
                }
            }
        }
        .allowsHitTesting(false)
    }

    private func strokeContent(travelPhase: CGFloat, lengthPhase: CGFloat) -> some View {
        let traceLength = reduceMotion ? maximumTraceLength : traceLength(at: lengthPhase)
        return ZStack {
            RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
                .strokeBorder(AVGlassPalette.hardenTrace.opacity(baseStrokeOpacity), lineWidth: 1)
            traceSegments(endingAt: travelPhase, length: traceLength)
        }
    }

    @ViewBuilder
    private func traceSegments(endingAt end: CGFloat, length: CGFloat) -> some View {
        let start = end - length
        if start >= 0 {
            traceSegment(from: start, to: end)
        } else {
            traceSegment(from: start + 1, to: 1)
            traceSegment(from: 0, to: end)
        }
    }

    private func traceLength(at phase: CGFloat) -> CGFloat {
        let wavePhase = phase.truncatingRemainder(dividingBy: 1)

        let wave: CGFloat
        if wavePhase < growDuration {
            wave = easedProgress(wavePhase / growDuration)
        } else if wavePhase < growDuration + holdDuration {
            wave = 1
        } else {
            let shrinkDuration = 1 - growDuration - holdDuration
            let progress = (wavePhase - growDuration - holdDuration) / shrinkDuration
            wave = 1 - easedProgress(progress)
        }
        return minimumTraceLength + ((maximumTraceLength - minimumTraceLength) * wave)
    }

    private func easedProgress(_ progress: CGFloat) -> CGFloat {
        let clamped = min(max(progress, 0), 1)
        return clamped * clamped * (3 - (2 * clamped))
    }

    private func traceSegment(from start: CGFloat, to end: CGFloat) -> some View {
        RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
            .trim(from: start, to: end)
            .stroke(
                AVGlassPalette.hardenTrace,
                style: StrokeStyle(
                    lineWidth: traceStrokeWidth,
                    lineCap: .round,
                    lineJoin: .round
                )
            )
            .shadow(
                color: AVGlassPalette.hardenTrace.opacity(shadowOpacity),
                radius: shadowRadius
            )
    }
}

struct DossierSecurityWarningContent: Equatable {
    let headline: String
    let body: String
    let reasons: [String]
    let detectorError: String?
    let caveats: PackageSecurityNotice.Caveats?
    let learnMoreURL: URL

    init?(detail: PackageDetail) {
        let detectorError = detail.securityState?.error?
            .trimmingCharacters(in: .whitespacesAndNewlines)
        let nonEmptyDetectorError = detectorError?.isEmpty == false ? detectorError : nil

        if let notice = detail.securityNotice {
            headline = notice.headline
            body = notice.body
            reasons = notice.reasons
            self.detectorError = nonEmptyDetectorError
            caveats = notice.caveats
            learnMoreURL = notice.learnMoreURL
            return
        }

        guard let securityState = detail.securityState,
              let nonEmptyDetectorError else {
            return nil
        }

        headline = "DETECTOR NEEDS REVIEW"
        body = "The detector for isotope:\(securityState.isotopeName) did not complete cleanly."
        reasons = securityState.reasons
        self.detectorError = nonEmptyDetectorError
        caveats = nil
        learnMoreURL = PackageSecurityNotice.defaultLearnMoreURL
    }

    var hasCaveats: Bool {
        switch caveats {
        case .paragraph(let paragraph):
            return paragraph.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
        case .bullets(let bullets):
            return bullets.contains {
                $0.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
            }
        case .none:
            return false
        }
    }
}

private struct DossierSecurityWarningCard: View {
    let warning: DossierSecurityWarningContent
    @Environment(\.openURL) private var openURL

    var body: some View {
        VStack(alignment: .leading, spacing: 11) {
            Text(warning.headline)
                .font(.system(size: 15, weight: .bold))
                .foregroundStyle(AVGlassPalette.vulnerableText)
                .lineLimit(nil)
                .textSelection(.enabled)

            DossierSecurityMarkdownText(
                warning.body,
                fontSize: 13,
                weight: .medium,
                color: AVGlassPalette.secondaryText
            )

            if warning.reasons.isEmpty == false {
                DossierSecurityWarningSection(title: "DETECTION") {
                    VStack(alignment: .leading, spacing: 7) {
                        ForEach(warning.reasons.indices, id: \.self) { index in
                            DossierSecurityWarningBullet(text: warning.reasons[index])
                        }
                    }
                }
            }

            if let detectorError = warning.detectorError {
                DossierSecurityWarningSection(title: "DETECTOR ERROR") {
                    Text(detectorError)
                        .font(.system(size: 11, weight: .regular, design: .monospaced))
                        .foregroundStyle(AVGlassPalette.secondaryText)
                        .lineSpacing(2)
                        .fixedSize(horizontal: false, vertical: true)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .textSelection(.enabled)
                }
            }

            if warning.hasCaveats {
                DossierSecurityWarningSection(title: "CAVEATS") {
                    caveatsContent
                }
            }

            Button {
                openURL(warning.learnMoreURL)
            } label: {
                Text("LEARN MORE")
                    .font(.system(size: 12, weight: .bold))
                    .foregroundStyle(AVGlassPalette.vulnerableText)
                    .tracking(0.6)
                    .frame(maxWidth: .infinity)
                    .frame(height: 32)
                    .background(AVGlassPalette.controlFill, in: RoundedRectangle(cornerRadius: 5))
                    .overlay(
                        RoundedRectangle(cornerRadius: 5)
                            .stroke(AVGlassPalette.vulnerableBorder.opacity(0.66), lineWidth: 1)
                    )
            }
            .buttonStyle(.plain)
            .padding(.top, 3)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(12)
        .background(AVGlassPalette.vulnerableFill, in: RoundedRectangle(cornerRadius: 8))
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(AVGlassPalette.vulnerableBorder.opacity(0.72), lineWidth: 1)
        )
    }

    @ViewBuilder
    private var caveatsContent: some View {
        switch warning.caveats {
        case .paragraph(let paragraph):
            DossierSecurityMarkdownText(
                paragraph,
                fontSize: 12,
                weight: .regular,
                color: AVGlassPalette.secondaryText
            )
        case .bullets(let bullets):
            VStack(alignment: .leading, spacing: 7) {
                ForEach(bullets.indices, id: \.self) { index in
                    DossierSecurityWarningBullet(text: bullets[index])
                }
            }
        case .none:
            EmptyView()
        }
    }
}

private struct DossierSecurityWarningSection<Content: View>: View {
    let title: String
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(AVGlassPalette.vulnerableText.opacity(0.86))
                .tracking(0.7)
            content
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct DossierSecurityWarningBullet: View {
    let text: String

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 7) {
            Text("•")
                .font(.system(size: 12, weight: .bold))
                .foregroundStyle(AVGlassPalette.vulnerableText.opacity(0.82))
            DossierSecurityMarkdownText(
                text,
                fontSize: 12,
                weight: .regular,
                color: AVGlassPalette.secondaryText
            )
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct DossierSecurityMarkdownText: View {
    let text: String
    let fontSize: CGFloat
    let weight: Font.Weight
    let color: Color

    init(
        _ text: String,
        fontSize: CGFloat,
        weight: Font.Weight,
        color: Color
    ) {
        self.text = text
        self.fontSize = fontSize
        self.weight = weight
        self.color = color
    }

    var body: some View {
        Text(attributedText)
            .font(.system(size: fontSize, weight: weight))
            .foregroundStyle(color)
            .lineSpacing(3)
            .fixedSize(horizontal: false, vertical: true)
            .frame(maxWidth: .infinity, alignment: .leading)
            .textSelection(.enabled)
    }

    private var attributedText: AttributedString {
        let normalized = normalizedSecurityMarkdown(text)
        do {
            return try AttributedString(
                markdown: normalized,
                options: AttributedString.MarkdownParsingOptions(
                    interpretedSyntax: .inlineOnlyPreservingWhitespace,
                    failurePolicy: .returnPartiallyParsedIfPossible
                )
            )
        } catch {
            return AttributedString(normalized)
        }
    }

    private func normalizedSecurityMarkdown(_ markdown: String) -> String {
        let lines = markdown
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .components(separatedBy: .newlines)
        var blocks: [[String]] = []
        var currentBlock: [String] = []

        for line in lines {
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            if trimmed.isEmpty {
                if currentBlock.isEmpty == false {
                    blocks.append(currentBlock)
                    currentBlock = []
                }
            } else {
                currentBlock.append(trimmed)
            }
        }

        if currentBlock.isEmpty == false {
            blocks.append(currentBlock)
        }

        return blocks
            .map { block in
                if block.contains(where: isMarkdownListItem) {
                    return block.joined(separator: "\n")
                }
                return block.joined(separator: " ")
            }
            .joined(separator: "\n\n")
    }

    private func isMarkdownListItem(_ line: String) -> Bool {
        line.hasPrefix("- ") || line.hasPrefix("* ")
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
                            PackageInlineBadgeText(badge: badge)
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

private struct PackageInlineBadgeText: View {
    let badge: MainWindowPackageBadge

    var body: some View {
        Text(title)
            .font(.system(size: 9, weight: .regular))
            .foregroundStyle(foreground)
            .kerning(1.2)
            .lineLimit(1)
            .fixedSize(horizontal: true, vertical: false)
            .alignmentGuide(.firstTextBaseline) { dimensions in
                dimensions[.firstTextBaseline]
            }
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
            return "OUTDATED"
        }
    }

    private var foreground: Color {
        switch badge {
        case .new, .outdated:
            return AVGlassPalette.orange
        case .vulnerable:
            return AVGlassPalette.vulnerableText
        case .hardened:
            return AVGlassPalette.green
        case .immutable:
            return AVGlassPalette.cyan
        }
    }
}

private struct PackageBadgePill: View {
    let badge: MainWindowPackageBadge

    var body: some View {
        Text(title)
            .font(.system(size: 11, weight: .semibold))
            .foregroundStyle(foreground)
            .padding(.horizontal, 8)
            .frame(height: 22)
            .background(background, in: Capsule())
            .overlay(Capsule().stroke(border, lineWidth: 1))
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

private struct SectionLabel: View {
    let title: String

    init(_ title: String) {
        self.title = title
    }

    var body: some View {
        Text(title)
            .font(.system(size: 11, weight: .semibold))
            .foregroundStyle(AVGlassPalette.quietText)
            .kerning(1.2)
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
        .background(background, in: Capsule())
        .overlay(Capsule().stroke(border, lineWidth: 1))
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

private struct LinkURLBar: View {
    let url: URL?
    let open: () -> Void

    var body: some View {
        HStack(spacing: 8) {
            Text(displayedURLString)
                .font(.system(size: 13, weight: .semibold, design: .monospaced))
                .foregroundStyle(url == nil ? AVGlassPalette.quietText : AVGlassPalette.secondaryText)
                .lineLimit(1)
                .truncationMode(.middle)
                .frame(maxWidth: .infinity, alignment: .leading)

            Button(action: open) {
                Image(systemName: "arrow.up.right.square")
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(url == nil ? AVGlassPalette.quietText : AVGlassPalette.secondaryText)
                    .frame(width: 24, height: 24)
            }
            .buttonStyle(.plain)
            .disabled(url == nil)
            .help("Open externally")
        }
        .padding(.leading, 12)
        .padding(.trailing, 5)
        .frame(height: 32)
        .frame(minWidth: 140, maxWidth: .infinity)
        .glassEffect(.regular, in: RoundedRectangle(cornerRadius: 10, style: .continuous))
    }

    private var displayedURLString: String {
        guard let urlString = url?.absoluteString else {
            return ""
        }
        return urlString.strippingPrefix("https://") ?? urlString
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
    static let tabBarFill = Color.white.opacity(0.055)
    static let tabBarSelectedFill = AVGlassPalette.selectedFill
    static let hairline = Color.white.opacity(0.10)
    static let primaryText = Color.white.opacity(0.92)
    static let secondaryText = Color.white.opacity(0.64)
    static let quietText = Color.white.opacity(0.36)
    static let green = Color(red: 0.10, green: 0.86, blue: 0.58)
    static let hardenTrace = Color(red: 0.00, green: 1.00, blue: 0.50)
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
