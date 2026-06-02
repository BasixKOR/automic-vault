import Foundation

struct AutomicVaultDeepLink: Equatable {
    enum Action: Equatable {
        case install(packageNames: [String])
    }

    let action: Action

    init?(url: URL) {
        guard url.scheme?.lowercased() == "automicvault" else {
            return nil
        }

        let actionName = Self.actionName(from: url)
        switch actionName {
        case "install":
            let packageNames = Self.installPackageNames(from: url)
            guard packageNames.isEmpty == false else {
                return nil
            }
            action = .install(packageNames: packageNames)
        default:
            return nil
        }
    }

    private static func actionName(from url: URL) -> String {
        if let host = url.host?.trimmingCharacters(in: .whitespacesAndNewlines),
           host.isEmpty == false {
            return host.lowercased()
        }

        return url.pathComponents
            .dropFirst()
            .first?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
            ?? ""
    }

    private static func installPackageNames(from url: URL) -> [String] {
        var candidates: [String] = []

        if let components = URLComponents(url: url, resolvingAgainstBaseURL: false) {
            for item in components.queryItems ?? [] {
                let name = item.name.lowercased()
                switch name {
                case "package", "package[]":
                    candidates.append(item.value ?? "")
                case "packages":
                    candidates.append(contentsOf: splitPackageList(item.value ?? ""))
                default:
                    continue
                }
            }
        }

        if actionName(from: url) == "install" {
            candidates.append(
                contentsOf: url.pathComponents
                    .dropFirst()
                    .filter { $0 != "install" }
            )
        }

        return normalizedPackageNames(candidates)
    }

    private static func splitPackageList(_ value: String) -> [String] {
        value.split { character in
            character == "," || character == "\n" || character == "\r" || character == "\t"
        }
        .map(String.init)
    }

    private static func normalizedPackageNames(_ candidates: [String]) -> [String] {
        let allowed = CharacterSet(
            charactersIn: "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789:@/._+-"
        )
        var seen = Set<String>()
        var result: [String] = []

        for candidate in candidates {
            let trimmed = candidate.trimmingCharacters(in: .whitespacesAndNewlines)
            guard trimmed.isEmpty == false,
                  trimmed.count <= 200,
                  trimmed.rangeOfCharacter(from: allowed.inverted) == nil,
                  seen.insert(trimmed).inserted else {
                continue
            }
            result.append(trimmed)
        }

        return result
    }
}
