import Foundation

private let authorizationRedaction = "<redacted>"

public func redactedAuthorizationArguments(tool: String, arguments: [String]) -> [String] {
    let tool = URL(fileURLWithPath: tool).lastPathComponent.lowercased()
    var result: [String] = []
    var index = 0

    while index < arguments.count {
        let argument = arguments[index]

        if isSensitiveValueFlag(argument) {
            result.append(argument)
            if index + 1 < arguments.count {
                result.append(authorizationRedaction)
                index += 2
            } else {
                index += 1
            }
            continue
        }

        if let redacted = redactingInlineSensitiveFlag(argument) {
            result.append(redacted)
            index += 1
            continue
        }

        if ["uaa", "uaa-cli"].contains(tool), isUAASensitiveValueFlag(argument) {
            result.append(argument)
            if index + 1 < arguments.count {
                result.append(authorizationRedaction)
                index += 2
            } else {
                index += 1
            }
            continue
        }

        if ["uaa", "uaa-cli"].contains(tool), let redacted = redactingInlineUAAFlag(argument) {
            result.append(redacted)
            index += 1
            continue
        }

        if ["openhue", "openhue-cli"].contains(tool), ["-k", "--key"].contains(argument.lowercased()) {
            result.append(argument)
            if index + 1 < arguments.count {
                result.append(authorizationRedaction)
                index += 2
            } else {
                index += 1
            }
            continue
        }

        if ["openhue", "openhue-cli"].contains(tool),
           (argument.lowercased().hasPrefix("--key=")
               || argument.lowercased().hasPrefix("-k") && argument.count > 2)
        {
            let prefix = argument.lowercased().hasPrefix("--key=") ? "--key=" : String(argument.prefix(2))
            result.append("\(prefix)\(authorizationRedaction)")
            index += 1
            continue
        }

        if tool == "curl", ["-u", "--user"].contains(argument.lowercased()) {
            result.append(argument)
            if index + 1 < arguments.count {
                result.append(redactingUserPassword(arguments[index + 1]))
                index += 2
            } else {
                index += 1
            }
            continue
        }

        if tool == "curl", argument == "-H" || argument.lowercased() == "--header" {
            result.append(argument)
            if index + 1 < arguments.count {
                result.append(redactingHeader(arguments[index + 1]))
                index += 2
            } else {
                index += 1
            }
            continue
        }

        if tool == "curl", let redacted = redactingInlineCurlArgument(argument) {
            result.append(redacted)
            index += 1
            continue
        }

        if tool == "sshpass", argument.lowercased() == "-p" {
            result.append(argument)
            if index + 1 < arguments.count {
                result.append(authorizationRedaction)
                index += 2
            } else {
                index += 1
            }
            continue
        }

        if tool == "sshpass", argument.lowercased().hasPrefix("-p"), argument.count > 2 {
            result.append("\(argument.prefix(2))\(authorizationRedaction)")
            index += 1
            continue
        }

        result.append(redactingStandaloneValue(argument))
        index += 1
    }

    return result
}

private func isUAASensitiveValueFlag(_ argument: String) -> Bool {
    ["-s", "-p", "--client_secret", "--old_secret", "--secret", "--password"]
        .contains(argument.lowercased())
}

private func redactingInlineUAAFlag(_ argument: String) -> String? {
    let lowercased = argument.lowercased()
    for flag in ["--client_secret=", "--old_secret=", "--secret=", "--password="]
    where lowercased.hasPrefix(flag) {
        return "\(argument.prefix(flag.count))\(authorizationRedaction)"
    }
    if (lowercased.hasPrefix("-s") || lowercased.hasPrefix("-p")) && argument.count > 2 {
        return "\(argument.prefix(2))\(authorizationRedaction)"
    }
    return nil
}

private let sensitiveValueFlags: Set<String> = [
    "--access-token",
    "--api-key",
    "--api-token",
    "--apikey",
    "--auth-token",
    "--authorization",
    "--bearer-token",
    "--client-secret",
    "--cookie",
    "--credentials",
    "--password",
    "--passwd",
    "--passphrase",
    "--private-key",
    "--refresh-token",
    "--secret-access-key",
    "--secret-key",
    "--session-token",
    "--token",
    "--webhook-secret",
]

private func isSensitiveValueFlag(_ argument: String) -> Bool {
    sensitiveValueFlags.contains(argument.lowercased())
}

private func redactingInlineSensitiveFlag(_ argument: String) -> String? {
    guard let equals = argument.firstIndex(of: "=") else { return nil }
    let flag = String(argument[..<equals])
    guard isSensitiveValueFlag(flag) else { return nil }
    return "\(flag)=\(authorizationRedaction)"
}

private func redactingInlineCurlArgument(_ argument: String) -> String? {
    let lowercased = argument.lowercased()
    if lowercased.hasPrefix("--user=") {
        let value = String(argument.dropFirst("--user=".count))
        return "\(argument.prefix("--user=".count))\(redactingUserPassword(value))"
    }
    if lowercased.hasPrefix("-u"), argument.count > 2 {
        return "\(argument.prefix(2))\(redactingUserPassword(String(argument.dropFirst(2))))"
    }
    if lowercased.hasPrefix("--header=") {
        let value = String(argument.dropFirst("--header=".count))
        return "\(argument.prefix("--header=".count))\(redactingHeader(value))"
    }
    if argument.hasPrefix("-H"), argument.count > 2 {
        return "\(argument.prefix(2))\(redactingHeader(String(argument.dropFirst(2))))"
    }
    return nil
}

private func redactingStandaloneValue(_ value: String) -> String {
    if let assignment = redactingEnvironmentAssignment(value) {
        return assignment
    }
    let header = redactingHeader(value)
    if header != value {
        return header
    }
    let url = redactingURL(value)
    if url != value {
        return url
    }
    if isRecognizableCredential(value) {
        return authorizationRedaction
    }
    return value
}

private func redactingEnvironmentAssignment(_ argument: String) -> String? {
    guard let equals = argument.firstIndex(of: "=") else { return nil }
    let name = String(argument[..<equals])
    guard isEnvironmentName(name), isSensitiveName(name) else { return nil }
    return "\(name)=\(authorizationRedaction)"
}

private func isEnvironmentName(_ value: String) -> Bool {
    guard let first = value.unicodeScalars.first,
          CharacterSet.letters.union(CharacterSet(charactersIn: "_")).contains(first)
    else { return false }
    return value.unicodeScalars.allSatisfy {
        CharacterSet.alphanumerics.union(CharacterSet(charactersIn: "_")).contains($0)
    }
}

private let sensitiveNames: Set<String> = [
    "access_key",
    "access_token",
    "api_key",
    "apikey",
    "api_token",
    "auth",
    "auth_token",
    "authorization",
    "bearer_token",
    "client_secret",
    "cookie",
    "credentials",
    "pass",
    "passwd",
    "passphrase",
    "password",
    "private_key",
    "refresh_token",
    "secret",
    "secret_access_key",
    "secret_key",
    "session_token",
    "signature",
    "token",
    "webhook_secret",
]

private let visibleMetadataNames: Set<String> = [
    "key_id",
    "key_name",
    "profile",
    "secret_id",
    "secret_name",
    "token_id",
    "token_name",
]

private func isSensitiveName(_ name: String) -> Bool {
    let normalized = name
        .lowercased()
        .replacingOccurrences(of: "-", with: "_")
    guard !visibleMetadataNames.contains(normalized) else { return false }
    if sensitiveNames.contains(normalized) { return true }
    return sensitiveNames.contains { normalized.hasSuffix("_\($0)") }
}

private let sensitiveHeaderNames: Set<String> = [
    "api-key",
    "authorization",
    "cookie",
    "proxy-authorization",
    "set-cookie",
    "x-access-token",
    "x-api-key",
    "x-auth-token",
]

private func redactingHeader(_ argument: String) -> String {
    guard let colon = argument.firstIndex(of: ":") else { return argument }
    let name = argument[..<colon].trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
    guard sensitiveHeaderNames.contains(name) else { return argument }
    return "\(argument[..<colon]): \(authorizationRedaction)"
}

private func redactingUserPassword(_ value: String) -> String {
    guard let colon = value.firstIndex(of: ":") else { return authorizationRedaction }
    return "\(value[..<colon]):\(authorizationRedaction)"
}

private func redactingURL(_ argument: String) -> String {
    guard let scheme = argument.range(of: "://") else { return argument }
    var result = argument
    let authorityStart = scheme.upperBound
    let authorityEnd = result[authorityStart...].firstIndex(where: { "/?#".contains($0) }) ?? result.endIndex
    let authority = result[authorityStart..<authorityEnd]

    if let at = authority.lastIndex(of: "@"),
       let colon = authority[..<at].firstIndex(of: ":")
    {
        result.replaceSubrange(result.index(after: colon)..<at, with: authorizationRedaction)
    }

    guard let question = result.firstIndex(of: "?") else { return result }
    let queryStart = result.index(after: question)
    let queryEnd = result[queryStart...].firstIndex(of: "#") ?? result.endIndex
    let query = result[queryStart..<queryEnd]
    let redactedQuery = query.split(separator: "&", omittingEmptySubsequences: false).map { item -> String in
        guard let equals = item.firstIndex(of: "=") else { return String(item) }
        let encodedName = String(item[..<equals])
        let name = encodedName.removingPercentEncoding ?? encodedName
        guard isSensitiveName(name) else { return String(item) }
        return "\(item[..<equals])=\(authorizationRedaction)"
    }.joined(separator: "&")
    result.replaceSubrange(queryStart..<queryEnd, with: redactedQuery)
    return result
}

private func isRecognizableCredential(_ value: String) -> Bool {
    let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
    if trimmed.contains("-----BEGIN"), trimmed.contains("PRIVATE KEY-----") {
        return true
    }

    let token = trimmed.lowercased().hasPrefix("bearer ")
        ? String(trimmed.dropFirst("bearer ".count))
        : trimmed
    let prefixLengths: [(String, Int)] = [
        ("github_pat_", 12),
        ("ghp_", 16), ("gho_", 16), ("ghu_", 16), ("ghs_", 16), ("ghr_", 16),
        ("sk_live_", 12), ("sk_test_", 12), ("rk_live_", 12), ("rk_test_", 12),
        ("xoxa-", 10), ("xoxb-", 10), ("xoxp-", 10), ("xoxr-", 10), ("xoxs-", 10),
    ]
    if prefixLengths.contains(where: { token.hasPrefix($0.0) && token.count >= $0.0.count + $0.1 }) {
        return true
    }

    let segments = token.split(separator: ".", omittingEmptySubsequences: false)
    return token.count >= 32
        && segments.count == 3
        && segments[0].hasPrefix("eyJ")
        && segments.allSatisfy { segment in
            !segment.isEmpty && segment.unicodeScalars.allSatisfy {
                CharacterSet.alphanumerics.union(CharacterSet(charactersIn: "-_")).contains($0)
            }
        }
}
