import AppKit
import Foundation

private let packageSearchOrderPrefixes = [
    "brew:",
    "cask:",
    "isotope:",
    "av:",
    "npm:",
    "pip:"
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
}

struct PackageRecord: Decodable, Equatable {
    let name: String
    let source: PackageSource?
    let version: String
    let description: String?
    let latestVersion: String?
    let securityState: PackageSecurityState?
    let installRoot: String?
    let installPackageNames: [String]?
    let installedVersions: [String]
    let isHomebrewMigrationCandidate: Bool
    let isUnsupportedHomebrewInstall: Bool

    enum CodingKeys: String, CodingKey {
        case name
        case source
        case version
        case description
        case latestVersion
        case securityState
        case installRoot
        case installPackageNames
        case installedVersions
        case isHomebrewMigrationCandidate
        case isUnsupportedHomebrewInstall
    }

    init(
        name: String,
        source: PackageSource?,
        version: String,
        description: String?,
        latestVersion: String? = nil,
        securityState: PackageSecurityState?,
        installRoot: String? = nil,
        installPackageNames: [String]? = nil,
        installedVersions: [String] = [],
        isHomebrewMigrationCandidate: Bool = false,
        isUnsupportedHomebrewInstall: Bool = false
    ) {
        self.name = name
        self.source = source
        self.version = version
        self.description = description
        self.latestVersion = latestVersion
        self.securityState = securityState
        self.installRoot = installRoot
        self.installPackageNames = installPackageNames
        self.installedVersions = installedVersions
        self.isHomebrewMigrationCandidate = isHomebrewMigrationCandidate
        self.isUnsupportedHomebrewInstall = isUnsupportedHomebrewInstall
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        name = try container.decode(String.self, forKey: .name)
        source = try container.decodeIfPresent(PackageSource.self, forKey: .source)
        version = try container.decode(String.self, forKey: .version)
        description = try container.decodeIfPresent(String.self, forKey: .description)
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
        isHomebrewMigrationCandidate = try container.decodeIfPresent(
            Bool.self,
            forKey: .isHomebrewMigrationCandidate
        ) ?? false
        isUnsupportedHomebrewInstall = try container.decodeIfPresent(
            Bool.self,
            forKey: .isUnsupportedHomebrewInstall
        ) ?? false
    }

    var isOutdated: Bool {
        guard let latestVersion, !latestVersion.isEmpty else {
            return false
        }
        return version != latestVersion
    }

    func applying(outdated: OutdatedPackageRecord) -> PackageRecord {
        PackageRecord(
            name: name,
            source: source,
            version: version,
            description: description,
            latestVersion: outdated.latestVersion,
            securityState: securityState,
            installRoot: installRoot,
            installPackageNames: installPackageNames,
            installedVersions: installedVersions,
            isHomebrewMigrationCandidate: isHomebrewMigrationCandidate,
            isUnsupportedHomebrewInstall: isUnsupportedHomebrewInstall
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
                homepage: nil,
                license: nil,
                dependencies: []
            ),
            homebrewInfoError: nil,
            npmHomepage: nil,
            npmPackageInfoError: nil,
            securityState: securityState,
            installPackageNames: installPackageNames,
            homebrewMigration: nil
        )
    }
}

struct PackageSecurityState: Decodable, Equatable {
    let isotopeName: String
    let installIsInsecure: Bool
    let error: String?
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
    let homebrewMigration: HomebrewMigrationRecommendation?
    let versionOptions: [PackageVersionOption]

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
        homebrewMigration: HomebrewMigrationRecommendation?,
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
        self.homebrewMigration = homebrewMigration
        self.versionOptions = versionOptions
    }

    var primaryDescription: String {
        if let description = homebrewInfo?.description, !description.isEmpty {
            return description
        }
        if installed {
            return "Installed component record available in the local vault."
        }
        return "Component metadata is available, but the local vault has not initialized it."
    }

    var metadataLine: String {
        let version = installedVersion ?? latestVersion ?? "unversioned"
        return [version, source?.displayLabel, installed ? "installed" : "uninstalled"]
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
        guard let raw = rawHomepage else {
            return nil
        }
        let normalized = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalized.isEmpty,
              let url = URL(string: normalized),
              let scheme = url.scheme else {
            return nil
        }
        guard scheme == "http" || scheme == "https" else {
            return nil
        }
        if isOutdated, let latestReleaseURL = url.githubLatestReleaseURL {
            return latestReleaseURL
        }
        return url.githubRepositoryReadmeURL
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
        if isUnsupportedHomebrewInstall {
            return "Tapped Homebrew formulae are detected but cannot be migrated to Automic Vault."
        }
        if isHomebrewMigrationCandidate {
            return "av install \(helperPackageNames.joined(separator: " "))"
        }
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
            homebrewMigration: homebrewMigration,
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
            homebrewMigration: homebrewMigration,
            versionOptions: versionOptions
        )
    }

    private var rawHomepage: String? {
        homebrewInfo?.homepage ?? npmHomepage
    }

    var isAutomicVaultCLT: Bool {
        packageName == PackageRecommendation.automicVaultCLTName
    }

    var isXcodeCLT: Bool {
        packageName == PackageRecommendation.xcodeCLTName
    }

    var isHomebrewMigrationCandidate: Bool {
        installRoot.hasPrefix("/opt/homebrew/")
            && (packageName.hasPrefix("brew:") || packageName.hasPrefix("cask:"))
            && installPackageNames?.isEmpty == false
            && installed
    }

    var isUnsupportedHomebrewInstall: Bool {
        installRoot.hasPrefix("/opt/homebrew/")
            && (packageName.hasPrefix("brew:") || packageName.hasPrefix("cask:"))
            && !isHomebrewMigrationCandidate
            && installed
    }

    var securityNotice: PackageSecurityNotice? {
        return SecurityCatalog.shared.notice(for: self)
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
        if displayName == "@latest", let aliasName {
            title += " (\(aliasName))"
        }
        if isRecommended {
            title += " recommended"
        }
        return title
    }
}

struct HomebrewMigrationRecommendation: Decodable, Equatable {
    let packages: [HomebrewMigrationPackage]
    let hazards: [HomebrewMigrationHazard]

    var packageNames: [String] {
        packages.map(\.name)
    }

    var installPackageNames: [String] {
        packages.filter(\.isMigratable).map { package in
            let packageName = package.name
            return packageName.hasPrefix("brew:") || packageName.hasPrefix("cask:")
                ? packageName
                : "brew:\(packageName)"
        }
    }

    var hazardSummaries: [String] {
        hazards.map { hazard in
            if let error = hazard.error, error.isEmpty == false {
                return "\(hazard.packageName): isotope:\(hazard.isotopeName) detection failed (\(error))"
            }
            return "\(hazard.packageName): isotope:\(hazard.isotopeName) detector triggered"
        }
    }

    var learnMoreURL: URL? {
        hazards.lazy.compactMap(\.radioisotopeReadmeURL).first
    }
}

struct HomebrewMigrationPackage: Decodable, Equatable {
    let name: String
    let version: String?
    let description: String?
    let tap: String?
    let isMigratable: Bool
    let securityState: PackageSecurityState?

    var source: PackageSource {
        if let caskName = name.strippingPrefix("cask:") {
            return .cask(caskName: caskName)
        }
        if let formula = name.strippingPrefix("brew:") {
            return .formula(rootFormula: formula)
        }
        return .formula(rootFormula: name)
    }

    var installRoot: String {
        if let caskName = name.strippingPrefix("cask:") {
            return "/opt/homebrew/Caskroom/\(caskName)"
        }
        let formula = (name.strippingPrefix("brew:") ?? name)
            .split(separator: "/")
            .last
            .map(String.init)
            ?? name
        return "/opt/homebrew/Cellar/\(formula)"
    }

    var record: PackageRecord {
        PackageRecord(
            name: normalizedName,
            source: source,
            version: version ?? "installed",
            description: description,
            securityState: securityState,
            installRoot: installRoot,
            installPackageNames: isMigratable ? [normalizedName] : nil,
            isHomebrewMigrationCandidate: isMigratable,
            isUnsupportedHomebrewInstall: !isMigratable
        )
    }

    private var normalizedName: String {
        if name.hasPrefix("brew:") || name.hasPrefix("cask:") {
            return name
        }
        return "brew:\(name)"
    }
}

struct HomebrewMigrationHazard: Decodable, Equatable {
    let packageName: String
    let isotopeName: String
    let error: String?

    var radioisotopeReadmeURL: URL? {
        var pathAllowed = CharacterSet.urlPathAllowed
        pathAllowed.remove("/")
        guard let isotopePath = isotopeName.addingPercentEncoding(
            withAllowedCharacters: pathAllowed
        ) else {
            return nil
        }
        return URL(
            string: "https://github.com/automic-vault/radioisotopes/tree/main/\(isotopePath)#readme"
        )
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
            return "npm"
        case .pip:
            return "PyPI"
        }
    }
}

struct HomebrewPackageInfo: Decodable, Equatable {
    let formula: String
    let description: String?
    let homepage: String?
    let license: String?
    let dependencies: [String]
}

struct PackageSearchResult: Decodable, Equatable {
    let name: String
    let source: PackageSource?
    let version: String?
    let description: String?
    let homepage: String?
    let dependencies: [String]
    let securityState: PackageSecurityState?

    enum CodingKeys: String, CodingKey {
        case name = "packageName"
        case legacyName = "name"
        case source
        case version = "latestVersion"
        case legacyVersion = "version"
        case description = "summary"
        case legacyDescription = "description"
        case homepage
        case dependencies
        case securityState
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
        dependencies = try container.decodeIfPresent([String].self, forKey: .dependencies) ?? []
        securityState = try container.decodeIfPresent(
            PackageSecurityState.self,
            forKey: .securityState
        )
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
            lastUpdatedAt: nil,
            homebrewInfo: HomebrewPackageInfo(
                formula: name,
                description: description,
                homepage: homepage,
                license: nil,
                dependencies: dependencies
            ),
            homebrewInfoError: nil,
            npmHomepage: nil,
            npmPackageInfoError: nil,
            securityState: securityState,
            installPackageNames: nil,
            homebrewMigration: nil
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
}

struct PackageSearchPage: Decodable, Equatable {
    let packages: [PackageSearchResult]
    let totalCount: Int
    let nextOffset: Int?
}

private extension URL {
    var githubLatestReleaseURL: URL? {
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
        components.path = "/\(pathComponents[0])/\(pathComponents[1])/releases/latest"
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
    static let homebrewMigrationName = "Homebrew Migration"
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

    static func automicVaultCLT(
        installedVersion: String?,
        latestVersion: String,
        missingToolNames: [String]
    ) -> PackageRecommendation {
        let description = missingToolNames.isEmpty
            ? "Bundled command line tools are installed but need updating."
            : "Installs the Automic Vault command line tool"
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
            installPackageNames: nil,
            homebrewMigration: nil
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
            "Installs Apple's Command Line Tools for compilers, SDK headers and system build utilities."
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
            installPackageNames: nil,
            homebrewMigration: nil
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
            "Tools agents need. Image manipulation, media processing, language runtimes, search, shell, build, OCR and document conversion tools."
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
            installPackageNames: missingPackageNames.map { "brew:\($0)" },
            homebrewMigration: nil
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
            "Agent CLIs and coding assistants for terminal-native planning, editing, review, model routing and usage inspection."
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
            installPackageNames: missingPackageNames.map(agentPackInstallPackageName),
            homebrewMigration: nil
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
            "Modern UNIX command line replacements and operators for search, file inspection, process monitoring, data wrangling and HTTP/DNS work."
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
            installPackageNames: missingPackageNames.map { "brew:\($0)" },
            homebrewMigration: nil
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

    static func homebrewMigration(
        _ migration: HomebrewMigrationRecommendation
    ) -> PackageRecommendation? {
        let installPackageNames = migration.installPackageNames
        guard installPackageNames.isEmpty == false else {
            return nil
        }
        let packageCount = installPackageNames.count
        let hazardCount = migration.hazards.count
        let description = hazardCount > 0
            ? "Migrate \(packageCount) Homebrew packages and casks; \(hazardCount) need radioisotope review."
            : "Migrate \(packageCount) Homebrew packages and casks into the vault."
        let detail = PackageDetail(
            packageName: homebrewMigrationName,
            qualifiedName: homebrewMigrationName,
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
                formula: homebrewMigrationName,
                description: "Installs selected packages and casks from /opt/homebrew " +
                    "into Automic Vault. After migration, their " +
                    "Homebrew packages will be uninstalled.",
                homepage: nil,
                license: nil,
                dependencies: migration.packageNames
            ),
            homebrewInfoError: nil,
            npmHomepage: nil,
            npmPackageInfoError: nil,
            securityState: nil,
            installPackageNames: installPackageNames,
            homebrewMigration: migration
        )
        return PackageRecommendation(
            packageName: homebrewMigrationName,
            installedVersion: nil,
            latestVersion: nil,
            missingPackageNames: migration.packageNames,
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

    var isHomebrewMigrationCandidate: Bool {
        switch item {
        case .installed(let record):
            return record.isHomebrewMigrationCandidate
        case .recommendation, .available, .command:
            return false
        }
    }

    var isHomebrewInstall: Bool {
        switch item {
        case .installed(let record):
            return record.isHomebrewMigrationCandidate || record.isUnsupportedHomebrewInstall
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

    var plainTextSecretAlertIsGhosted: Bool {
        if let detail, detail.securityNotice != nil {
            return !detail.installed
        }
        switch item {
        case .installed:
            return false
        case .recommendation(let recommendation):
            return recommendation.detail.securityNotice != nil
                && !recommendation.detail.installed
        case .available(let result):
            let fallbackDetail = result.fallbackDetail
            return fallbackDetail.securityNotice != nil && !fallbackDetail.installed
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
        case .available, .command:
            return nil
        }
    }

    var selectionID: String {
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
                return record.installedVersions.joined(separator: ", ")
            }
            return "v\(record.version)"
        case .recommendation(let recommendation):
            if let installedVersion = recommendation.installedVersion,
               let latestVersion = recommendation.latestVersion,
               recommendation.isOutdated {
                return "v\(installedVersion) -> v\(latestVersion)"
            }
            if let latestVersion = recommendation.latestVersion {
                return "v\(latestVersion)"
            }
            return recommendation.description
        case .available(let result):
            if let latestVersion = result.version, !latestVersion.isEmpty {
                return "latest \(latestVersion)"
            }
            return result.source?.displayLabel ?? "Homebrew"
        case .command(let command):
            return command.description
        }
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

    var commandQueryText: String? {
        switch item {
        case .command(let command):
            return command.queryText
        case .installed, .recommendation, .available:
            return nil
        }
    }
}
