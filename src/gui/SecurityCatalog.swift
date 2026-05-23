import Foundation

private func radioisotopeReadmeURL(for isotopeName: String) -> URL? {
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

struct PackageSecurityNotice: Equatable {
    enum Caveats: Equatable {
        case paragraph(String)
        case bullets([String])
    }

    enum Source: Equatable {
        case isotope
        case enrichmentManifest
    }

    let source: Source
    let applyPackageName: String?
    let headline: String
    let body: String
    let reasons: [String]
    let caveats: Caveats?
    let learnMoreURL: URL

    static let defaultLearnMoreURL = URL(string: "https://github.com/automic-vault/")!

    init(
        source: Source,
        applyPackageName: String?,
        headline: String = Self.defaultHeadline,
        body: String = Self.defaultBody,
        reasons: [String] = [],
        caveats: Caveats? = nil,
        learnMoreURL: URL = Self.defaultLearnMoreURL
    ) {
        self.source = source
        self.applyPackageName = applyPackageName
        self.headline = headline
        self.body = body
        self.reasons = reasons
        self.caveats = caveats
        self.learnMoreURL = learnMoreURL
    }

    fileprivate static var defaultHeadline: String {
        "PLAIN TEXT SECRET"
    }

    fileprivate static var defaultBody: String {
        "This package stores its secrets in plain text. " +
            "Automic Vault isotopes are aftermarket open source packages " +
            "modded by packaging experts to incorporate Automic Vault’s " +
            "security bolstering technologies."
    }

    fileprivate static var detectorOnlyHeadline: String {
        "LOCAL SECRET EXPOSURE"
    }

    fileprivate static var detectorOnlyBody: String {
        "Automic Vault detected plaintext secret exposure for this package. " +
            "A detector exists, but Automic Vault does not yet provide " +
            "migration or package modification for this tool."
    }
}

final class SecurityCatalog {
    static let shared = SecurityCatalog(bundle: .main)

    private let isotopeIdentifiers: Set<String>
    private let enrichmentIdentifiers: Set<String>

    init(bundle: Bundle) {
        let isotopePackages = Self.loadIsotopePackages(bundle: bundle)
        isotopeIdentifiers = Set(isotopePackages.keys)
        enrichmentIdentifiers = Self.loadEnrichmentIdentifiers(bundle: bundle)
        self.isotopePackages = isotopePackages
    }

    func notice(for detail: PackageDetail) -> PackageSecurityNotice? {
        let identifiers = packageIdentifiers(for: detail)
        if identifiers.isDisjoint(with: enrichmentIdentifiers) == false {
            return PackageSecurityNotice(
                source: .enrichmentManifest,
                applyPackageName: nil
            )
        }
        let matchedIsotopePackages = identifiers.compactMap { isotopePackages[$0] }
        if let matchedIsotope = matchedIsotopePackages.first {
            let remediationAvailable = detail.securityState?.remediationAvailable ?? true
            if let securityState = detail.securityState,
               securityState.isotopeName == matchedIsotope.isotopeName {
                guard securityState.installIsInsecure else {
                    return nil
                }
            } else if case .isotope = detail.source {
                return nil
            }
            return PackageSecurityNotice(
                source: .isotope,
                applyPackageName: remediationAvailable ? matchedIsotope.name : nil,
                headline: matchedIsotope.justification?.title
                    ?? (remediationAvailable
                        ? PackageSecurityNotice.defaultHeadline
                        : PackageSecurityNotice.detectorOnlyHeadline),
                body: matchedIsotope.justification?.detail
                    ?? (remediationAvailable
                        ? PackageSecurityNotice.defaultBody
                        : PackageSecurityNotice.detectorOnlyBody),
                reasons: detail.securityState?.reasons ?? [],
                caveats: matchedIsotope.caveats?.noticeCaveats,
                learnMoreURL: matchedIsotope.learnMoreURL
                    ?? PackageSecurityNotice.defaultLearnMoreURL
            )
        }
        if let securityState = detail.securityState,
           securityState.installIsInsecure {
            let remediationAvailable = securityState.remediationAvailable
            return PackageSecurityNotice(
                source: .isotope,
                applyPackageName: remediationAvailable
                    ? "isotope:\(securityState.isotopeName)"
                    : nil,
                headline: remediationAvailable
                    ? PackageSecurityNotice.defaultHeadline
                    : PackageSecurityNotice.detectorOnlyHeadline,
                body: remediationAvailable
                    ? PackageSecurityNotice.defaultBody
                    : PackageSecurityNotice.detectorOnlyBody,
                reasons: securityState.reasons,
                learnMoreURL: radioisotopeReadmeURL(for: securityState.isotopeName)
                    ?? PackageSecurityNotice.defaultLearnMoreURL
            )
        }
        return nil
    }

    private let isotopePackages: [String: IsotopeRecord]

    private func packageIdentifiers(for detail: PackageDetail) -> Set<String> {
        var identifiers = Set<String>()

        identifiers.formUnion(
            [
                detail.packageName,
                detail.qualifiedName,
                detail.helperPackageName
            ].compactMap(Self.normalizeIdentifier)
        )

        identifiers.formUnion(detail.aliases.compactMap(Self.normalizeIdentifier))
        identifiers.formUnion(
            detail.executablePaths.compactMap { path in
                Self.normalizeIdentifier(URL(fileURLWithPath: path).lastPathComponent)
            }
        )

        if let source = detail.source {
            identifiers.formUnion(source.identifiers)
        }

        let currentIdentifiers = identifiers
        for identifier in currentIdentifiers {
            if let cliStem = Self.cliStem(for: identifier) {
                identifiers.insert(cliStem)
            }
        }

        return identifiers
    }

    private static func loadIsotopePackages(bundle: Bundle) -> [String: IsotopeRecord] {
        guard let data = resourceData(named: "combined", bundle: bundle) else {
            return [:]
        }
        guard
            let combined = try? JSONDecoder().decode(CombinedData.self, from: data)
        else {
            return [:]
        }

        var packages: [String: IsotopeRecord] = [:]
        for (name, record) in combined.sources.isotopes {
            for identifier in candidateIdentifiers(for: name) {
                packages[identifier] = record
            }
            for identifier in candidateIdentifiers(for: record.repository) {
                packages[identifier] = record
            }
            for identifier in candidateIdentifiers(for: record.upstreamRepository) {
                packages[identifier] = record
            }
            for identifier in candidateIdentifiers(for: record.replaces) {
                packages[identifier] = record
            }
            for identifier in candidateIdentifiers(for: record.modifies) {
                packages[identifier] = record
            }
        }
        return packages
    }

    private static func loadEnrichmentIdentifiers(bundle: Bundle) -> Set<String> {
        guard let data = resourceData(named: "enrichment-manifests", bundle: bundle) else {
            return []
        }
        guard let manifests = try? JSONDecoder().decode([String].self, from: data) else {
            return []
        }
        return Set(manifests.compactMap(normalizeIdentifier))
    }

    private static func resourceData(named name: String, bundle: Bundle) -> Data? {
        guard let url = bundle.url(forResource: name, withExtension: "json") else {
            return nil
        }
        return try? Data(contentsOf: url)
    }

    private static func candidateIdentifiers(for rawValue: String?) -> Set<String> {
        guard let rawValue else { return [] }

        var identifiers = Set<String>()
        if let normalized = normalizeIdentifier(rawValue) {
            identifiers.insert(normalized)
            if let cliStem = cliStem(for: normalized) {
                identifiers.insert(cliStem)
            }
        }

        if let leaf = rawValue.split(separator: "/").last,
           let normalizedLeaf = normalizeIdentifier(String(leaf)) {
            identifiers.insert(normalizedLeaf)
            if let cliStem = cliStem(for: normalizedLeaf) {
                identifiers.insert(cliStem)
            }
        }

        return identifiers
    }

    private static func cliStem(for identifier: String) -> String? {
        if identifier.hasSuffix("-cli") {
            let stem = String(identifier.dropLast(4))
            return stem.isEmpty ? nil : stem
        }
        if identifier.hasSuffix("cli") {
            let stem = String(identifier.dropLast(3))
            return stem.isEmpty ? nil : stem
        }
        return nil
    }

    fileprivate static func normalizeIdentifier(_ rawValue: String?) -> String? {
        guard let rawValue else { return nil }
        let normalized = rawValue
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
        guard normalized.isEmpty == false else {
            return nil
        }
        if let separator = normalized.firstIndex(of: ":") {
            let suffix = normalized[normalized.index(after: separator)...]
            return suffix.isEmpty ? nil : String(suffix)
        }
        return normalized
    }
}

private struct CombinedData: Decodable {
    let sources: CombinedDataSources
}

private struct CombinedDataSources: Decodable {
    let isotopes: [String: IsotopeRecord]
}

private struct IsotopeRecord: Decodable {
    let name: String
    let replaces: String?
    let modifies: String?
    let repository: String?
    let upstreamRepository: String?
    let justification: IsotopeJustification?
    let caveats: IsotopeCaveats?

    var isotopeName: String {
        if let separator = name.firstIndex(of: ":") {
            return String(name[name.index(after: separator)...])
        }
        return name
    }

    var learnMoreURL: URL? {
        guard let repository,
              repository.contains("/") else {
            return nil
        }
        if repository.lowercased() == "automic-vault/radioisotopes" {
            return radioisotopeReadmeURL(for: isotopeName)
        }
        return URL(string: "https://github.com/\(repository)#readme")
    }
}

private struct IsotopeJustification: Decodable {
    let title: String
    let detail: String
}

private enum IsotopeCaveats: Decodable {
    case paragraph(String)
    case bullets([String])

    init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if let paragraph = try? container.decode(String.self) {
            self = .paragraph(paragraph)
            return
        }
        self = .bullets(try container.decode([String].self))
    }

    var noticeCaveats: PackageSecurityNotice.Caveats? {
        switch self {
        case .paragraph(let paragraph):
            let trimmed = paragraph.trimmingCharacters(in: .whitespacesAndNewlines)
            return trimmed.isEmpty ? nil : .paragraph(trimmed)
        case .bullets(let bullets):
            let trimmedBullets = bullets
                .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
                .filter { $0.isEmpty == false }
            return trimmedBullets.isEmpty ? nil : .bullets(trimmedBullets)
        }
    }
}

private extension PackageSource {
    var identifiers: Set<String> {
        switch self {
        case .formula(let rootFormula):
            return Set([rootFormula].compactMap(SecurityCatalog.normalizeIdentifier))
        case .cask(let caskName):
            return Set([caskName].compactMap(SecurityCatalog.normalizeIdentifier))
        case .isotope(let isotopeName):
            return Set([isotopeName].compactMap(SecurityCatalog.normalizeIdentifier))
        case .vendor(let vendorName):
            return Set([vendorName].compactMap(SecurityCatalog.normalizeIdentifier))
        case .npm(let packageName):
            return Set([packageName].compactMap(SecurityCatalog.normalizeIdentifier))
        case .pip(let packageName):
            return Set([packageName].compactMap(SecurityCatalog.normalizeIdentifier))
        }
    }
}
