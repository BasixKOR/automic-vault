import Foundation

enum PackagePack: String, CaseIterable, Identifiable {
    case agenticToolkit
    case agent
    case unixPlusPlus

    var id: String { rawValue }

    var title: String {
        switch self {
        case .agenticToolkit:
            return PackageRecommendation.agenticToolingPackName
        case .agent:
            return PackageRecommendation.agentPackName
        case .unixPlusPlus:
            return PackageRecommendation.unixPlusPlusPackName
        }
    }

    var summary: String {
        switch self {
        case .agenticToolkit:
            return L10n.string(
                "Tools agents need. Image manipulation, media processing, language runtimes, search, shell, build, OCR and document conversion tools."
            )
        case .agent:
            return L10n.string(
                "Agent CLIs and coding assistants for terminal-native planning, editing, review, model routing and usage inspection."
            )
        case .unixPlusPlus:
            return L10n.string(
                "Modern UNIX command line replacements and operators for search, file inspection, process monitoring, data wrangling and HTTP/DNS work."
            )
        }
    }

    var packageNames: [String] {
        switch self {
        case .agenticToolkit:
            return PackageRecommendation.agenticToolingPackPackageNames
        case .agent:
            return PackageRecommendation.agentPackPackageNames
        case .unixPlusPlus:
            return PackageRecommendation.unixPlusPlusPackPackageNames
        }
    }

    var installPackageNames: [String] {
        switch self {
        case .agenticToolkit, .unixPlusPlus:
            return packageNames.map { "brew:\($0)" }
        case .agent:
            return packageNames.map { packageName in
                packageName == "codex" ? "cask:\(packageName)" : "brew:\(packageName)"
            }
        }
    }

    var systemImage: String {
        switch self {
        case .agenticToolkit:
            return "shippingbox.and.arrow.backward"
        case .agent:
            return "terminal"
        case .unixPlusPlus:
            return "square.stack.3d.up"
        }
    }
}
