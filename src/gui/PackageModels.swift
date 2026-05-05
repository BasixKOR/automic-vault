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
            securityState: securityState
        )
    }

    var fallbackDetail: PackageDetail {
        PackageDetail(
            packageName: name,
            qualifiedName: name,
            installRoot: "/opt",
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
            installPackageNames: nil,
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
            homebrewMigration: homebrewMigration
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

    var securityNotice: PackageSecurityNotice? {
        if let migrationNotice {
            return migrationNotice
        }
        return SecurityCatalog.shared.notice(for: self)
    }

    private var migrationNotice: PackageSecurityNotice? {
        guard let homebrewMigration, homebrewMigration.hazards.isEmpty == false else {
            return nil
        }
        return PackageSecurityNotice(
            source: .isotope,
            applyPackageName: nil,
            headline: "HOMEBREW SECRET MIGRATION",
            body: "Some explicitly installed Homebrew packages have Automic Vault " +
                "radioisotope detectors. Review these before migration because the " +
                "Homebrew packages will be removed after their Vault packages are installed.",
            caveats: .bullets(homebrewMigration.hazardSummaries),
            learnMoreURL: PackageSecurityNotice.defaultLearnMoreURL
        )
    }
}

struct HomebrewMigrationRecommendation: Decodable, Equatable {
    let packages: [HomebrewMigrationPackage]
    let hazards: [HomebrewMigrationHazard]

    var packageNames: [String] {
        packages.map(\.name)
    }

    var installPackageNames: [String] {
        packageNames.map { "brew:\($0)" }
    }

    var hazardSummaries: [String] {
        hazards.map { hazard in
            if let error = hazard.error, error.isEmpty == false {
                return "\(hazard.packageName): isotope:\(hazard.isotopeName) detection failed (\(error))"
            }
            return "\(hazard.packageName): isotope:\(hazard.isotopeName) detector triggered"
        }
    }
}

struct HomebrewMigrationPackage: Decodable, Equatable {
    let name: String
    let version: String?
    let description: String?
}

struct HomebrewMigrationHazard: Decodable, Equatable {
    let packageName: String
    let isotopeName: String
    let error: String?
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
            securityState: nil,
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
    static let agenticToolingPackName = "Agentic Tooling Pack"
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
            "Image manipulation, media processing, language runtimes, search, shell, build, OCR and document conversion tools."
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

    static func homebrewMigration(
        _ migration: HomebrewMigrationRecommendation
    ) -> PackageRecommendation? {
        guard migration.packages.isEmpty == false else {
            return nil
        }
        let packageCount = migration.packages.count
        let hazardCount = migration.hazards.count
        let description = hazardCount > 0
            ? "Migrate \(packageCount) explicit Homebrew packages; \(hazardCount) need radioisotope review."
            : "Migrate \(packageCount) explicitly installed Homebrew packages into the vault."
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
                description: "Installs the explicitly requested packages from " +
                    "/opt/homebrew into Automic Vault. After migration, their " +
                    "Homebrew packages will be deleted.",
                homepage: nil,
                license: nil,
                dependencies: migration.packageNames
            ),
            homebrewInfoError: nil,
            npmHomepage: nil,
            npmPackageInfoError: nil,
            securityState: nil,
            installPackageNames: migration.installPackageNames,
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

    var hasPlainTextSecretAlert: Bool {
        plainTextSecretAlertSource != nil
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
