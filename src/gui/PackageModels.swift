import AppKit
import Foundation

private let packageSearchOrderPrefixes = [
    "brew:",
    "cask:",
    "gone:",
    "isotope:",
    "sys:",
    "av:",
    "npm:",
    "pip:"
]

private let macOSSystemDetectorPackageNames: Set<String> = [
    "curl",
    "git",
    "openssh",
    "openssl@3",
    "perl",
    "rsync",
    "ruby",
]

extension String {
    func strippingPrefix(_ prefix: String) -> String? {
        guard hasPrefix(prefix) else {
            return nil
        }
        return String(dropFirst(prefix.count))
    }

    var packageSearchOrderName: String {
        for prefix in packageSearchOrderPrefixes where hasPrefix(prefix) {
            return String(dropFirst(prefix.count)).packageScopeOrderName
        }
        return packageScopeOrderName
    }

    private var packageScopeOrderName: String {
        guard hasPrefix("@"),
              let separator = firstIndex(of: "/") else {
            return self
        }
        return String(self[index(after: separator)...])
    }

    var isLocalDetectorDisplayPackageName: Bool {
        hasPrefix("gone:") || hasPrefix("sys:")
    }
}

enum CategoryPackageSortOrder: String, CaseIterable, Identifiable, Sendable {
    case rank
    case alphabetical

    var id: String { rawValue }

    var title: String {
        switch self {
        case .rank:
            return L10n.string("Popularity")
        case .alphabetical:
            return L10n.string("A-Z")
        }
    }

    var protocolValue: String {
        switch self {
        case .rank:
            return "rank"
        case .alphabetical:
            return "az"
        }
    }
}

struct PackageRecord: Decodable, Equatable {
    let name: String
    let source: PackageSource?
    let version: String
    let description: String?
    let homepage: String?
    let repository: String?
    let upstreamDocs: String?
    let docs: [String]
    let category: String?
    let latestVersion: String?
    let securityState: PackageSecurityState?
    let installRoot: String?
    let installPackageNames: [String]?
    let installedVersions: [String]

    enum CodingKeys: String, CodingKey {
        case name
        case source
        case version
        case description
        case homepage
        case repository
        case repo
        case upstreamDocs
        case docs
        case category
        case latestVersion
        case securityState
        case installRoot
        case installPackageNames
        case installedVersions
    }

    init(
        name: String,
        source: PackageSource?,
        version: String,
        description: String?,
        homepage: String? = nil,
        repository: String? = nil,
        upstreamDocs: String? = nil,
        docs: [String] = [],
        category: String? = nil,
        latestVersion: String? = nil,
        securityState: PackageSecurityState?,
        installRoot: String? = nil,
        installPackageNames: [String]? = nil,
        installedVersions: [String] = []
    ) {
        self.name = name
        self.source = source
        self.version = version
        self.description = description
        self.homepage = homepage
        self.repository = repository
        self.upstreamDocs = upstreamDocs
        self.docs = docs
        self.category = category
        self.latestVersion = latestVersion
        self.securityState = securityState
        self.installRoot = installRoot
        self.installPackageNames = installPackageNames
        self.installedVersions = installedVersions
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        name = try container.decode(String.self, forKey: .name)
        source = try container.decodeIfPresent(PackageSource.self, forKey: .source)
        version = try container.decode(String.self, forKey: .version)
        description = try container.decodeIfPresent(String.self, forKey: .description)
        homepage = try container.decodeIfPresent(String.self, forKey: .homepage)
        repository =
            try container.decodeIfPresent(String.self, forKey: .repository)
            ?? container.decodeIfPresent(String.self, forKey: .repo)
        upstreamDocs = try container.decodeIfPresent(String.self, forKey: .upstreamDocs)
        docs = try container.decodeIfPresent([String].self, forKey: .docs) ?? []
        category = try container.decodeIfPresent(String.self, forKey: .category)
        latestVersion = try container.decodeIfPresent(String.self, forKey: .latestVersion)
        securityState = try container.decodeIfPresent(PackageSecurityState.self, forKey: .securityState)
        installRoot = try container.decodeIfPresent(String.self, forKey: .installRoot)
        installPackageNames = try container.decodeIfPresent(
            [String].self,
            forKey: .installPackageNames
        )
        installedVersions = try container.decodeIfPresent(
            [String].self,
            forKey: .installedVersions
        ) ?? []
    }

    var isOutdated: Bool {
        guard let latestVersion, !latestVersion.isEmpty else {
            return false
        }
        return version != latestVersion
    }

    var hasMainWindowSecurityAlert: Bool {
        PackagePresentation(
            item: .installed(self),
            detail: fallbackDetail,
            freshness: 0
        ).hasMainWindowSecurityAlert()
    }

    func applying(outdated: OutdatedPackageRecord) -> PackageRecord {
        PackageRecord(
            name: name,
            source: source,
            version: version,
            description: description,
            homepage: homepage,
            repository: repository,
            upstreamDocs: upstreamDocs,
            docs: docs,
            category: category,
            latestVersion: outdated.latestVersion,
            securityState: securityState,
            installRoot: installRoot,
            installPackageNames: installPackageNames,
            installedVersions: installedVersions
        )
    }

    func clearingSecurityState() -> PackageRecord {
        PackageRecord(
            name: name,
            source: source,
            version: version,
            description: description,
            homepage: homepage,
            repository: repository,
            upstreamDocs: upstreamDocs,
            docs: docs,
            category: category,
            latestVersion: latestVersion,
            securityState: nil,
            installRoot: installRoot,
            installPackageNames: installPackageNames,
            installedVersions: installedVersions
        )
    }

    var fallbackDetail: PackageDetail {
        PackageDetail(
            packageName: name,
            qualifiedName: name,
            installRoot: installRoot ?? "/opt",
            installed: true,
            source: source,
            sourceError: nil,
            aliases: [],
            aliasesError: nil,
            installedVersion: version,
            latestVersion: latestVersion,
            latestVersionError: nil,
            executablePaths: [],
            executablePathsError: nil,
            popularity: nil,
            lastUpdatedAt: nil,
            homebrewInfo: HomebrewPackageInfo(
                formula: name,
                description: description,
                homepage: homepage,
                repository: repository,
                upstreamDocs: upstreamDocs,
                docs: docs,
                license: nil,
                dependencies: []
            ),
            homebrewInfoError: nil,
            npmHomepage: nil,
            npmPackageInfoError: nil,
            securityState: securityState,
            installPackageNames: installPackageNames
        )
    }
}

struct PackageSecurityState: Decodable, Equatable {
    let isotopeName: String
    let installIsInsecure: Bool
    let remediationAvailable: Bool
    let reasons: [String]
    let error: String?

    enum CodingKeys: String, CodingKey {
        case isotopeName
        case installIsInsecure
        case remediationAvailable
        case reasons
        case error
    }

    init(
        isotopeName: String,
        installIsInsecure: Bool,
        remediationAvailable: Bool = true,
        reasons: [String],
        error: String?
    ) {
        self.isotopeName = isotopeName
        self.installIsInsecure = installIsInsecure
        self.remediationAvailable = remediationAvailable
        self.reasons = reasons
        self.error = error
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        isotopeName = try container.decode(String.self, forKey: .isotopeName)
        installIsInsecure = try container.decode(Bool.self, forKey: .installIsInsecure)
        remediationAvailable = try container.decodeIfPresent(
            Bool.self,
            forKey: .remediationAvailable
        ) ?? true
        reasons = try container.decodeIfPresent([String].self, forKey: .reasons) ?? []
        error = try container.decodeIfPresent(String.self, forKey: .error)
    }

    var needsMainWindowSecurityAlert: Bool {
        installIsInsecure
            || error?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
    }
}

struct OutdatedPackageRecord: Codable, Equatable {
    let name: String
    let currentVersion: String
    let latestVersion: String
}

struct PackageDetail: Decodable, Equatable {
    let packageName: String
    let qualifiedName: String
    let installRoot: String
    let installed: Bool
    let source: PackageSource?
    let sourceError: String?
    let aliases: [String]
    let aliasesError: String?
    let installedVersion: String?
    let latestVersion: String?
    let latestVersionError: String?
    let executablePaths: [String]
    let executablePathsError: String?
    let popularity: PackagePopularity?
    let lastUpdatedAt: String?
    let homebrewInfo: HomebrewPackageInfo?
    let homebrewInfoError: String?
    let npmHomepage: String?
    let npmPackageInfoError: String?
    let securityState: PackageSecurityState?
    let installPackageNames: [String]?
    let versionOptions: [PackageVersionOption]

    enum CodingKeys: String, CodingKey {
        case packageName
        case qualifiedName
        case installRoot
        case installed
        case source
        case sourceError
        case aliases
        case aliasesError
        case installedVersion
        case latestVersion
        case latestVersionError
        case executablePaths
        case executablePathsError
        case popularity
        case lastUpdatedAt
        case homebrewInfo
        case homebrewInfoError
        case npmHomepage
        case npmPackageInfoError
        case securityState
        case installPackageNames
        case versionOptions
    }

    init(
        packageName: String,
        qualifiedName: String,
        installRoot: String,
        installed: Bool,
        source: PackageSource?,
        sourceError: String?,
        aliases: [String],
        aliasesError: String?,
        installedVersion: String?,
        latestVersion: String?,
        latestVersionError: String?,
        executablePaths: [String],
        executablePathsError: String?,
        popularity: PackagePopularity?,
        lastUpdatedAt: String?,
        homebrewInfo: HomebrewPackageInfo?,
        homebrewInfoError: String?,
        npmHomepage: String?,
        npmPackageInfoError: String?,
        securityState: PackageSecurityState?,
        installPackageNames: [String]?,
        versionOptions: [PackageVersionOption] = []
    ) {
        self.packageName = packageName
        self.qualifiedName = qualifiedName
        self.installRoot = installRoot
        self.installed = installed
        self.source = source
        self.sourceError = sourceError
        self.aliases = aliases
        self.aliasesError = aliasesError
        self.installedVersion = installedVersion
        self.latestVersion = latestVersion
        self.latestVersionError = latestVersionError
        self.executablePaths = executablePaths
        self.executablePathsError = executablePathsError
        self.popularity = popularity
        self.lastUpdatedAt = lastUpdatedAt
        self.homebrewInfo = homebrewInfo
        self.homebrewInfoError = homebrewInfoError
        self.npmHomepage = npmHomepage
        self.npmPackageInfoError = npmPackageInfoError
        self.securityState = securityState
        self.installPackageNames = installPackageNames
        self.versionOptions = versionOptions
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        packageName = try container.decode(String.self, forKey: .packageName)
        qualifiedName = try container.decode(String.self, forKey: .qualifiedName)
        installRoot = try container.decode(String.self, forKey: .installRoot)
        installed = try container.decode(Bool.self, forKey: .installed)
        source = try container.decodeIfPresent(PackageSource.self, forKey: .source)
        sourceError = try container.decodeIfPresent(String.self, forKey: .sourceError)
        aliases = try container.decode([String].self, forKey: .aliases)
        aliasesError = try container.decodeIfPresent(String.self, forKey: .aliasesError)
        installedVersion = try container.decodeIfPresent(String.self, forKey: .installedVersion)
        latestVersion = try container.decodeIfPresent(String.self, forKey: .latestVersion)
        latestVersionError = try container.decodeIfPresent(String.self, forKey: .latestVersionError)
        executablePaths = try container.decode([String].self, forKey: .executablePaths)
        executablePathsError = try container.decodeIfPresent(
            String.self,
            forKey: .executablePathsError
        )
        popularity = try container.decodeIfPresent(PackagePopularity.self, forKey: .popularity)
        lastUpdatedAt = try container.decodeIfPresent(String.self, forKey: .lastUpdatedAt)
        homebrewInfo = try container.decodeIfPresent(
            HomebrewPackageInfo.self,
            forKey: .homebrewInfo
        )
        homebrewInfoError = try container.decodeIfPresent(String.self, forKey: .homebrewInfoError)
        npmHomepage = try container.decodeIfPresent(String.self, forKey: .npmHomepage)
        npmPackageInfoError = try container.decodeIfPresent(
            String.self,
            forKey: .npmPackageInfoError
        )
        securityState = try container.decodeIfPresent(
            PackageSecurityState.self,
            forKey: .securityState
        )
        installPackageNames = try container.decodeIfPresent(
            [String].self,
            forKey: .installPackageNames
        )
        versionOptions = try container.decodeIfPresent(
            [PackageVersionOption].self,
            forKey: .versionOptions
        ) ?? []
    }

    var primaryDescription: String {
        if let description = homebrewInfo?.description, !description.isEmpty {
            return description
        }
        if installed {
            return L10n.string("Installed component record available in the local vault.")
        }
        return L10n.string(
            "Component metadata is available, but the local vault has not initialized it."
        )
    }

    var metadataLine: String {
        let version = installedVersion ?? latestVersion ?? L10n.string("unversioned")
        return [version, source?.displayLabel, installed ? L10n.string("installed") : L10n.string("uninstalled")]
            .compactMap { $0 }
            .joined(separator: "  ·  ")
    }

    var hasConfiguredHomepage: Bool {
        guard let rawHomepage else {
            return false
        }
        return !rawHomepage.isEmpty
    }

    var homepageURL: URL? {
        let homepage = Self.externalURL(from: rawHomepage)
            .flatMap { $0.isHomebrewPackageManagerPage ? nil : $0 }

        if isOutdated {
            if let latestReleaseURL = homepage?.githubLatestReleaseURL {
                return latestReleaseURL
            }
            if let latestReleaseURL = Self.githubRepositoryURL(
                from: homebrewInfo?.repository
            )?.githubLatestReleaseURL {
                return latestReleaseURL
            }
        }

        return homepage?.githubRepositoryReadmeURL
    }

    var repositoryURL: URL? {
        Self.repositoryURL(from: homebrewInfo?.repository)
    }

    var upstreamDocsURL: URL? {
        let candidates = [homebrewInfo?.upstreamDocs, homebrewInfo?.docs.first, npmHomepage]
        return candidates.lazy.compactMap(Self.externalURL).first
    }

    var isOutdated: Bool {
        guard let installedVersion, let latestVersion else {
            return false
        }
        guard !installedVersion.isEmpty, !latestVersion.isEmpty else {
            return false
        }
        return installedVersion != latestVersion
    }

    var dependencies: [String] {
        homebrewInfo?.dependencies ?? []
    }

    var installCommand: String {
        if isAutomicVaultCLT {
            return "Install bundled av"
        }
        if isXcodeCLT {
            return "xcode-select --install"
        }
        return "av install \(helperPackageNames.joined(separator: " "))"
    }

    var helperPackageNames: [String] {
        if let installPackageNames, installPackageNames.isEmpty == false {
            return installPackageNames
        }
        return [helperPackageName]
    }

    var helperPackageName: String {
        if isAutomicVaultCLT {
            return packageName
        }
        switch source {
        case .formula(let rootFormula):
            return installed ? "brew:\(rootFormula)" : rootFormula
        case .cask(let caskName):
            return "cask:\(caskName)"
        case .isotope(let isotopeName):
            return "isotope:\(isotopeName)"
        case .vendor(let vendorName):
            return vendorName
        case .npm(let packageName):
            return "npm:\(packageName)"
        case .pip(let packageName):
            return "pip:\(packageName)"
        case .none:
            return packageName
        }
    }

    func selecting(versionOption option: PackageVersionOption) -> PackageDetail {
        PackageDetail(
            packageName: option.packageName,
            qualifiedName: "brew:\(option.rootFormula)",
            installRoot: option.installRoot,
            installed: option.installed,
            source: .formula(rootFormula: option.rootFormula),
            sourceError: sourceError,
            aliases: aliases,
            aliasesError: aliasesError,
            installedVersion: option.installed ? option.version : nil,
            latestVersion: option.version ?? latestVersion,
            latestVersionError: latestVersionError,
            executablePaths: executablePaths,
            executablePathsError: executablePathsError,
            popularity: popularity,
            lastUpdatedAt: lastUpdatedAt,
            homebrewInfo: homebrewInfo,
            homebrewInfoError: homebrewInfoError,
            npmHomepage: npmHomepage,
            npmPackageInfoError: npmPackageInfoError,
            securityState: securityState,
            installPackageNames: [option.installPackageName],
            versionOptions: versionOptions
        )
    }

    func applying(outdated: OutdatedPackageRecord?) -> PackageDetail {
        guard installed else {
            return self
        }

        let resolvedLatestVersion: String?
        let resolvedLatestVersionError: String?

        if let outdated {
            resolvedLatestVersion = outdated.latestVersion
            resolvedLatestVersionError = nil
        } else {
            resolvedLatestVersion = installedVersion
            resolvedLatestVersionError = nil
        }

        return PackageDetail(
            packageName: packageName,
            qualifiedName: qualifiedName,
            installRoot: installRoot,
            installed: installed,
            source: source,
            sourceError: sourceError,
            aliases: aliases,
            aliasesError: aliasesError,
            installedVersion: installedVersion,
            latestVersion: resolvedLatestVersion,
            latestVersionError: resolvedLatestVersionError,
            executablePaths: executablePaths,
            executablePathsError: executablePathsError,
            popularity: popularity,
            lastUpdatedAt: lastUpdatedAt,
            homebrewInfo: homebrewInfo,
            homebrewInfoError: homebrewInfoError,
            npmHomepage: npmHomepage,
            npmPackageInfoError: npmPackageInfoError,
            securityState: securityState,
            installPackageNames: installPackageNames,
            versionOptions: versionOptions
        )
    }

    func clearingSecurityState() -> PackageDetail {
        PackageDetail(
            packageName: packageName,
            qualifiedName: qualifiedName,
            installRoot: installRoot,
            installed: installed,
            source: source,
            sourceError: sourceError,
            aliases: aliases,
            aliasesError: aliasesError,
            installedVersion: installedVersion,
            latestVersion: latestVersion,
            latestVersionError: latestVersionError,
            executablePaths: executablePaths,
            executablePathsError: executablePathsError,
            popularity: popularity,
            lastUpdatedAt: lastUpdatedAt,
            homebrewInfo: homebrewInfo,
            homebrewInfoError: homebrewInfoError,
            npmHomepage: npmHomepage,
            npmPackageInfoError: npmPackageInfoError,
            securityState: nil,
            installPackageNames: installPackageNames,
            versionOptions: versionOptions
        )
    }

    func withPackageIdentity(
        packageName displayPackageName: String,
        installPackageNames displayInstallPackageNames: [String]?
    ) -> PackageDetail {
        PackageDetail(
            packageName: displayPackageName,
            qualifiedName: displayPackageName,
            installRoot: installRoot,
            installed: installed,
            source: source,
            sourceError: sourceError,
            aliases: aliases,
            aliasesError: aliasesError,
            installedVersion: installedVersion,
            latestVersion: latestVersion,
            latestVersionError: latestVersionError,
            executablePaths: executablePaths,
            executablePathsError: executablePathsError,
            popularity: popularity,
            lastUpdatedAt: lastUpdatedAt,
            homebrewInfo: homebrewInfo,
            homebrewInfoError: homebrewInfoError,
            npmHomepage: npmHomepage,
            npmPackageInfoError: npmPackageInfoError,
            securityState: securityState,
            installPackageNames: displayInstallPackageNames,
            versionOptions: versionOptions
        )
    }

    func preservingLocalSecurityContext(from fallback: PackageDetail?) -> PackageDetail {
        guard let fallback,
              fallback.securityStateNeedsReview,
              !securityStateNeedsReview else {
            return self
        }

        return PackageDetail(
            packageName: fallback.packageName,
            qualifiedName: fallback.qualifiedName,
            installRoot: installRoot,
            installed: installed,
            source: source,
            sourceError: sourceError,
            aliases: aliases,
            aliasesError: aliasesError,
            installedVersion: installedVersion,
            latestVersion: latestVersion,
            latestVersionError: latestVersionError,
            executablePaths: executablePaths,
            executablePathsError: executablePathsError,
            popularity: popularity,
            lastUpdatedAt: lastUpdatedAt,
            homebrewInfo: homebrewInfo,
            homebrewInfoError: homebrewInfoError,
            npmHomepage: npmHomepage,
            npmPackageInfoError: npmPackageInfoError,
            securityState: fallback.securityState,
            installPackageNames: fallback.installPackageNames ?? installPackageNames,
            versionOptions: versionOptions
        )
    }

    private var rawHomepage: String? {
        homebrewInfo?.homepage ?? npmHomepage
    }

    private static func externalURL(from raw: String?) -> URL? {
        let normalized = raw?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        guard !normalized.isEmpty,
              let url = URL(string: normalized),
              let scheme = url.scheme,
              scheme == "http" || scheme == "https" else {
            return nil
        }
        return url
    }

    private static func repositoryURL(from raw: String?) -> URL? {
        if let url = externalURL(from: raw) {
            return url
        }
        return githubRepositoryURL(from: raw)
    }

    private static func githubRepositoryURL(from raw: String?) -> URL? {
        let normalized = raw?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        guard !normalized.isEmpty else {
            return nil
        }

        if let url = externalURL(from: normalized) {
            return url.githubRepositoryURL
        }

        let components = normalized
            .split(separator: "/", omittingEmptySubsequences: true)
            .map(String.init)
        guard components.count == 2,
              components.allSatisfy({ !$0.isEmpty && !$0.contains(where: { $0.isWhitespace }) }) else {
            return nil
        }
        return URL(string: "https://github.com/\(components[0])/\(components[1])")
    }

    var isAutomicVaultCLT: Bool {
        packageName == PackageRecommendation.automicVaultCLTName
    }

    var isXcodeCLT: Bool {
        packageName == PackageRecommendation.xcodeCLTName
    }

    var isSystemDetectorOnlyHazard: Bool {
        packageName.hasPrefix("sys:")
            && securityState?.installIsInsecure == true
            && securityState?.remediationAvailable == false
    }

    var securityNotice: PackageSecurityNotice? {
        let notice = SecurityCatalog.shared.notice(for: self)
        if installed,
           !isIsotopeInstall,
           notice?.source == .isotope,
           !securityStateNeedsReview {
            return nil
        }
        return notice
    }

    private var isIsotopeInstall: Bool {
        if case .isotope = source {
            return true
        }
        return false
    }

    private var securityStateNeedsReview: Bool {
        securityState?.needsMainWindowSecurityAlert == true
    }
}

struct PackageVersionOption: Decodable, Equatable {
    let displayName: String
    let aliasName: String?
    let packageName: String
    let installPackageName: String
    let rootFormula: String
    let version: String?
    let installRoot: String
    let installed: Bool
    let stubActive: Bool
    let isLatest: Bool
    let isRecommended: Bool
    let supportsSideBySideStubs: Bool

    var menuTitle: String {
        var title = displayName
        if isRecommended {
            title += " recommended"
        }
        return title
    }
}

struct PackagePopularity: Decodable, Equatable {
    let installsPer365Days: UInt64
    let rank: UInt32

    var installsPer365DaysText: String {
        NumberFormatter.localizedString(
            from: NSNumber(value: installsPer365Days),
            number: .decimal
        )
    }

    var rankText: String {
        NumberFormatter.localizedString(from: NSNumber(value: rank), number: .decimal)
    }
}

enum PackageSource: Decodable, Equatable {
    case formula(rootFormula: String)
    case cask(caskName: String)
    case isotope(isotopeName: String)
    case vendor(vendorName: String)
    case npm(packageName: String)
    case pip(packageName: String)

    enum CodingKeys: String, CodingKey {
        case kind
        case rootFormula
        case caskName
        case isotopeName
        case vendorName
        case packageName
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(String.self, forKey: .kind) {
        case "formula":
            self = .formula(rootFormula: try container.decode(String.self, forKey: .rootFormula))
        case "cask":
            self = .cask(caskName: try container.decode(String.self, forKey: .caskName))
        case "isotope":
            self = .isotope(isotopeName: try container.decode(String.self, forKey: .isotopeName))
        case "vendor":
            self = .vendor(vendorName: try container.decode(String.self, forKey: .vendorName))
        case "npm":
            self = .npm(packageName: try container.decode(String.self, forKey: .packageName))
        case "pip":
            self = .pip(packageName: try container.decode(String.self, forKey: .packageName))
        default:
            throw DecodingError.dataCorruptedError(
                forKey: .kind,
                in: container,
                debugDescription: "Unsupported source kind"
            )
        }
    }

    var displayLabel: String {
        switch self {
        case .formula:
            return "Homebrew"
        case .cask:
            return "Homebrew Cask"
        case .isotope:
            return "Isotope"
        case .vendor:
            return "Vault"
        case .npm:
            return "NPM"
        case .pip:
            return "PyPI"
        }
    }
}

struct HomebrewPackageInfo: Decodable, Equatable {
    let formula: String
    let description: String?
    let homepage: String?
    let repository: String?
    let upstreamDocs: String?
    let docs: [String]
    let license: String?
    let dependencies: [String]

    enum CodingKeys: String, CodingKey {
        case formula
        case description
        case homepage
        case repository
        case repo
        case upstreamDocs
        case docs
        case license
        case dependencies
    }

    init(
        formula: String,
        description: String?,
        homepage: String?,
        repository: String? = nil,
        upstreamDocs: String? = nil,
        docs: [String] = [],
        license: String?,
        dependencies: [String]
    ) {
        self.formula = formula
        self.description = description
        self.homepage = homepage
        self.repository = repository
        self.upstreamDocs = upstreamDocs
        self.docs = docs
        self.license = license
        self.dependencies = dependencies
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        formula = try container.decode(String.self, forKey: .formula)
        description = try container.decodeIfPresent(String.self, forKey: .description)
        homepage = try container.decodeIfPresent(String.self, forKey: .homepage)
        repository =
            try container.decodeIfPresent(String.self, forKey: .repository)
            ?? container.decodeIfPresent(String.self, forKey: .repo)
        upstreamDocs = try container.decodeIfPresent(String.self, forKey: .upstreamDocs)
        docs = try container.decodeIfPresent([String].self, forKey: .docs) ?? []
        license = try container.decodeIfPresent(String.self, forKey: .license)
        dependencies = try container.decodeIfPresent([String].self, forKey: .dependencies) ?? []
    }
}

struct PackageSearchResult: Decodable, Equatable {
    let name: String
    let source: PackageSource?
    let version: String?
    let description: String?
    let homepage: String?
    let repository: String?
    let upstreamDocs: String?
    let docs: [String]
    let category: String?
    let dependencies: [String]
    let rank: UInt32?
    let lastUpdatedAt: String?
    let securityState: PackageSecurityState?
    let pulseKind: String?

    enum CodingKeys: String, CodingKey {
        case name = "packageName"
        case legacyName = "name"
        case source
        case version = "latestVersion"
        case legacyVersion = "version"
        case description = "summary"
        case legacyDescription = "description"
        case homepage
        case repository
        case repo
        case upstreamDocs
        case docs
        case category
        case dependencies
        case rank
        case lastUpdatedAt
        case securityState
        case pulseKind
    }

    init(
        name: String,
        source: PackageSource?,
        version: String?,
        description: String?,
        homepage: String?,
        repository: String? = nil,
        upstreamDocs: String? = nil,
        docs: [String] = [],
        category: String? = nil,
        dependencies: [String],
        rank: UInt32? = nil,
        lastUpdatedAt: String? = nil,
        securityState: PackageSecurityState?,
        pulseKind: String?
    ) {
        self.name = name
        self.source = source
        self.version = version
        self.description = description
        self.homepage = homepage
        self.repository = repository
        self.upstreamDocs = upstreamDocs
        self.docs = docs
        self.category = category
        self.dependencies = dependencies
        self.rank = rank
        self.lastUpdatedAt = lastUpdatedAt
        self.securityState = securityState
        self.pulseKind = pulseKind
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        if let packageName = try container.decodeIfPresent(String.self, forKey: .name) {
            name = packageName
        } else {
            name = try container.decode(String.self, forKey: .legacyName)
        }
        source = try container.decodeIfPresent(PackageSource.self, forKey: .source)
        version =
            try container.decodeIfPresent(String.self, forKey: .version)
            ?? container.decodeIfPresent(String.self, forKey: .legacyVersion)
        description =
            try container.decodeIfPresent(String.self, forKey: .description)
            ?? container.decodeIfPresent(String.self, forKey: .legacyDescription)
        homepage = try container.decodeIfPresent(String.self, forKey: .homepage)
        repository =
            try container.decodeIfPresent(String.self, forKey: .repository)
            ?? container.decodeIfPresent(String.self, forKey: .repo)
        upstreamDocs = try container.decodeIfPresent(String.self, forKey: .upstreamDocs)
        docs = try container.decodeIfPresent([String].self, forKey: .docs) ?? []
        category = try container.decodeIfPresent(String.self, forKey: .category)
        dependencies = try container.decodeIfPresent([String].self, forKey: .dependencies) ?? []
        rank = try container.decodeIfPresent(UInt32.self, forKey: .rank)
        lastUpdatedAt = try container.decodeIfPresent(String.self, forKey: .lastUpdatedAt)
        securityState = try container.decodeIfPresent(
            PackageSecurityState.self,
            forKey: .securityState
        )
        pulseKind = try container.decodeIfPresent(String.self, forKey: .pulseKind)
    }

    var fallbackDetail: PackageDetail {
        let fallbackSource = source ?? .formula(rootFormula: name)
        return PackageDetail(
            packageName: name,
            qualifiedName: name,
            installRoot: "/opt",
            installed: false,
            source: fallbackSource,
            sourceError: nil,
            aliases: [],
            aliasesError: nil,
            installedVersion: nil,
            latestVersion: version,
            latestVersionError: nil,
            executablePaths: [],
            executablePathsError: nil,
            popularity: nil,
            lastUpdatedAt: lastUpdatedAt,
            homebrewInfo: HomebrewPackageInfo(
                formula: name,
                description: description,
                homepage: homepage,
                repository: repository,
                upstreamDocs: upstreamDocs,
                docs: docs,
                license: nil,
                dependencies: dependencies
            ),
            homebrewInfoError: nil,
            npmHomepage: nil,
            npmPackageInfoError: nil,
            securityState: securityState,
            installPackageNames: nil
        )
    }

    var detailLookupName: String {
        switch source {
        case .formula(let rootFormula):
            return "brew:\(rootFormula)"
        case .cask(let caskName):
            return "cask:\(caskName)"
        case .isotope(let isotopeName):
            return "isotope:\(isotopeName)"
        case .vendor(let vendorName):
            return vendorName
        case .npm(let packageName):
            return "npm:\(packageName)"
        case .pip(let packageName):
            return "pip:\(packageName)"
        case .none:
            return name
        }
    }

    var isNewPulse: Bool {
        let trimmed = pulseKind?.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed?.localizedCaseInsensitiveCompare("new") == .orderedSame
    }

    func detectedLocalHazardPresentation(freshness: CGFloat) -> PackageDetectedLocalHazard? {
        guard securityState?.installIsInsecure == true else {
            return nil
        }

        let lookupName = detailLookupName
        let displayName = Self.localHazardDisplayName(for: lookupName)
        let displayDetail = fallbackDetail.withPackageIdentity(
            packageName: displayName,
            installPackageNames: [lookupName]
        )
        let record = PackageRecord(
            name: displayName,
            source: displayDetail.source,
            version: displayDetail.installedVersion
                ?? displayDetail.latestVersion
                ?? version
                ?? "",
            description: displayDetail.homebrewInfo?.description ?? description,
            homepage: displayDetail.homebrewInfo?.homepage ?? homepage,
            repository: displayDetail.homebrewInfo?.repository ?? repository,
            upstreamDocs: displayDetail.homebrewInfo?.upstreamDocs ?? upstreamDocs,
            docs: displayDetail.homebrewInfo?.docs ?? docs,
            category: category,
            latestVersion: displayDetail.latestVersion,
            securityState: displayDetail.securityState,
            installRoot: displayDetail.installRoot,
            installPackageNames: [lookupName]
        )
        return PackageDetectedLocalHazard(
            lookupName: lookupName,
            detail: displayDetail,
            presentation: PackagePresentation(
                item: .installed(record),
                detail: displayDetail,
                freshness: freshness
            )
        )
    }

    func clearingSecurityState() -> PackageSearchResult {
        PackageSearchResult(
            name: name,
            source: source,
            version: version,
            description: description,
            homepage: homepage,
            repository: repository,
            upstreamDocs: upstreamDocs,
            docs: docs,
            category: category,
            dependencies: dependencies,
            rank: rank,
            lastUpdatedAt: lastUpdatedAt,
            securityState: nil,
            pulseKind: pulseKind
        )
    }

    private static func localHazardDisplayName(for lookupName: String) -> String {
        if let formula = lookupName.strippingPrefix("brew:"), !formula.isEmpty {
            let prefix = macOSSystemDetectorPackageNames.contains(formula) ? "sys:" : "gone:"
            return "\(prefix)\(formula)"
        }
        if let caskName = lookupName.strippingPrefix("cask:"), !caskName.isEmpty {
            return "gone:\(caskName)"
        }
        let packageName = lookupName.packageSearchOrderName
        let prefix = macOSSystemDetectorPackageNames.contains(packageName) ? "sys:" : "gone:"
        return "\(prefix)\(packageName)"
    }
}

struct PackageSearchPage: Decodable, Equatable {
    let packages: [PackageSearchResult]
    let totalCount: Int
    let nextOffset: Int?
    let categoryCounts: [String: Int]

    enum CodingKeys: String, CodingKey {
        case packages
        case totalCount
        case nextOffset
        case categoryCounts
    }

    init(
        packages: [PackageSearchResult],
        totalCount: Int,
        nextOffset: Int?,
        categoryCounts: [String: Int] = [:]
    ) {
        self.packages = packages
        self.totalCount = totalCount
        self.nextOffset = nextOffset
        self.categoryCounts = categoryCounts
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        packages = try container.decode([PackageSearchResult].self, forKey: .packages)
        totalCount = try container.decode(Int.self, forKey: .totalCount)
        nextOffset = try container.decodeIfPresent(Int.self, forKey: .nextOffset)
        categoryCounts = try container.decodeIfPresent(
            [String: Int].self,
            forKey: .categoryCounts
        ) ?? [:]
    }
}

struct PackageDetectedLocalHazard: Equatable {
    let lookupName: String
    let detail: PackageDetail
    let presentation: PackagePresentation
}

private extension URL {
    var isHomebrewPackageManagerPage: Bool {
        guard host?.localizedCaseInsensitiveCompare("formulae.brew.sh") == .orderedSame else {
            return false
        }
        let pathComponents = path
            .split(separator: "/", omittingEmptySubsequences: true)
            .map(String.init)
        return pathComponents.first == "formula" || pathComponents.first == "cask"
    }

    var githubLatestReleaseURL: URL? {
        guard let repositoryURL = githubRepositoryURL,
              var components = URLComponents(url: repositoryURL, resolvingAgainstBaseURL: false) else {
            return nil
        }

        components.path = "\(components.path)/releases/latest"
        return components.url
    }

    var githubRepositoryURL: URL? {
        guard host?.localizedCaseInsensitiveCompare("github.com") == .orderedSame else {
            return nil
        }
        guard fragment == nil else {
            return nil
        }
        let pathComponents = path
            .split(separator: "/", omittingEmptySubsequences: true)
            .map(String.init)
        guard pathComponents.count == 2 else {
            return nil
        }
        var components = URLComponents()
        components.scheme = scheme
        components.host = host
        components.port = port
        components.path = "/\(pathComponents[0])/\(pathComponents[1])"
        return components.url
    }

    var githubRepositoryReadmeURL: URL {
        guard host?.localizedCaseInsensitiveCompare("github.com") == .orderedSame else {
            return self
        }
        guard fragment == nil else {
            return self
        }

        let pathComponents = path
            .split(separator: "/", omittingEmptySubsequences: true)
            .map(String.init)
        guard pathComponents.count == 2 else {
            return self
        }

        guard var components = URLComponents(url: self, resolvingAgainstBaseURL: false) else {
            return self
        }
        components.fragment = "readme"
        return components.url ?? self
    }
}

enum PackageListItem: Equatable {
    case installed(PackageRecord)
    case recommendation(PackageRecommendation)
    case available(PackageSearchResult)
    case command(CommandPaletteItem)

    var isAvailable: Bool {
        switch self {
        case .recommendation, .available, .command:
            return true
        case .installed:
            return false
        }
    }
}

struct PackageRecommendation: Equatable {
    static let automicVaultCLTName = "Automic Vault CLT"
    static let xcodeCLTName = "Xcode CLT"
    static let agenticToolingPackName = "Agentic Toolkit"
    static let agentPackName = "Agent Pack"
    static let unixPlusPlusPackName = "UNIX++ Pack"
    static let agenticToolingPackPackageNames = [
        "ffmpeg-full",
        "imagemagick-full",
        "node",
        "uv",
        "python@3.13",
        "bash",
        "ripgrep",
        "pnpm",
        "yq",
        "gh",
        "gum",
        "fd",
        "bat",
        "tree",
        "git-delta",
        "shellcheck",
        "shfmt",
        "hyperfine",
        "deno",
        "go",
        "pkgconf",
        "cmake",
        "poppler",
        "tesseract",
        "exiftool",
        "pandoc"
    ]
    static let agentPackPackageNames = [
        "codex",
        "claude-code",
        "block-goose-cli",
        "aider",
        "opencode",
        "gemini-cli",
        "qwen-code",
        "ccusage",
        "llm",
        "mods"
    ]
    static let unixPlusPlusPackPackageNames = [
        "bat",
        "bat-extras",
        "eza",
        "fd",
        "ripgrep",
        "sd",
        "dust",
        "duf",
        "procs",
        "bottom",
        "zoxide",
        "fzf",
        "git-delta",
        "hyperfine",
        "tokei",
        "choose",
        "jq",
        "yq",
        "xh",
        "doggo",
        "miller",
        "entr",
        "watchexec"
    ]

    let packageName: String
    let installedVersion: String?
    let latestVersion: String?
    let missingPackageNames: [String]
    let detail: PackageDetail
    let description: String

    var isInstalled: Bool {
        missingPackageNames.isEmpty
    }

    var isOutdated: Bool {
        guard let installedVersion, let latestVersion else {
            return false
        }
        return installedVersion != latestVersion
    }

    func clearingSecurityState() -> PackageRecommendation {
        PackageRecommendation(
            packageName: packageName,
            installedVersion: installedVersion,
            latestVersion: latestVersion,
            missingPackageNames: missingPackageNames,
            detail: detail.clearingSecurityState(),
            description: description
        )
    }

    static func automicVaultCLT(
        installedVersion: String?,
        latestVersion: String,
        missingToolNames: [String]
    ) -> PackageRecommendation {
        let description = missingToolNames.isEmpty
            ? L10n.string("Bundled command line tools are installed but need updating.")
            : L10n.string("Installs the Automic Vault command line tool")
        let detail = PackageDetail(
            packageName: automicVaultCLTName,
            qualifiedName: automicVaultCLTName,
            installRoot: "/usr/local/bin",
            installed: missingToolNames.isEmpty,
            source: nil,
            sourceError: nil,
            aliases: [],
            aliasesError: nil,
            installedVersion: installedVersion,
            latestVersion: latestVersion,
            latestVersionError: nil,
            executablePaths: ["/usr/local/bin/av"],
            executablePathsError: nil,
            popularity: nil,
            lastUpdatedAt: nil,
            homebrewInfo: HomebrewPackageInfo(
                formula: automicVaultCLTName,
                description: description,
                homepage: "https://automicvault.com",
                license: nil,
                dependencies: []
            ),
            homebrewInfoError: nil,
            npmHomepage: nil,
            npmPackageInfoError: nil,
            securityState: nil,
            installPackageNames: nil
        )
        return PackageRecommendation(
            packageName: automicVaultCLTName,
            installedVersion: installedVersion,
            latestVersion: latestVersion,
            missingPackageNames: missingToolNames,
            detail: detail,
            description: description
        )
    }

    static func xcodeCLT() -> PackageRecommendation {
        let description =
            L10n.string(
                "Installs Apple's Command Line Tools for compilers, SDK headers and system build utilities."
            )
        let detail = PackageDetail(
            packageName: xcodeCLTName,
            qualifiedName: xcodeCLTName,
            installRoot: "/Library/Developer/CommandLineTools",
            installed: false,
            source: nil,
            sourceError: nil,
            aliases: [],
            aliasesError: nil,
            installedVersion: nil,
            latestVersion: nil,
            latestVersionError: nil,
            executablePaths: [],
            executablePathsError: nil,
            popularity: nil,
            lastUpdatedAt: nil,
            homebrewInfo: HomebrewPackageInfo(
                formula: xcodeCLTName,
                description: description,
                homepage: "https://developer.apple.com/xcode/resources/",
                license: nil,
                dependencies: []
            ),
            homebrewInfoError: nil,
            npmHomepage: nil,
            npmPackageInfoError: nil,
            securityState: nil,
            installPackageNames: nil
        )
        return PackageRecommendation(
            packageName: xcodeCLTName,
            installedVersion: nil,
            latestVersion: nil,
            missingPackageNames: [xcodeCLTName],
            detail: detail,
            description: description
        )
    }

    static func agenticToolingPack(missingPackageNames: [String]) -> PackageRecommendation? {
        guard missingPackageNames.isEmpty == false else {
            return nil
        }
        let description =
            L10n.string(
                "Tools agents need. Image manipulation, media processing, language runtimes, search, shell, build, OCR and document conversion tools."
            )
        let detail = PackageDetail(
            packageName: agenticToolingPackName,
            qualifiedName: agenticToolingPackName,
            installRoot: "/opt",
            installed: false,
            source: nil,
            sourceError: nil,
            aliases: [],
            aliasesError: nil,
            installedVersion: nil,
            latestVersion: nil,
            latestVersionError: nil,
            executablePaths: [],
            executablePathsError: nil,
            popularity: nil,
            lastUpdatedAt: nil,
            homebrewInfo: HomebrewPackageInfo(
                formula: agenticToolingPackName,
                description: description,
                homepage: nil,
                license: nil,
                dependencies: agenticToolingPackPackageNames
            ),
            homebrewInfoError: nil,
            npmHomepage: nil,
            npmPackageInfoError: nil,
            securityState: nil,
            installPackageNames: missingPackageNames.map { "brew:\($0)" }
        )
        return PackageRecommendation(
            packageName: agenticToolingPackName,
            installedVersion: nil,
            latestVersion: nil,
            missingPackageNames: missingPackageNames,
            detail: detail,
            description: description
        )
    }

    static func agentPack(missingPackageNames: [String]) -> PackageRecommendation? {
        guard missingPackageNames.isEmpty == false else {
            return nil
        }
        let description =
            L10n.string(
                "Agent CLIs and coding assistants for terminal-native planning, editing, review, model routing and usage inspection."
            )
        let detail = PackageDetail(
            packageName: agentPackName,
            qualifiedName: agentPackName,
            installRoot: "/opt",
            installed: false,
            source: nil,
            sourceError: nil,
            aliases: [],
            aliasesError: nil,
            installedVersion: nil,
            latestVersion: nil,
            latestVersionError: nil,
            executablePaths: [],
            executablePathsError: nil,
            popularity: nil,
            lastUpdatedAt: nil,
            homebrewInfo: HomebrewPackageInfo(
                formula: agentPackName,
                description: description,
                homepage: nil,
                license: nil,
                dependencies: agentPackPackageNames
            ),
            homebrewInfoError: nil,
            npmHomepage: nil,
            npmPackageInfoError: nil,
            securityState: nil,
            installPackageNames: missingPackageNames.map(agentPackInstallPackageName)
        )
        return PackageRecommendation(
            packageName: agentPackName,
            installedVersion: nil,
            latestVersion: nil,
            missingPackageNames: missingPackageNames,
            detail: detail,
            description: description
        )
    }

    private static func agentPackInstallPackageName(_ packageName: String) -> String {
        switch packageName {
        case "codex":
            return "cask:\(packageName)"
        default:
            return "brew:\(packageName)"
        }
    }

    static func unixPlusPlusPack(missingPackageNames: [String]) -> PackageRecommendation? {
        guard missingPackageNames.isEmpty == false else {
            return nil
        }
        let description =
            L10n.string(
                "Modern UNIX command line replacements and operators for search, file inspection, process monitoring, data wrangling and HTTP/DNS work."
            )
        let detail = PackageDetail(
            packageName: unixPlusPlusPackName,
            qualifiedName: unixPlusPlusPackName,
            installRoot: "/opt",
            installed: false,
            source: nil,
            sourceError: nil,
            aliases: [],
            aliasesError: nil,
            installedVersion: nil,
            latestVersion: nil,
            latestVersionError: nil,
            executablePaths: [],
            executablePathsError: nil,
            popularity: nil,
            lastUpdatedAt: nil,
            homebrewInfo: HomebrewPackageInfo(
                formula: unixPlusPlusPackName,
                description: description,
                homepage: nil,
                license: nil,
                dependencies: unixPlusPlusPackPackageNames
            ),
            homebrewInfoError: nil,
            npmHomepage: nil,
            npmPackageInfoError: nil,
            securityState: nil,
            installPackageNames: missingPackageNames.map { "brew:\($0)" }
        )
        return PackageRecommendation(
            packageName: unixPlusPlusPackName,
            installedVersion: nil,
            latestVersion: nil,
            missingPackageNames: missingPackageNames,
            detail: detail,
            description: description
        )
    }

}

struct CommandPaletteItem: Equatable {
    let token: String
    let description: String

    var selectionID: String {
        "command:\(token)"
    }

    var displayName: String {
        "> \(token)"
    }

    var queryText: String {
        "> \(token)"
    }
}

struct PackagePresentation: Equatable {
    let item: PackageListItem
    let detail: PackageDetail?
    let freshness: CGFloat
    let presentationID: String?

    init(
        item: PackageListItem,
        detail: PackageDetail?,
        freshness: CGFloat,
        presentationID: String? = nil
    ) {
        self.item = item
        self.detail = detail
        self.freshness = freshness
        self.presentationID = presentationID
    }

    static func sortsByPackageSearchOrder(
        _ left: PackagePresentation,
        before right: PackagePresentation
    ) -> Bool {
        let leftName = left.packageName ?? left.selectionID
        let rightName = right.packageName ?? right.selectionID
        let leftSortName = leftName.packageSearchOrderName
        let rightSortName = rightName.packageSearchOrderName
        if leftSortName == rightSortName {
            return leftName < rightName
        }
        return leftSortName < rightSortName
    }

    var isInstalledIsotope: Bool {
        switch item {
        case .installed(let record):
            if case .isotope = record.source {
                return true
            }
            return record.name.hasPrefix("isotope:")
        case .recommendation, .available, .command:
            return false
        }
    }

    var hasPlainTextSecretAlert: Bool {
        plainTextSecretAlertSource != nil
    }

    var hasActivePlainTextSecretAlert: Bool {
        hasPlainTextSecretAlert && !plainTextSecretAlertIsGhosted
    }

    func hasMainWindowSecurityAlert(
        resolvedDetail detailOverride: PackageDetail? = nil
    ) -> Bool {
        let resolvedDetail = detailOverride ?? detail
        if case .installed(let record) = item,
           record.securityState?.needsMainWindowSecurityAlert == true {
            return true
        }
        if resolvedDetail?.securityState?.installIsInsecure == true {
            return true
        }
        if resolvedDetail?.securityState?.error?
            .trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false {
            return true
        }
        if resolvedDetail?.securityNotice != nil {
            return true
        }
        guard let resolvedDetail else {
            return hasActivePlainTextSecretAlert
        }
        return PackagePresentation(
            item: item,
            detail: resolvedDetail,
            freshness: freshness,
            presentationID: presentationID
        ).hasActivePlainTextSecretAlert
    }

    var preferredDetailLookupName: String {
        switch item {
        case .available(let result):
            return result.detailLookupName
        case .installed(let record):
            if record.name.isLocalDetectorDisplayPackageName,
               let lookupName = Self.firstNonEmptyPackageName(
                   record.installPackageNames,
                   detail?.installPackageNames
               ) {
                return lookupName
            }
            if let lookupName = Self.groupedVersionedFormulaLookupName(
                record: record,
                detail: detail
            ) {
                return lookupName
            }
            return packageName ?? selectionID
        case .recommendation, .command:
            return packageName ?? selectionID
        }
    }

    var plainTextSecretAlertIsGhosted: Bool {
        if let detail, detail.securityNotice != nil {
            return !detail.installed && !detail.hasLocalPlainTextSecretExposure
        }
        switch item {
        case .installed:
            return false
        case .recommendation(let recommendation):
            return recommendation.detail.securityNotice != nil
                && !recommendation.detail.installed
                && !recommendation.detail.hasLocalPlainTextSecretExposure
        case .available(let result):
            let fallbackDetail = result.fallbackDetail
            return fallbackDetail.securityNotice != nil
                && !fallbackDetail.installed
                && !fallbackDetail.hasLocalPlainTextSecretExposure
        case .command:
            return false
        }
    }

    var plainTextSecretAlertSource: PackageSecurityNotice.Source? {
        if let detail {
            return detail.securityNotice?.source
        }
        switch item {
        case .installed(let record):
            return record.fallbackDetail.securityNotice?.source
        case .recommendation(let recommendation):
            return recommendation.detail.securityNotice?.source
        case .available(let result):
            return result.fallbackDetail.securityNotice?.source
        case .command:
            return nil
        }
    }

    var selectionID: String {
        if let presentationID {
            return presentationID
        }
        switch item {
        case .installed(let record):
            return record.name
        case .recommendation(let recommendation):
            return recommendation.detail.packageName
        case .available(let result):
            return result.name
        case .command(let command):
            return command.selectionID
        }
    }

    var packageName: String? {
        switch item {
        case .installed(let record):
            return record.name
        case .recommendation(let recommendation):
            return recommendation.detail.packageName
        case .available(let result):
            return result.name
        case .command:
            return nil
        }
    }

    var categoryIdentifier: String? {
        switch item {
        case .installed(let record):
            return record.category
        case .available(let result):
            return result.category
        case .recommendation, .command:
            return nil
        }
    }

    var popularityRank: UInt32? {
        switch item {
        case .installed:
            return detail?.popularity?.rank
        case .available(let result):
            return result.rank
        case .recommendation, .command:
            return nil
        }
    }

    var displayName: String {
        switch item {
        case .installed(let record):
            return record.name
        case .recommendation(let recommendation):
            return recommendation.detail.packageName
        case .available(let result):
            return result.name
        case .command(let command):
            return command.displayName
        }
    }

    var versionText: String {
        switch item {
        case .installed(let record):
            if record.installedVersions.count > 1 {
                return record.installedVersions
                    .map(Self.versionLabel)
                    .joined(separator: ", ")
            }
            return Self.versionLabel(record.version)
        case .recommendation(let recommendation):
            if let installedVersion = recommendation.installedVersion,
               let latestVersion = recommendation.latestVersion,
               recommendation.isOutdated {
                return "v\(installedVersion) → v\(latestVersion)"
            }
            if let latestVersion = recommendation.latestVersion {
                return "v\(latestVersion)"
            }
            return recommendation.description
        case .available(let result):
            if case .npm = result.source {
                return result.source?.displayLabel ?? "NPM"
            }
            if case .pip = result.source {
                return result.source?.displayLabel ?? "PyPI"
            }
            if let latestVersion = result.version, !latestVersion.isEmpty {
                return L10n.format("latest %@", latestVersion)
            }
            return result.source?.displayLabel ?? "Homebrew"
        case .command(let command):
            return command.description
        }
    }

    private static func versionLabel(_ version: String) -> String {
        version.hasPrefix("v") ? version : "v\(version)"
    }

    private static func firstNonEmptyPackageName(_ candidates: [String]?...) -> String? {
        for names in candidates {
            guard let names else {
                continue
            }
            for name in names {
                let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
                if trimmed.isEmpty == false {
                    return trimmed
                }
            }
        }
        return nil
    }

    private static func groupedVersionedFormulaLookupName(
        record: PackageRecord,
        detail: PackageDetail?
    ) -> String? {
        guard case .formula(let rootFormula) = record.source,
              let base = formulaVersionedBase(rootFormula),
              unqualifiedBrewPackageName(record.name) == base,
              let lookupName = firstNonEmptyPackageName(
                  record.installPackageNames,
                  detail?.installPackageNames
              ) else {
            return nil
        }

        let formula = unqualifiedBrewPackageName(lookupName)
        guard formulaVersionedBase(formula) == base else {
            return nil
        }
        return "brew:\(formula)"
    }

    private static func formulaVersionedBase(_ formula: String) -> String? {
        let formula = unqualifiedBrewPackageName(formula)
        guard let separator = formula.lastIndex(of: "@") else {
            return nil
        }
        let base = formula[..<separator]
        let version = formula[formula.index(after: separator)...]
        guard !base.isEmpty,
              !version.isEmpty,
              version.unicodeScalars.contains(where: { scalar in
                  scalar.value >= 48 && scalar.value <= 57
              }) else {
            return nil
        }
        return String(base)
    }

    private static func unqualifiedBrewPackageName(_ name: String) -> String {
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.strippingPrefix("brew:") ?? trimmed
    }

    var listSecondaryText: String {
        switch item {
        case .installed:
            return versionText
        case .recommendation(let recommendation):
            return recommendation.description
        case .available(let result):
            if let description = result.description?.trimmingCharacters(
                in: .whitespacesAndNewlines
            ), !description.isEmpty {
                return description
            }
            return versionText
        case .command(let command):
            return command.description
        }
    }

    var pulseLabel: String? {
        guard case .available(let result) = item,
              let pulseKind = result.pulseKind?.trimmingCharacters(in: .whitespacesAndNewlines),
              pulseKind.isEmpty == false else {
            return nil
        }
        switch pulseKind.lowercased() {
        case "new":
            return L10n.string("NEW")
        default:
            return nil
        }
    }

    var commandQueryText: String? {
        switch item {
        case .command(let command):
            return command.queryText
        case .installed, .recommendation, .available:
            return nil
        }
    }
}

private extension PackageDetail {
    var hasLocalPlainTextSecretExposure: Bool {
        securityState?.installIsInsecure == true
    }
}
