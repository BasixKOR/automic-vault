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
                if model.selectedSection == .dashboard {
                    dashboardPanel
                        .frame(width: max(width - sidebarWidth - 1, 720))
                } else {
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
    }

    private var sidebar: some View {
        VStack(alignment: .leading, spacing: 0) {
            sidebarHeader("AUTOMIC VAULT")
                .kerning(1.2)
                .padding(.top, 26)
            ForEach(MainWindowSection.librarySections) { section in
                sidebarRow(section)
            }

            sidebarHeader(L10n.string("CATEGORIES"))
                .padding(.top, 22)
                .kerning(1.2)
            ForEach(MainWindowSection.categorySections) { section in
                sidebarRow(section)
            }
            sidebarDivider
            ForEach(MainWindowSection.categoryShortcutSections) { section in
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

    private var sidebarDivider: some View {
        Rectangle()
            .fill(AVGlassPalette.hairline)
            .frame(height: 1)
            .padding(.horizontal, 7)
            .padding(.vertical, 6)
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
                if let count = model.count(for: section),
                   section.shouldDisplaySidebarCount(count) {
                    if section == .geigerCounter && section.shouldHighlightSidebarCount(count) {
                        CountPill(count: count, prominence: .critical)
                            .fixedSize()
                    } else if section.shouldHighlightSidebarCount(count) {
                        CountPill(count: count, prominence: .normal)
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
                Text(L10n.string("Package"))
                Image(systemName: "arrow.up.arrow.down")
                    .font(.system(size: 11, weight: .bold))
                Spacer()
                if model.shouldShowCategorySortControl && shouldShowPackageListSpinner {
                    packageListSpinner
                }
                if model.shouldShowCategorySortControl {
                    Menu {
                        ForEach(CategoryPackageSortOrder.allCases) { sortOrder in
                            Button {
                                model.selectCategorySortOrder(sortOrder)
                            } label: {
                                if model.categoryPackageSortOrder == sortOrder {
                                    Label(sortOrder.title, systemImage: "checkmark")
                                } else {
                                    Text(sortOrder.title)
                                }
                            }
                        }
                    } label: {
                        PackageListHeaderButtonLabel(
                            systemImage: nil,
                            title: model.categorySortButtonTitle
                        )
                    }
                    .buttonStyle(.glass)
                    .tint(.clear)
                    .help(L10n.string("Choose category sort order"))
                    .fixedSize(horizontal: true, vertical: false)
                    .offset(y: 2)
                }
                if model.activeSidebarSection == .outdated {
                    Button {
                        model.requestOutdatedUpdateAll()
                    } label: {
                        PackageListHeaderButtonLabel(
                            systemImage: "arrow.triangle.2.circlepath",
                            title: model.isUpdatingAll
                                ? L10n.string("Updating")
                                : L10n.string("Update All")
                        )
                    }
                    .buttonStyle(.glass)
                    .tint(.clear)
                    .disabled(!model.canUpdateAllOutdated)
                    .opacity(model.canUpdateAllOutdated ? 1 : 0.42)
                    .help(updateAllHelpText)
                    .offset(y: 2)
                }
                if !model.shouldShowCategorySortControl && shouldShowPackageListSpinner {
                    packageListSpinner
                }
            }
            .font(.system(size: 13, weight: .bold))
            .foregroundStyle(AVGlassPalette.quietText)
            .padding(.leading, 18)
            .padding(.trailing, 7)
            .frame(height: 42)

            hairline

            let versionedPackageBases = packageListVersionedPackageBases
            ScrollView {
                LazyVStack(spacing: 0) {
                    ForEach(model.displayedPackages, id: \.selectionID) { package in
                        PackageRow(
                            package: package,
                            title: packageRowTitle(
                                for: package,
                                versionedPackageBases: versionedPackageBases
                            ),
                            description: model.packageDescription(for: package),
                            version: packageRowVersion(for: package),
                            inlineBadges: model.packageInlineBadges(for: package),
                            severityLevel: model.securityRecommendationSeverityLevel(for: package),
                            badges: model.packageListBadges(for: package),
                            selected: model.selectedItemID == package.selectionID
                        ) {
                            model.select(package)
                        }
                        .onAppear {
                            model.loadNextPageIfNeeded(after: package)
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

    private var dashboardPanel: some View {
        let summary = model.dashboardSummary
        let isLoading = model.isReloading || model.isLoadingSectionPage
        return ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                HStack(alignment: .top, spacing: 18) {
                    DashboardDonutCard(summary: summary, isLoading: isLoading)
                        .frame(maxWidth: 520)
                    DashboardStatsPanel(
                        summary: summary,
                        isLoading: isLoading
                    )
                    .frame(maxWidth: 420)
                }

                HStack(alignment: .top, spacing: 18) {
                    dashboardPackageSection(
                        title: L10n.string("New Packages"),
                        packages: summary.newPackages,
                        emptyText: L10n.string("No new packages loaded yet"),
                        isLoading: isLoading
                    )
                    dashboardPackageSection(
                        title: L10n.string("Recently Updated"),
                        packages: summary.recentlyUpdatedPackages,
                        emptyText: L10n.string("No recent updates loaded yet"),
                        isLoading: isLoading
                    )
                    dashboardPackageSection(
                        title: L10n.string("Outdated AV Packages"),
                        badgeCount: summary.outdatedPackageCount,
                        packages: summary.outdatedPackages,
                        emptyText: L10n.string("No outdated AV packages"),
                        isLoading: isLoading
                    )
                }
            }
            .padding(.horizontal, 26)
            .padding(.vertical, 26)
            .frame(maxWidth: .infinity, alignment: .topLeading)
        }
        .scrollIndicators(.visible)
        .background {
            LiquidGlassSurface(
                material: .thinMaterial,
                tint: AVGlassPalette.packageTint
            )
        }
    }

    private func dashboardPackageSection(
        title: String,
        badgeCount: Int? = nil,
        packages: [PackagePresentation],
        emptyText: String,
        isLoading: Bool = false
    ) -> some View {
        DashboardSectionCard(title: title, badgeCount: badgeCount) {
            if packages.isEmpty {
                if isLoading {
                    ProgressView()
                        .controlSize(.small)
                        .frame(maxWidth: .infinity, minHeight: 106, alignment: .center)
                } else {
                    Text(emptyText)
                        .font(.system(size: 13, weight: .medium))
                        .foregroundStyle(AVGlassPalette.quietText)
                        .frame(maxWidth: .infinity, minHeight: 106, alignment: .center)
                }
            } else {
                VStack(spacing: 0) {
                    ForEach(packages, id: \.selectionID) { package in
                        DashboardPackageRow(
                            title: model.displayName(for: package),
                            subtitle: model.packageDescription(for: package),
                            trailing: model.versionText(for: package)
                        )
                        if package.selectionID != packages.last?.selectionID {
                            hairline
                        }
                    }
                }
            }
        }
        .frame(maxWidth: .infinity)
    }

    private var shouldShowPackageListSpinner: Bool {
        model.isReloading
            || model.isSearching
            || model.isLoadingSectionPage
            || model.isUpdatingAll
    }

    private var packageListSpinner: some View {
        ProgressView()
            .controlSize(.small)
    }

    private var updateAllHelpText: String {
        let count = model.outdatedUpdatePackageNames.count
        guard count > 0 else {
            return L10n.string("No outdated packages to update")
        }
        return count == 1
            ? L10n.string("Update 1 outdated package")
            : L10n.format("Update %d outdated packages", count)
    }

    private func packageRowVersion(for package: PackagePresentation) -> String {
        if !model.isSearchActive,
           model.selectedSection == .newUpdated,
           case .available(let result) = package.item {
            return model.pulseListTimestampText(for: result)
        }
        return model.versionText(for: package)
    }

    private var packageListVersionedPackageBases: Set<String> {
        guard model.isSearchActive else {
            return []
        }
        return Set(
            model.displayedPackages.compactMap { package in
                PackageDisplayTitle.versionedBase(displayName: model.displayName(for: package))
            }
        )
    }

    private func packageRowTitle(
        for package: PackagePresentation,
        versionedPackageBases: Set<String>
    ) -> PackageDisplayTitle {
        let title = model.displayName(for: package)
        guard model.isSearchActive else {
            return PackageDisplayTitle(name: title)
        }
        return PackageDisplayTitle(
            displayName: title,
            latestVersionedBases: versionedPackageBases
        )
    }

    private var dossierPanel: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                if let detail = model.selectedDetail,
                   let package = model.selectedPackage {
                    let warning = DossierSecurityWarningContent(detail: detail)
                    let hardeningSummary = warning == nil
                        ? model.dossierHardeningSummary(for: detail, package: package)
                        : nil

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
                        notesSection(
                            detail: detail,
                            package: package,
                            hardeningSummary: hardeningSummary
                        )
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
                    Text(model.dossierVersionText(for: package, detail: detail))
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
                            .help(L10n.string("Refreshing dossier"))
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
        InfoSection(title: L10n.string("EXECUTABLES")) {
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
                            Text(L10n.string("Hardened"))
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
                    Text(L10n.format("%d more", paths.count - 2))
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
        InfoSection(title: L10n.string("PERMISSIONS")) {
            VStack(spacing: 8) {
                PermissionRow(icon: "network", title: L10n.string("Network Access"), allowed: true)
                PermissionRow(icon: "folder", title: L10n.string("File System"), allowed: true)
                PermissionRow(
                    icon: "point.3.connected.trianglepath.dotted",
                    title: L10n.string("Process Spawning"),
                    allowed: true
                )
                PermissionRow(
                    icon: "key",
                    title: L10n.string("Secrets Access"),
                    allowed: model.isHardened(package) || detail.securityNotice != nil
                )
            }
        }
    }

    private func notesSection(
        detail: PackageDetail,
        package: PackagePresentation,
        hardeningSummary: PackageHardeningSummary?
    ) -> some View {
        InfoSection(title: L10n.string("NOTES")) {
            if let hardeningSummary {
                DossierHardeningSummaryCard(summary: hardeningSummary)
            } else {
                Text(noteText(detail: detail, package: package))
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(AVGlassPalette.secondaryText)
                    .lineSpacing(3)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(12)
                    .background(AVGlassPalette.controlFill, in: RoundedRectangle(cornerRadius: 8))
            }
        }
    }

    private func securityWarningSection(
        warning: DossierSecurityWarningContent
    ) -> some View {
        DossierSecurityWarningCard(warning: warning)
    }

    private func lastUpdatedSection(detail: PackageDetail) -> some View {
        InfoSection(title: L10n.string("LAST UPDATED")) {
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
            return L10n.string(
                "This package is hardened. Binary execution is sandboxed and secret access is restricted."
            )
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
        let hasSelectedPackage = model.selectedPackage != nil
        let highlightedTab = model.highlightedLinkTab(for: linkTab)

        return HStack(spacing: 10) {
            LinkTabBar(selection: $linkTab, highlightedTab: highlightedTab)
                .frame(minWidth: 150, idealWidth: 162, maxWidth: 180)
                .layoutPriority(3)
                .disabled(!hasSelectedPackage)

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
        if model.selectedSection == .settings {
            settingsPanel
        } else if let url = model.selectedURL(for: linkTab) {
            PackageWebView(url: url)
                .id(url)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else {
            Color.clear
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }

    private var settingsPanel: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 22) {
                Text(L10n.string("Settings"))
                    .font(.system(size: 24, weight: .semibold))
                    .foregroundStyle(AVGlassPalette.primaryText)

                VStack(alignment: .leading, spacing: 12) {
                    Text(L10n.string("Dotenv approvals"))
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(AVGlassPalette.primaryText)

                    Toggle(isOn: dotenvRememberApprovedBinding) {
                        VStack(alignment: .leading, spacing: 4) {
                            Text(L10n.string("Remember approved dotenv files"))
                                .font(.system(size: 13, weight: .medium))
                                .foregroundStyle(AVGlassPalette.secondaryText)
                            Text(dotenvApprovalPolicyStatusText)
                                .font(.system(size: 12, weight: .regular))
                                .foregroundStyle(AVGlassPalette.quietText)
                                .fixedSize(horizontal: false, vertical: true)
                        }
                    }
                    .toggleStyle(.switch)
                    .disabled(
                        model.isLoadingDotenvApprovalPolicy
                            || model.isUpdatingDotenvApprovalPolicy
                    )

                    if model.isLoadingDotenvApprovalPolicy || model.isUpdatingDotenvApprovalPolicy {
                        HStack(spacing: 8) {
                            ProgressView()
                                .controlSize(.small)
                            Text(L10n.string("Updating dotenv approval policy"))
                                .font(.system(size: 12, weight: .regular))
                                .foregroundStyle(AVGlassPalette.quietText)
                        }
                    }

                    if let error = model.dotenvApprovalPolicyError {
                        Text(error)
                            .font(.system(size: 12, weight: .regular))
                            .foregroundStyle(AVGlassPalette.vulnerableText)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                }
                .padding(16)
                .frame(maxWidth: 520, alignment: .leading)
                .background(
                    AVGlassPalette.controlFill.opacity(0.55),
                    in: RoundedRectangle(cornerRadius: 8, style: .continuous)
                )
                .overlay(
                    RoundedRectangle(cornerRadius: 8, style: .continuous)
                        .stroke(AVGlassPalette.controlBorder.opacity(0.18), lineWidth: 1)
                )
            }
            .padding(.horizontal, 24)
            .padding(.vertical, 24)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .scrollIndicators(.visible)
    }

    private var dotenvRememberApprovedBinding: Binding<Bool> {
        Binding(
            get: { model.dotenvApprovalPolicy == .rememberApproved },
            set: { isEnabled in
                model.requestDotenvApprovalPolicy(
                    isEnabled ? .rememberApproved : .approveEveryTime
                )
            }
        )
    }

    private var dotenvApprovalPolicyStatusText: String {
        switch model.dotenvApprovalPolicy {
        case .approveEveryTime:
            return L10n.string("Approve every dotenv export and run request.")
        case .rememberApproved:
            return L10n.string("Approved dotenv files can be reused while their digest and key list stay unchanged.")
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

private struct DashboardStatsPanel: View {
    let summary: DashboardSummary
    let isLoading: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack(alignment: .top, spacing: 28) {
                DashboardStatValue(
                    title: L10n.string("Database"),
                    value: summary.databasePackageCount.map { $0.formatted() } ?? "--"
                )
                DashboardStatValue(
                    title: L10n.string("Categories"),
                    value: summary.databaseCategoryCount > 0
                        ? summary.databaseCategoryCount.formatted()
                        : "--"
                )
                if isLoading {
                    ProgressView()
                        .controlSize(.small)
                        .frame(width: 18, height: 54, alignment: .center)
                }
            }

            DashboardSourceBreakdown(sourceCounts: summary.databaseSourceCounts, isLoading: isLoading)
        }
        .padding(18)
        .frame(maxWidth: .infinity, alignment: .topLeading)
        .background(
            AVGlassPalette.controlFill.opacity(0.58),
            in: RoundedRectangle(cornerRadius: 8, style: .continuous)
        )
        .overlay(
            RoundedRectangle(cornerRadius: 8, style: .continuous)
                .stroke(AVGlassPalette.controlBorder.opacity(0.16), lineWidth: 1)
        )
    }
}

private struct DashboardStatValue: View {
    let title: String
    let value: String

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title.uppercased())
                .font(.system(size: 10, weight: .bold))
                .foregroundStyle(AVGlassPalette.quietText)
                .tracking(0.7)
                .lineLimit(1)
            Text(value)
                .font(.system(size: 24, weight: .semibold))
                .foregroundStyle(AVGlassPalette.primaryText)
                .monospacedDigit()
                .lineLimit(1)
                .minimumScaleFactor(0.72)
        }
        .frame(minWidth: 88, minHeight: 54, alignment: .leading)
    }
}

private struct DashboardSourceBreakdown: View {
    let sourceCounts: [(String, Int)]
    let isLoading: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 11) {
            Text(L10n.string("Package Managers").uppercased())
                .font(.system(size: 10, weight: .bold))
                .foregroundStyle(AVGlassPalette.quietText)
                .tracking(0.7)

            if sourceCounts.isEmpty {
                if isLoading {
                    ProgressView()
                        .controlSize(.small)
                        .frame(maxWidth: .infinity, minHeight: 48, alignment: .center)
                } else {
                    Text(L10n.string("No package manager counts loaded yet"))
                        .font(.system(size: 13, weight: .medium))
                        .foregroundStyle(AVGlassPalette.quietText)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            } else {
                VStack(spacing: 8) {
                    ForEach(Array(sourceCounts.prefix(5).enumerated()), id: \.offset) { _, source in
                        HStack {
                            Text(source.0)
                                .font(.system(size: 13, weight: .medium))
                                .foregroundStyle(AVGlassPalette.secondaryText)
                                .lineLimit(1)
                            Spacer(minLength: 10)
                            Text(source.1.formatted())
                                .font(.system(size: 13, weight: .semibold))
                                .foregroundStyle(AVGlassPalette.primaryText)
                                .monospacedDigit()
                        }
                    }
                }
            }
        }
    }
}

private struct DashboardDonutCard: View {
    let summary: DashboardSummary
    let isLoading: Bool

    var body: some View {
        DashboardSectionCard(title: L10n.string("Package Posture")) {
            HStack(alignment: .center, spacing: 24) {
                DashboardDonutChart(slices: summary.slices)
                    .frame(width: 190, height: 190)
                    .overlay {
                        if isLoading && summary.totalPackages == 0 {
                            ProgressView()
                                .controlSize(.small)
                        } else {
                            VStack(spacing: 3) {
                                Text(summary.totalPackages.formatted())
                                    .font(.system(size: 32, weight: .semibold))
                                    .foregroundStyle(AVGlassPalette.primaryText)
                                    .monospacedDigit()
                                Text(L10n.string("Installed"))
                                    .font(.system(size: 12, weight: .semibold))
                                    .foregroundStyle(AVGlassPalette.quietText)
                            }
                        }
                    }

                VStack(spacing: 10) {
                    ForEach(summary.slices) { slice in
                        HStack(spacing: 9) {
                            Circle()
                                .fill(dashboardSliceColor(slice.id))
                                .frame(width: 9, height: 9)
                            Text(slice.title)
                                .font(.system(size: 13, weight: .medium))
                                .foregroundStyle(AVGlassPalette.secondaryText)
                                .lineLimit(1)
                            Spacer(minLength: 8)
                            Group {
                                if slice.id == "security-alerts" && slice.count > 0 {
                                    DashboardCountPill(
                                        count: slice.count,
                                        color: dashboardSliceColor(slice.id)
                                    )
                                } else {
                                    Text(slice.count.formatted())
                                        .font(.system(size: 13, weight: .semibold))
                                        .foregroundStyle(AVGlassPalette.primaryText)
                                        .monospacedDigit()
                                }
                            }
                            .frame(width: 34, alignment: .trailing)
                        }
                    }
                }
            }
        }
    }
}

private struct DashboardSectionCard<Content: View>: View {
    let title: String
    let badgeCount: Int?
    @ViewBuilder let content: Content

    init(
        title: String,
        badgeCount: Int? = nil,
        @ViewBuilder content: () -> Content
    ) {
        self.title = title
        self.badgeCount = badgeCount
        self.content = content()
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 13) {
            HStack(spacing: 8) {
                Text(title.uppercased())
                    .font(.system(size: 11, weight: .bold))
                    .foregroundStyle(AVGlassPalette.quietText)
                    .tracking(0.8)
                if let badgeCount, badgeCount > 0 {
                    DashboardCountPill(count: badgeCount, color: AVGlassPalette.orange)
                }
            }
            content
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .topLeading)
        .background(
            AVGlassPalette.controlFill.opacity(0.58),
            in: RoundedRectangle(cornerRadius: 8, style: .continuous)
        )
        .overlay(
            RoundedRectangle(cornerRadius: 8, style: .continuous)
                .stroke(AVGlassPalette.controlBorder.opacity(0.16), lineWidth: 1)
        )
    }
}

private struct DashboardCountPill: View {
    let count: Int
    let color: Color

    var body: some View {
        Text(count.formatted())
            .font(.system(size: 11, weight: .bold))
            .foregroundStyle(color)
            .monospacedDigit()
            .padding(.horizontal, 7)
            .padding(.vertical, 2)
            .background(color.opacity(0.14), in: Capsule())
    }
}

private struct DashboardPackageRow: View {
    let title: String
    let subtitle: String
    let trailing: String

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(title)
                .font(.system(size: 13, weight: .semibold))
                .foregroundStyle(AVGlassPalette.primaryText)
                .lineLimit(1)
                .frame(maxWidth: .infinity, alignment: .leading)
            Text(subtitle)
                .font(.system(size: 12, weight: .regular))
                .foregroundStyle(AVGlassPalette.quietText)
                .lineLimit(1)
                .frame(maxWidth: .infinity, alignment: .leading)
            Text(trailing)
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(AVGlassPalette.secondaryText)
                .lineLimit(1)
                .frame(maxWidth: .infinity, alignment: .trailing)
        }
        .padding(.vertical, 9)
    }
}

private struct DashboardDonutChart: View {
    let slices: [DashboardPackageSlice]

    private var total: Int {
        slices.reduce(0) { $0 + max($1.count, 0) }
    }

    var body: some View {
        ZStack {
            if total == 0 {
                Circle()
                    .stroke(AVGlassPalette.controlBorder.opacity(0.22), lineWidth: 26)
            } else {
                ForEach(Array(slices.enumerated()), id: \.element.id) { index, slice in
                    DonutSegment(
                        start: startFraction(at: index),
                        end: endFraction(at: index)
                    )
                    .stroke(
                        dashboardSliceColor(slice.id),
                        style: StrokeStyle(lineWidth: 26, lineCap: .butt)
                    )
                }
            }
        }
        .rotationEffect(.degrees(-90))
        .padding(13)
        .accessibilityLabel(L10n.string("Package posture chart"))
    }

    private func startFraction(at index: Int) -> Double {
        fraction(slices.prefix(index).reduce(0) { $0 + max($1.count, 0) })
    }

    private func endFraction(at index: Int) -> Double {
        fraction(slices.prefix(index + 1).reduce(0) { $0 + max($1.count, 0) })
    }

    private func fraction(_ value: Int) -> Double {
        guard total > 0 else { return 0 }
        return Double(value) / Double(total)
    }
}

private struct DonutSegment: Shape {
    let start: Double
    let end: Double

    func path(in rect: CGRect) -> Path {
        var path = Path()
        let center = CGPoint(x: rect.midX, y: rect.midY)
        let radius = min(rect.width, rect.height) / 2
        path.addArc(
            center: center,
            radius: radius,
            startAngle: .degrees(start * 360),
            endAngle: .degrees(end * 360),
            clockwise: false
        )
        return path
    }
}

private func dashboardSliceColor(_ id: String) -> Color {
    switch id {
    case "hardened":
        return AVGlassPalette.green
    case "immutable":
        return AVGlassPalette.blue
    case "mutable":
        return AVGlassPalette.orange
    case "security-alerts":
        return AVGlassPalette.vulnerableText
    default:
        return AVGlassPalette.secondaryText
    }
}

private struct LinkTabBar: View {
    @Environment(\.isEnabled) private var isEnabled
    @Binding var selection: MainWindowLinkTab
    let highlightedTab: MainWindowLinkTab

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
                            highlightedTab == tab
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
                    if highlightedTab == tab {
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
        .opacity(isEnabled ? 1 : 0.45)
    }

    private func title(for tab: MainWindowLinkTab) -> String {
        switch tab {
        case .homepage:
            return L10n.string("Home")
        case .repository:
            return L10n.string("Repo")
        case .documentation:
            return L10n.string("Docs")
        }
    }
}

private struct PackageWebView: NSViewRepresentable {
    let url: URL

    func makeNSView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .default()
        configuration.userContentController.addUserScript(Self.defaultStyleScript)

        let webView = WKWebView(frame: .zero, configuration: configuration)
        webView.allowsBackForwardNavigationGestures = true
        webView.underPageBackgroundColor = .clear
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

    private static let defaultStyleScript = WKUserScript(
        source: #"""
        (() => {
          const style = document.createElement("style");
          style.dataset.automicVaultDefaults = "true";
          style.textContent = `
            @layer automic-vault-defaults {
              :where(html) {
                background-color: white;
                color: black;
              }

              :where(body) {
                background-color: white;
                color: black;
              }
            }
          `;

          const install = () => {
            const target = document.head || document.documentElement;
            if (!target) {
              return false;
            }
            target.prepend(style);
            return true;
          };

          if (!install()) {
            document.addEventListener("DOMContentLoaded", install, { once: true });
          }
        })();
        """#,
        injectionTime: .atDocumentStart,
        forMainFrameOnly: false
    )

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

private enum SidebarCountMetrics {
    static let columnWidth: CGFloat = 18
    static let pillHorizontalPadding: CGFloat = 8
}

private struct SidebarCountText: View {
    let count: Int

    var body: some View {
        Text(count.formatted())
            .font(.system(size: 12, weight: .regular))
            .monospacedDigit()
            .foregroundStyle(foreground)
            .lineLimit(1)
            .frame(minWidth: SidebarCountMetrics.columnWidth, alignment: .trailing)
    }

    private var foreground: Color {
        count == 0
            ? AVGlassPalette.secondaryText.opacity(0.68)
            : AVGlassPalette.secondaryText
    }
}

private struct PackageListHeaderButtonLabel: View {
    let systemImage: String?
    let title: String

    var body: some View {
        HStack(spacing: 6) {
            if let systemImage {
                Image(systemName: systemImage)
                    .font(.system(size: 10, weight: .regular))
                    .symbolRenderingMode(.hierarchical)
            }
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

private struct DossierHardeningSummaryCard: View {
    let summary: PackageHardeningSummary
    @Environment(\.openURL) private var openURL

    var body: some View {
        VStack(alignment: .leading, spacing: 11) {
            Text(summary.headline)
                .font(.system(size: 15, weight: .bold))
                .foregroundStyle(AVGlassPalette.green)
                .lineLimit(nil)
                .textSelection(.enabled)

            DossierSecurityMarkdownText(
                summary.body,
                fontSize: 13,
                weight: .medium,
                color: AVGlassPalette.secondaryText
            )

            VStack(alignment: .leading, spacing: 7) {
                if let hardenedPackageName = summary.hardenedPackageName {
                    DossierHardeningSummaryFact(
                        label: L10n.string("Package"),
                        value: hardenedPackageName
                    )
                }
                DossierHardeningSummaryFact(
                    label: "Isotope",
                    value: summary.isotopePackageName
                )
            }

            if summary.hasCaveats {
                DossierHardeningSummarySection(title: L10n.string("CAVEATS")) {
                    caveatsContent
                }
            }

            Button {
                openURL(summary.learnMoreURL)
            } label: {
                Text(L10n.string("LEARN MORE"))
                    .font(.system(size: 12, weight: .bold))
                    .foregroundStyle(AVGlassPalette.green)
                    .tracking(0.6)
                    .frame(maxWidth: .infinity)
                    .frame(height: 32)
                    .background(AVGlassPalette.controlFill, in: RoundedRectangle(cornerRadius: 5))
                    .overlay(
                        RoundedRectangle(cornerRadius: 5)
                            .stroke(AVGlassPalette.green.opacity(0.48), lineWidth: 1)
                    )
            }
            .buttonStyle(.plain)
            .padding(.top, 3)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(12)
        .background(AVGlassPalette.green.opacity(0.10), in: RoundedRectangle(cornerRadius: 8))
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(AVGlassPalette.green.opacity(0.42), lineWidth: 1)
        )
    }

    @ViewBuilder
    private var caveatsContent: some View {
        switch summary.caveats {
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
                    DossierHardeningSummaryBullet(text: bullets[index])
                }
            }
        case .none:
            EmptyView()
        }
    }
}

private struct DossierHardeningSummarySection<Content: View>: View {
    let title: String
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(AVGlassPalette.green.opacity(0.86))
                .tracking(0.7)
            content
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct DossierHardeningSummaryFact: View {
    let label: String
    let value: String

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Text(label.uppercased())
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(AVGlassPalette.green.opacity(0.86))
                .tracking(0.7)
                .frame(width: 58, alignment: .leading)
            Text(value)
                .font(.system(size: 11, weight: .regular, design: .monospaced))
                .foregroundStyle(AVGlassPalette.secondaryText)
                .lineLimit(nil)
                .fixedSize(horizontal: false, vertical: true)
                .frame(maxWidth: .infinity, alignment: .leading)
                .textSelection(.enabled)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct DossierHardeningSummaryBullet: View {
    let text: String

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 7) {
            Text("•")
                .font(.system(size: 12, weight: .bold))
                .foregroundStyle(AVGlassPalette.green.opacity(0.82))
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

        headline = L10n.string("DETECTOR NEEDS REVIEW")
        body = L10n.format(
            "The detector for %@ did not complete cleanly.",
            "isotope:\(securityState.isotopeName)"
        )
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
                DossierSecurityWarningSection(title: L10n.string("DETECTION")) {
                    VStack(alignment: .leading, spacing: 7) {
                        ForEach(warning.reasons.indices, id: \.self) { index in
                            DossierSecurityWarningBullet(text: warning.reasons[index])
                        }
                    }
                }
            }

            if let detectorError = warning.detectorError {
                DossierSecurityWarningSection(title: L10n.string("DETECTOR ERROR")) {
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
                DossierSecurityWarningSection(title: L10n.string("CAVEATS")) {
                    caveatsContent
                }
            }

            Button {
                openURL(warning.learnMoreURL)
            } label: {
                Text(L10n.string("LEARN MORE"))
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
            .monospacedDigit()
            .foregroundStyle(foreground)
            .padding(.horizontal, SidebarCountMetrics.pillHorizontalPadding)
            .frame(height: 20)
            .background(background, in: Capsule())
            .overlay(Capsule().stroke(border, lineWidth: borderWidth))
            .shadow(color: shadow, radius: 5)
            // The capsule extends into the row padding so its digit keeps the same trailing edge as plain counts.
            .padding(.trailing, -SidebarCountMetrics.pillHorizontalPadding)
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
    let title: PackageDisplayTitle
    let description: String
    let version: String
    let inlineBadges: [MainWindowPackageBadge]
    let severityLevel: Int?
    let badges: [MainWindowPackageBadge]
    let selected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(alignment: .top, spacing: 10) {
                VStack(alignment: .leading, spacing: 4) {
                    HStack(alignment: .firstTextBaseline, spacing: 6) {
                        PackageRowTitleText(title: title)
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

                if let severityLevel {
                    SeverityBars(level: severityLevel)
                        .padding(.top, 1)
                }

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

private struct PackageRowTitleText: View {
    let title: PackageDisplayTitle

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 0) {
            Text(title.name)
                .foregroundStyle(AVGlassPalette.primaryText)
                .lineLimit(1)
                .truncationMode(.tail)
            if let versionSuffix = title.versionSuffix {
                Text(versionSuffix)
                    .foregroundStyle(AVGlassPalette.quietText.opacity(0.68))
                    .lineLimit(1)
                    .fixedSize(horizontal: true, vertical: false)
            }
        }
        .font(.system(size: 14, weight: .semibold))
        .lineLimit(1)
        .truncationMode(.tail)
        .layoutPriority(1)
    }
}

private struct SeverityBars: View {
    let level: Int

    private var clampedLevel: Int {
        min(max(level, 1), 3)
    }

    var body: some View {
        HStack(alignment: .bottom, spacing: 2) {
            ForEach(0..<3, id: \.self) { index in
                RoundedRectangle(cornerRadius: 1, style: .continuous)
                    .fill(barColor(for: index))
                    .frame(width: 3, height: barHeight(for: index))
            }
        }
        .frame(width: 13, height: 12, alignment: .bottom)
        .accessibilityLabel("Severity \(clampedLevel) of 3")
    }

    private func barHeight(for index: Int) -> CGFloat {
        [4, 7, 10][index]
    }

    private func barColor(for index: Int) -> Color {
        index < clampedLevel
            ? AVGlassPalette.primaryText.opacity(0.76)
            : AVGlassPalette.quietText.opacity(0.18)
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
            return L10n.string("New")
        case .vulnerable:
            return L10n.string("Vulnerable")
        case .hardened:
            return L10n.string("Hardened")
        case .automicVault:
            return "Automic Vault"
        case .immutable:
            return L10n.string("Immutable")
        case .outdated:
            return L10n.string("OUTDATED")
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
        case .automicVault:
            return AVGlassPalette.green
        case .immutable:
            return AVGlassPalette.cyan
        }
    }
}

private struct PackageBadgePill: View {
    let badge: MainWindowPackageBadge

    var body: some View {
        HStack(spacing: 4) {
            if let systemImage {
                Image(systemName: systemImage)
                    .font(.system(size: 10, weight: .semibold))
            }
            Text(title)
                .lineLimit(1)
        }
        .font(.system(size: 11, weight: .semibold))
        .foregroundStyle(foreground)
        .padding(.horizontal, 8)
        .frame(height: 22)
        .fixedSize(horizontal: true, vertical: false)
        .background(background, in: Capsule())
        .overlay(Capsule().stroke(border, lineWidth: 1))
    }

    private var title: String {
        switch badge {
        case .new:
            return L10n.string("New")
        case .vulnerable:
            return L10n.string("Vulnerable")
        case .hardened:
            return L10n.string("Hardened")
        case .automicVault:
            return "Automic Vault"
        case .immutable:
            return L10n.string("Immutable")
        case .outdated:
            return L10n.string("Outdated")
        }
    }

    private var systemImage: String? {
        switch badge {
        case .automicVault:
            return "shield.fill"
        case .new, .vulnerable, .hardened, .immutable, .outdated:
            return nil
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
        case .automicVault:
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
        case .automicVault:
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
        case .automicVault:
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
            return L10n.string("New")
        case .vulnerable:
            return L10n.string("Vulnerable")
        case .hardened:
            return L10n.string("Hardened")
        case .automicVault:
            return "Automic Vault"
        case .immutable:
            return L10n.string("Immutable")
        case .outdated:
            return L10n.string("Outdated")
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
        case .automicVault:
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
        case .automicVault:
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
        case .automicVault:
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
        case .automicVault:
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
            .help(L10n.string("Open externally"))
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
