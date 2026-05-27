import SwiftUI

struct MainWindowView: View {
    @ObservedObject var model: MainWindowModel
    @State private var linkTab: MainWindowLinkTab = .homepage
    @Namespace private var glassNamespace

    var body: some View {
        ZStack {
            background
            VStack(spacing: 0) {
                topBar
                hairline
                mainContent
            }
            .ignoresSafeArea(.container, edges: .top)
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

    private var topBar: some View {
        GlassEffectContainer(spacing: 10) {
            HStack(spacing: 12) {
                Spacer(minLength: 320)

                searchField
                    .frame(width: 318)
                    .glassEffectID("search", in: glassNamespace)

                Text("\(model.installedCount) installed")
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(AVGlassPalette.secondaryText)

                Button {
                    model.reloadPackages()
                } label: {
                    Image(systemName: "arrow.clockwise")
                        .font(.system(size: 13, weight: .semibold))
                }
                .buttonStyle(.glass)
                .controlSize(.small)
                .tint(.clear)
                .disabled(model.isReloading)
                .help("Refresh packages")
                .glassEffectID("refresh", in: glassNamespace)

                topModeControl
                    .glassEffectID("mode", in: glassNamespace)
            }
        }
        .frame(height: 52)
        .padding(.leading, 108)
        .padding(.trailing, 14)
        .background {
            LiquidGlassSurface(
                material: .ultraThinMaterial,
                tint: AVGlassPalette.topBarTint
            )
        }
    }

    private var searchField: some View {
        HStack(spacing: 8) {
            Image(systemName: "magnifyingglass")
                .font(.system(size: 14, weight: .medium))
                .foregroundStyle(AVGlassPalette.quietText)
            TextField("Search packages...", text: $model.searchText)
                .textFieldStyle(.plain)
                .font(.system(size: 13, weight: .regular))
                .foregroundStyle(AVGlassPalette.primaryText)
            Text("Command-K")
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(AVGlassPalette.quietText)
                .padding(.horizontal, 6)
                .padding(.vertical, 2)
                .background(AVGlassPalette.controlFill, in: Capsule())
        }
        .padding(.horizontal, 10)
        .frame(height: 30)
        .glassEffect(.regular, in: RoundedRectangle(cornerRadius: 10, style: .continuous))
    }

    private var topModeControl: some View {
        HStack(spacing: 4) {
            Text("Dossier")
                .foregroundStyle(AVGlassPalette.secondaryText)
            Text("Links")
                .foregroundStyle(AVGlassPalette.primaryText)
            Image(systemName: "chevron.down")
                .font(.system(size: 10, weight: .bold))
                .foregroundStyle(AVGlassPalette.secondaryText)
        }
        .font(.system(size: 12, weight: .semibold))
        .padding(.horizontal, 7)
        .frame(height: 30)
        .glassEffect(.regular, in: RoundedRectangle(cornerRadius: 10, style: .continuous))
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
                .padding(.top, 26)
            ForEach(MainWindowSection.librarySections) { section in
                sidebarRow(section)
            }

            sidebarHeader("CATEGORIES")
                .padding(.top, 22)
            ForEach(MainWindowSection.categorySections) { section in
                sidebarRow(section)
            }

            Spacer(minLength: 24)

            ForEach(MainWindowSection.utilitySections) { section in
                sidebarRow(section)
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
                    .font(.system(size: 14, weight: .medium))
                    .frame(width: 17)
                Text(section.title)
                    .font(.system(size: 14, weight: .semibold))
                    .lineLimit(1)
                    .layoutPriority(1)
                Spacer(minLength: 6)
                if let count = model.count(for: section) {
                    CountPill(count: count)
                        .fixedSize()
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
                        .fill(AVGlassPalette.selectedFill)
                        .overlay(alignment: .leading) {
                            Capsule()
                                .fill(AVGlassPalette.primaryText.opacity(0.72))
                                .frame(width: 3, height: 21)
                                .offset(x: -6)
                        }
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
                if model.isReloading || model.isSearching {
                    ProgressView()
                        .controlSize(.small)
                }
            }
            .font(.system(size: 13, weight: .bold))
            .foregroundStyle(AVGlassPalette.quietText)
            .padding(.horizontal, 18)
            .frame(height: 42)

            hairline

            ScrollView {
                LazyVStack(spacing: 0) {
                    ForEach(model.displayedPackages, id: \.selectionID) { package in
                        PackageRow(
                            package: package,
                            title: model.displayName(for: package),
                            description: model.packageDescription(for: package),
                            version: model.versionText(for: package),
                            risk: model.riskLevel(for: package),
                            hardened: model.isHardened(package),
                            selected: model.selectedItemID == package.selectionID
                        ) {
                            model.select(package)
                        }
                        hairline
                    }

                    if model.displayedPackages.isEmpty {
                        EmptyPackageState(
                            title: model.selectedSection.title,
                            message: model.lastErrorMessage
                                ?? model.statusMessage
                                ?? "No packages match the current filter."
                        )
                        .frame(maxWidth: .infinity)
                        .padding(.top, 80)
                    }
                }
            }
            .scrollIndicators(.hidden)
        }
        .background {
            LiquidGlassSurface(
                material: .thinMaterial,
                tint: AVGlassPalette.packageTint
            )
        }
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
                    EmptyPackageState(
                        title: "Dossier",
                        message: "Select a package to inspect its local record."
                    )
                    .padding(.top, 94)
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
            SectionLabel("DOSSIER")
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

            VStack(alignment: .leading, spacing: 8) {
                RiskBanner(risk: model.riskLevel(for: package))
                if model.isHardened(package) {
                    HardenedBanner()
                }
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
            ScrollView {
                linkContent
                    .padding(.horizontal, 28)
                    .padding(.vertical, 26)
            }
            .scrollIndicators(.hidden)
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
                GlassEffectContainer(spacing: 8) {
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
                                        .glassEffect(.regular, in: RoundedRectangle(cornerRadius: 9, style: .continuous))
                                }
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
                ToolbarIcon(systemName: "chevron.left")
                ToolbarIcon(systemName: "chevron.right")
                ToolbarIcon(systemName: "arrow.clockwise")
                Text(model.selectedURL(for: linkTab)?.absoluteString ?? "No link available")
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

    private var linkContent: some View {
        VStack(alignment: .leading, spacing: 22) {
            if let detail = model.selectedDetail,
               let package = model.selectedPackage {
                if detail.isOutdated {
                    Button {
                        model.open(url: model.selectedURL(for: linkTab))
                    } label: {
                        HStack(spacing: 6) {
                            Circle()
                                .fill(AVGlassPalette.secondaryText)
                                .frame(width: 7, height: 7)
                            Text("\(model.versionText(for: package)) is out")
                            Text("Read the changelog")
                                .fontWeight(.bold)
                            Image(systemName: "arrow.right")
                        }
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(AVGlassPalette.secondaryText)
                        .padding(.horizontal, 12)
                        .frame(height: 30)
                    }
                    .buttonStyle(.plain)
                    .glassEffect(.regular, in: Capsule())
                }

                VStack(alignment: .leading, spacing: 14) {
                    Text(model.displayName(for: package))
                        .font(.system(size: 30, weight: .bold))
                        .foregroundStyle(AVGlassPalette.primaryText)
                        .lineLimit(1)

                    Text(detail.primaryDescription)
                        .font(.system(size: 21, weight: .bold))
                        .foregroundStyle(AVGlassPalette.primaryText)
                        .lineLimit(3)
                        .fixedSize(horizontal: false, vertical: true)

                    Text(model.packageDescription(for: package))
                        .font(.system(size: 14, weight: .regular))
                        .foregroundStyle(AVGlassPalette.quietText)
                        .lineSpacing(4)
                        .lineLimit(4)

                    Button {
                        model.open(url: model.selectedURL(for: .homepage))
                    } label: {
                        Label("Download \(model.displayName(for: package))", systemImage: "arrow.down.to.line.compact")
                            .font(.system(size: 14, weight: .semibold))
                            .frame(minWidth: 168)
                    }
                    .buttonStyle(.glassProminent)
                    .tint(.clear)
                    .disabled(model.selectedURL(for: .homepage) == nil)

                    Text("macOS 26+ and Automic Vault available")
                        .font(.system(size: 12, weight: .semibold))
                        .foregroundStyle(AVGlassPalette.quietText)
                }

                PackagePreviewCard(packageName: model.displayName(for: package))
                    .padding(.top, 4)
            } else {
                EmptyPackageState(
                    title: "Links",
                    message: "Package links appear here after selecting a package."
                )
                .frame(maxWidth: .infinity)
                .padding(.top, 100)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
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

private struct CountPill: View {
    let count: Int

    var body: some View {
        Text(count.formatted())
            .font(.system(size: 11, weight: .semibold))
            .foregroundStyle(AVGlassPalette.secondaryText)
            .padding(.horizontal, 8)
            .frame(height: 20)
            .background(AVGlassPalette.controlFill, in: Capsule())
    }
}

private struct PackageRow: View {
    let package: PackagePresentation
    let title: String
    let description: String
    let version: String
    let risk: MainWindowRiskLevel
    let hardened: Bool
    let selected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(alignment: .top, spacing: 10) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(title)
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(AVGlassPalette.primaryText)
                        .lineLimit(1)
                    Text(description)
                        .font(.system(size: 12, weight: .regular))
                        .foregroundStyle(AVGlassPalette.quietText)
                        .lineLimit(1)
                    Text(version)
                        .font(.system(size: 12, weight: .regular))
                        .foregroundStyle(AVGlassPalette.quietText.opacity(0.74))
                        .lineLimit(1)
                }

                Spacer(minLength: 8)

                VStack(alignment: .trailing, spacing: 6) {
                    RiskPill(risk: risk)
                    if hardened {
                        HardenedPill()
                    }
                }
            }
            .padding(.horizontal, 18)
            .padding(.vertical, 11)
            .frame(minHeight: 68)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(selected ? AVGlassPalette.selectedFill : .clear)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}

private struct RiskPill: View {
    let risk: MainWindowRiskLevel

    var body: some View {
        Text(risk.title)
            .font(.system(size: 11, weight: .semibold))
            .foregroundStyle(foreground)
            .padding(.horizontal, 8)
            .frame(height: 22)
            .background(background, in: Capsule())
            .overlay(Capsule().stroke(foreground.opacity(0.24), lineWidth: 1))
    }

    private var foreground: Color {
        switch risk {
        case .low:
            return AVGlassPalette.green
        case .medium:
            return AVGlassPalette.orange
        case .high:
            return AVGlassPalette.red
        }
    }

    private var background: Color {
        foreground.opacity(0.14)
    }
}

private struct HardenedPill: View {
    var body: some View {
        Text("Hardened")
            .font(.system(size: 11, weight: .semibold))
            .foregroundStyle(AVGlassPalette.blue)
            .padding(.horizontal, 8)
            .frame(height: 22)
            .background(AVGlassPalette.blue.opacity(0.14), in: Capsule())
            .overlay(Capsule().stroke(AVGlassPalette.blue.opacity(0.18), lineWidth: 1))
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

private struct RiskBanner: View {
    let risk: MainWindowRiskLevel

    var body: some View {
        Label(
            risk == .high ? "High Risk" : "\(risk.title) Risk",
            systemImage: risk == .high ? "shield.lefthalf.filled.badge.exclamationmark" : "shield"
        )
        .font(.system(size: 14, weight: .bold))
        .foregroundStyle(color)
        .padding(.horizontal, 10)
        .frame(height: 32)
        .background(color.opacity(0.14), in: RoundedRectangle(cornerRadius: 10))
        .overlay(RoundedRectangle(cornerRadius: 10).stroke(color.opacity(0.22), lineWidth: 1))
    }

    private var color: Color {
        switch risk {
        case .high:
            return AVGlassPalette.red
        case .medium:
            return AVGlassPalette.orange
        case .low:
            return AVGlassPalette.green
        }
    }
}

private struct HardenedBanner: View {
    var body: some View {
        Label("Hardened by Automic Vault", systemImage: "shield")
            .font(.system(size: 13, weight: .semibold))
            .foregroundStyle(AVGlassPalette.blue)
            .padding(.horizontal, 10)
            .frame(height: 32)
            .background(AVGlassPalette.blue.opacity(0.12), in: RoundedRectangle(cornerRadius: 10))
            .overlay(RoundedRectangle(cornerRadius: 10).stroke(AVGlassPalette.blue.opacity(0.18), lineWidth: 1))
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

private struct ToolbarIcon: View {
    let systemName: String

    var body: some View {
        Image(systemName: systemName)
            .font(.system(size: 13, weight: .bold))
            .foregroundStyle(AVGlassPalette.quietText)
            .frame(width: 26, height: 26)
    }
}

private struct PackagePreviewCard: View {
    let packageName: String

    var body: some View {
        VStack(alignment: .leading, spacing: 11) {
            HStack {
                Text("Hidden")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(AVGlassPalette.quietText)
                Spacer()
                Image(systemName: "xmark")
                    .foregroundStyle(AVGlassPalette.quietText)
            }
            .padding(.horizontal, 10)
            .frame(height: 30)
            .background(AVGlassPalette.selectedFill, in: RoundedRectangle(cornerRadius: 7))

            PreviewRow(color: AVGlassPalette.cyan, title: packageName)
            PreviewRow(color: AVGlassPalette.purple, title: "Slack")
            PreviewRow(color: AVGlassPalette.green, title: "Spotify")

            HStack {
                Text("Personal")
                    .foregroundStyle(AVGlassPalette.quietText)
                Image(systemName: "chevron.down")
                    .font(.system(size: 10, weight: .bold))
                    .foregroundStyle(AVGlassPalette.quietText)
                Spacer()
                Image(systemName: "xmark")
                    .foregroundStyle(AVGlassPalette.quietText)
            }
            .font(.system(size: 12, weight: .regular))
        }
        .padding(14)
        .frame(maxWidth: 340)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 10))
        .background(AVGlassPalette.previewTint, in: RoundedRectangle(cornerRadius: 10))
        .overlay(
            RoundedRectangle(cornerRadius: 10)
                .stroke(Color.white.opacity(0.06), lineWidth: 1)
        )
    }
}

private struct PreviewRow: View {
    let color: Color
    let title: String

    var body: some View {
        HStack(spacing: 10) {
            Circle()
                .fill(color)
                .frame(width: 8, height: 8)
            Text(title)
                .font(.system(size: 12, weight: .regular))
                .foregroundStyle(AVGlassPalette.secondaryText)
            Spacer()
            Image(systemName: "eye.slash")
                .foregroundStyle(AVGlassPalette.quietText.opacity(0.76))
            Image(systemName: "link")
                .foregroundStyle(AVGlassPalette.quietText.opacity(0.76))
        }
    }
}

private struct EmptyPackageState: View {
    let title: String
    let message: String

    var body: some View {
        VStack(spacing: 8) {
            Text(title)
                .font(.system(size: 15, weight: .semibold))
                .foregroundStyle(AVGlassPalette.secondaryText)
            Text(message)
                .font(.system(size: 12, weight: .regular))
                .foregroundStyle(AVGlassPalette.quietText)
                .multilineTextAlignment(.center)
                .lineLimit(3)
        }
        .padding(.horizontal, 28)
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
    static let topBarTint = Color.black.opacity(0.16)
    static let sidebarTint = Color(red: 0.025, green: 0.050, blue: 0.075).opacity(0.28)
    static let packageTint = Color.black.opacity(0.20)
    static let dossierTint = Color.black.opacity(0.24)
    static let linksTint = Color.black.opacity(0.18)
    static let previewTint = Color.black.opacity(0.34)
    static let controlFill = Color.white.opacity(0.075)
    static let selectedFill = Color.white.opacity(0.095)
    static let hairline = Color.white.opacity(0.10)
    static let primaryText = Color.white.opacity(0.92)
    static let secondaryText = Color.white.opacity(0.64)
    static let quietText = Color.white.opacity(0.36)
    static let green = Color(red: 0.10, green: 0.86, blue: 0.58)
    static let orange = Color(red: 0.95, green: 0.58, blue: 0.25)
    static let red = Color(red: 1.00, green: 0.45, blue: 0.45)
    static let blue = Color(red: 0.55, green: 0.67, blue: 0.82)
    static let cyan = Color(red: 0.10, green: 0.52, blue: 1.00)
    static let purple = Color(red: 0.44, green: 0.10, blue: 0.48)
}
