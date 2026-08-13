import CryptoKit
import Darwin
import Foundation

public enum AWSCredentialError: Error, Equatable, LocalizedError, Sendable {
    case invalidConfig(String)
    case unsupportedProfile(String)
    case unsupportedRuntime(String)
    case invalidResponse(String)

    public var errorDescription: String? {
        switch self {
        case .invalidConfig(let detail): "Invalid AWS config: \(detail)"
        case .unsupportedProfile(let detail): "Unsupported AWS profile: \(detail)"
        case .unsupportedRuntime(let detail): "Unsupported AWS runtime: \(detail)"
        case .invalidResponse(let detail): "Invalid AWS STS response: \(detail)"
        }
    }
}

public struct AWSProfile: Equatable, Sendable {
    public let name: String
    public let sourceProfile: String?
    public let roleARN: String?
    public let mfaSerial: String?
    public let region: String?
}

public struct AWSProfileChain: Equatable, Sendable {
    public let profiles: [AWSProfile]
    public var selected: AWSProfile { profiles.last! }
    public var region: String { profiles.reversed().compactMap(\.region).first ?? "us-east-1" }

    public static func parse(_ config: String, selectedProfile: String) throws -> Self {
        var sections: [String: [String: String]] = [:]
        var section: String?
        for (index, rawLine) in config.split(separator: "\n", omittingEmptySubsequences: false).enumerated() {
            let line = rawLine.trimmingCharacters(in: .whitespaces)
            if line.isEmpty || line.hasPrefix("#") || line.hasPrefix(";") { continue }
            if line.hasPrefix("[") && line.hasSuffix("]") {
                let rawName = line.dropFirst().dropLast().trimmingCharacters(in: .whitespaces)
                section = rawName.hasPrefix("profile ") ? String(rawName.dropFirst(8)) : rawName
                guard let section, !section.isEmpty, sections[section] == nil else {
                    throw AWSCredentialError.invalidConfig("duplicate or empty section on line \(index + 1)")
                }
                sections[section] = [:]
                continue
            }
            guard let section, let separator = line.firstIndex(of: "=") else { continue }
            let key = line[..<separator].trimmingCharacters(in: .whitespaces).lowercased()
            let value = line[line.index(after: separator)...].trimmingCharacters(in: .whitespaces)
            guard !key.isEmpty, !value.isEmpty else { continue }
            if sections[section]![key] != nil {
                throw AWSCredentialError.invalidConfig("duplicate \(key) in profile \(section)")
            }
            sections[section]![key] = value
        }

        var chain: [AWSProfile] = []
        var name = selectedProfile
        var seen: Set<String> = []
        while true {
            guard seen.insert(name).inserted else {
                throw AWSCredentialError.invalidConfig("source_profile cycle at \(name)")
            }
            let values = sections[name] ?? [:]
            let unsupported = [
                "credential_process", "credential_source", "mfa_process", "sso_session", "sso_start_url",
                "sso_region", "sso_account_id", "sso_role_name", "web_identity_token_file",
                "aws_access_key_id", "aws_secret_access_key", "aws_session_token",
            ].first { values[$0] != nil }
            if let unsupported {
                throw AWSCredentialError.unsupportedProfile("\(name) uses \(unsupported)")
            }
            let profile = AWSProfile(
                name: name,
                sourceProfile: values["source_profile"],
                roleARN: values["role_arn"],
                mfaSerial: values["mfa_serial"],
                region: values["region"]
            )
            if profile.roleARN != nil && profile.sourceProfile == nil {
                throw AWSCredentialError.unsupportedProfile("\(name) has role_arn without source_profile")
            }
            if profile.roleARN == nil && profile.sourceProfile != nil {
                throw AWSCredentialError.unsupportedProfile("\(name) has source_profile without role_arn")
            }
            chain.append(profile)
            guard let source = profile.sourceProfile else { break }
            name = source
        }
        chain.reverse()
        guard chain.first?.name == "default" else {
            throw AWSCredentialError.unsupportedProfile("the credential chain must end at default")
        }
        guard selectedProfile == "default" || chain.last?.roleARN != nil else {
            throw AWSCredentialError.unsupportedProfile("named profiles must assume a role from default")
        }
        return Self(profiles: chain)
    }
}

public struct AWSCredentials: Equatable, Sendable {
    public let accessKeyID: String
    public let secretAccessKey: String
    public let sessionToken: String?
    public let expiration: Date?

    public init(accessKeyID: String, secretAccessKey: String, sessionToken: String? = nil, expiration: Date? = nil) {
        self.accessKeyID = accessKeyID
        self.secretAccessKey = secretAccessKey
        self.sessionToken = sessionToken
        self.expiration = expiration
    }

    public func credentialProcessJSON() throws -> Data {
        var value: [String: Any] = [
            "Version": 1,
            "AccessKeyId": accessKeyID,
            "SecretAccessKey": secretAccessKey,
        ]
        if let sessionToken { value["SessionToken"] = sessionToken }
        if let expiration { value["Expiration"] = ISO8601DateFormatter().string(from: expiration) }
        return try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
    }
}

public struct AWSSignedRequest: Equatable, Sendable {
    public let url: URL
    public let headers: [String: String]
    public let body: Data
}

public enum AWSRuntimeGeneration: String, Equatable, Sendable {
    case homebrewV1 = "homebrew-v1"
    case officialV2 = "official-v2"

    public var target: String {
        switch self {
        case .homebrewV1: "/opt/homebrew/bin/aws"
        case .officialV2: "/opt/av/aws/current/aws"
        }
    }

    public var stub: String {
        switch self {
        case .homebrewV1: "#!/usr/local/bin/av aws\n"
        case .officialV2: "#!/usr/local/bin/av aws-official\n"
        }
    }
}

public func negotiatedAWSHelperProtocolVersion(requested: UInt64) -> Int? {
    switch requested {
    case 0: 1 // v1 clients predate negotiation.
    case 2: 2
    default: nil
    }
}

public func awsGenerationMatchesInstalledStub(
    _ generation: AWSRuntimeGeneration,
    target: String,
    stub: String
) -> Bool {
    target == generation.target && stub == generation.stub
}

public func readProtectedAWSStub(
    path: String,
    requiredUID: uid_t = 0,
    requiredGID: gid_t = 0
) -> String? {
    let descriptor = open(path, O_RDONLY | O_NOFOLLOW | O_CLOEXEC)
    guard descriptor >= 0 else { return nil }
    defer { close(descriptor) }
    var info = stat()
    guard fstat(descriptor, &info) == 0,
          info.st_mode & S_IFMT == S_IFREG,
          info.st_uid == requiredUID,
          info.st_gid == requiredGID,
          info.st_mode & 0o7777 == 0o755,
          info.st_size > 0,
          info.st_size <= 128
    else { return nil }
    var bytes = [UInt8](repeating: 0, count: Int(info.st_size) + 1)
    let count = Darwin.read(descriptor, &bytes, bytes.count)
    guard count == info.st_size else { return nil }
    return String(bytes: bytes.prefix(Int(count)), encoding: .utf8)
}

public func awsSTSRequest(
    region: String,
    parameters: [String: String],
    credentials: AWSCredentials,
    date: Date = Date()
) throws -> AWSSignedRequest {
    guard !region.isEmpty,
          region.utf8.allSatisfy({ $0 == 45 || 48...57 ~= $0 || 97...122 ~= $0 }),
          let url = URL(string: "https://sts.\(region).amazonaws.com/"),
          url.host == "sts.\(region).amazonaws.com"
    else {
        throw AWSCredentialError.invalidConfig("invalid region")
    }
    let body = parameters.sorted { $0.key < $1.key }
        .map { "\(awsPercentEncode($0.key))=\(awsPercentEncode($0.value))" }
        .joined(separator: "&")
    let host = url.host!
    let timestamp = awsTimestamp(date)
    var headers = [
        "content-type": "application/x-www-form-urlencoded; charset=utf-8",
        "host": host,
        "x-amz-date": timestamp,
    ]
    if let token = credentials.sessionToken { headers["x-amz-security-token"] = token }
    let signedHeaderNames = headers.keys.sorted().joined(separator: ";")
    let canonicalHeaders = headers.sorted { $0.key < $1.key }
        .map { "\($0.key):\($0.value.trimmingCharacters(in: .whitespacesAndNewlines))\n" }
        .joined()
    let payloadHash = sha256Hex(Data(body.utf8))
    let canonicalRequest = "POST\n/\n\n\(canonicalHeaders)\n\(signedHeaderNames)\n\(payloadHash)"
    let day = String(timestamp.prefix(8))
    let scope = "\(day)/\(region)/sts/aws4_request"
    let stringToSign = "AWS4-HMAC-SHA256\n\(timestamp)\n\(scope)\n\(sha256Hex(Data(canonicalRequest.utf8)))"
    let dateKey = hmac(Data(("AWS4" + credentials.secretAccessKey).utf8), day)
    let regionKey = hmac(dateKey, region)
    let serviceKey = hmac(regionKey, "sts")
    let signingKey = hmac(serviceKey, "aws4_request")
    let signature = hmac(signingKey, stringToSign).map { String(format: "%02x", $0) }.joined()
    headers["authorization"] = "AWS4-HMAC-SHA256 Credential=\(credentials.accessKeyID)/\(scope), SignedHeaders=\(signedHeaderNames), Signature=\(signature)"
    return AWSSignedRequest(url: url, headers: headers, body: Data(body.utf8))
}

public func parseAWSTSCredentials(_ data: Data) throws -> AWSCredentials {
    final class Parser: NSObject, XMLParserDelegate {
        var current = ""
        var values: [String: String] = [:]
        func parser(_ parser: XMLParser, didStartElement elementName: String, namespaceURI: String?, qualifiedName: String?, attributes attributeDict: [String: String] = [:]) { current = elementName }
        func parser(_ parser: XMLParser, foundCharacters string: String) { values[current, default: ""] += string }
        func parser(_ parser: XMLParser, didEndElement elementName: String, namespaceURI: String?, qualifiedName: String?) { current = "" }
    }
    let delegate = Parser()
    let parser = XMLParser(data: data)
    parser.delegate = delegate
    guard parser.parse() else {
        throw AWSCredentialError.invalidResponse(parser.parserError?.localizedDescription ?? "malformed XML")
    }
    if let code = delegate.values["Code"], let message = delegate.values["Message"] {
        throw AWSCredentialError.invalidResponse("\(code): \(message)")
    }
    guard let access = delegate.values["AccessKeyId"], !access.isEmpty,
          let secret = delegate.values["SecretAccessKey"], !secret.isEmpty,
          let token = delegate.values["SessionToken"], !token.isEmpty,
          let expirationText = delegate.values["Expiration"],
          let expiration = awsExpirationDate(expirationText.trimmingCharacters(in: .whitespacesAndNewlines))
    else { throw AWSCredentialError.invalidResponse("credentials are incomplete") }
    return AWSCredentials(
        accessKeyID: access.trimmingCharacters(in: .whitespacesAndNewlines),
        secretAccessKey: secret.trimmingCharacters(in: .whitespacesAndNewlines),
        sessionToken: token.trimmingCharacters(in: .whitespacesAndNewlines),
        expiration: expiration
    )
}

public func awsRuntimeMatches(
    generation: AWSRuntimeGeneration,
    interpreter: String,
    processPath: String,
    processArguments: [String],
    target: String,
    approvedArguments: [String]
) -> Bool {
    if target != generation.target { return false }
    if generation == .officialV2 {
        let expected = URL(fileURLWithPath: target).resolvingSymlinksInPath().path
        let live = URL(fileURLWithPath: processPath).resolvingSymlinksInPath().path
        let argumentsMatch = processArguments == [target] + approvedArguments
            || processArguments == [processPath] + approvedArguments
        return expected == live && argumentsMatch
    }
    let resolved = URL(fileURLWithPath: interpreter).resolvingSymlinksInPath().path
    let executableMatches: Bool
    if resolved == processPath {
        executableMatches = true
    } else if let marker = resolved.range(of: "/bin/", options: .backwards) {
        executableMatches = resolved[..<marker.lowerBound]
            + "/Resources/Python.app/Contents/MacOS/Python" == processPath
    } else {
        executableMatches = false
    }
    return executableMatches && processArguments == [processPath, target] + approvedArguments
}

public func awsInterpreter(fromShebang shebang: String) throws -> String {
    let words = shebang.dropFirst(2).split(whereSeparator: \.isWhitespace)
    guard shebang.hasPrefix("#!/"), words.count == 1, let interpreter = words.first,
          interpreter.hasPrefix("/")
    else {
        throw AWSCredentialError.unsupportedRuntime(
            "the AWS CLI shebang must contain one absolute interpreter without arguments"
        )
    }
    return String(interpreter)
}

private func awsExpirationDate(_ value: String) -> Date? {
    let formatter = ISO8601DateFormatter()
    if let date = formatter.date(from: value) { return date }
    formatter.formatOptions.insert(.withFractionalSeconds)
    return formatter.date(from: value)
}

private func awsPercentEncode(_ value: String) -> String {
    value.utf8.map { byte in
        switch byte {
        case 65...90, 97...122, 48...57, 45, 46, 95, 126: String(UnicodeScalar(byte))
        default: String(format: "%%%02X", byte)
        }
    }.joined()
}

private func awsTimestamp(_ date: Date) -> String {
    let formatter = DateFormatter()
    formatter.locale = Locale(identifier: "en_US_POSIX")
    formatter.timeZone = TimeZone(secondsFromGMT: 0)
    formatter.dateFormat = "yyyyMMdd'T'HHmmss'Z'"
    return formatter.string(from: date)
}

private func sha256Hex(_ data: Data) -> String {
    SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
}

private func hmac(_ key: Data, _ value: String) -> Data {
    Data(HMAC<SHA256>.authenticationCode(for: Data(value.utf8), using: SymmetricKey(data: key)))
}
