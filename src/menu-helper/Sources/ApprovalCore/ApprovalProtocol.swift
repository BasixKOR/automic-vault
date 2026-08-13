import CryptoKit
import Foundation

public enum ApprovalProtocolError: Error, Equatable {
    case invalidRootKey
    case invalidEnvelope
    case unsupportedVersion
    case invalidRequest
    case mismatchedResponse
}

public struct ApprovalRelayAddress: Equatable, Sendable {
    public let room: String
    public let credential: String

    public init(room: String, credential: String) {
        self.room = room
        self.credential = credential
    }
}

public struct ApprovalCiphertext: Codable, Equatable, Sendable {
    public static let currentVersion: UInt16 = 1

    public let version: UInt16
    public let combined: Data

    public init(version: UInt16 = currentVersion, combined: Data) {
        self.version = version
        self.combined = combined
    }
}

public struct ApprovalCrypto: Sendable {
    public static let rootKeyByteCount = 32

    private let rootKey: SymmetricKey

    public init(rootKeyData: Data) throws {
        guard rootKeyData.count == Self.rootKeyByteCount else {
            throw ApprovalProtocolError.invalidRootKey
        }
        rootKey = SymmetricKey(data: rootKeyData)
    }

    public var address: ApprovalRelayAddress {
        ApprovalRelayAddress(
            room: identifier(label: "room"),
            credential: identifier(label: "credential")
        )
    }

    public func seal(_ plaintext: Data, purpose: String) throws -> ApprovalCiphertext {
        let box = try AES.GCM.seal(
            plaintext,
            using: key(purpose: purpose),
            authenticating: authenticatedData(purpose: purpose)
        )
        guard let combined = box.combined else { throw ApprovalProtocolError.invalidEnvelope }
        return ApprovalCiphertext(combined: combined)
    }

    public func open(_ envelope: ApprovalCiphertext, purpose: String) throws -> Data {
        guard envelope.version == ApprovalCiphertext.currentVersion else {
            throw ApprovalProtocolError.unsupportedVersion
        }
        return try AES.GCM.open(
            AES.GCM.SealedBox(combined: envelope.combined),
            using: key(purpose: purpose),
            authenticating: authenticatedData(purpose: purpose)
        )
    }

    public func registrationProof(deviceID: String) -> String {
        identifier(label: "registration:\(deviceID)")
    }

    private func key(purpose: String) -> SymmetricKey {
        HKDF<SHA256>.deriveKey(
            inputKeyMaterial: rootKey,
            salt: Data("automic-vault-approval-v1".utf8),
            info: Data(purpose.utf8),
            outputByteCount: Self.rootKeyByteCount
        )
    }

    private func authenticatedData(purpose: String) -> Data {
        Data("automic-vault-approval:\(ApprovalCiphertext.currentVersion):\(purpose)".utf8)
    }

    private func identifier(label: String) -> String {
        let mac = HMAC<SHA256>.authenticationCode(
            for: Data("automic-vault-approval-address:\(label):v1".utf8),
            using: rootKey
        )
        return Data(mac).base64URLEncodedString()
    }
}

public enum ApprovalRisk: String, Codable, CaseIterable, Sendable {
    case routine
    case unknown
    case secretDisclosure
    case unconstrainedSecretApplication
    case securityWarning

    public var requiresFullReview: Bool { self != .routine }
}

public struct ApprovalDetailSection: Codable, Equatable, Sendable {
    public struct Row: Codable, Equatable, Sendable {
        public let label: String
        public let value: String

        public init(label: String, value: String) {
            self.label = label
            self.value = value
        }
    }

    public let title: String
    public let rows: [Row]

    public init(title: String, rows: [Row]) {
        self.title = title
        self.rows = rows
    }
}

public struct PhoneApprovalRequest: Codable, Equatable, Identifiable, Sendable {
    public static let maximumEncodedBytes = 256 * 1024

    public let version: UInt16
    public let id: UUID
    public let createdAtMilliseconds: UInt64
    public let macName: String
    public let launcher: String
    public let tool: String
    public let command: String
    public let cwd: String
    public let secretNames: [String]
    public let reason: String
    public let risks: [ApprovalRisk]
    public let details: [ApprovalDetailSection]

    public init(
        version: UInt16 = 1,
        id: UUID = UUID(),
        createdAtMilliseconds: UInt64 = UInt64(Date().timeIntervalSince1970 * 1_000),
        macName: String,
        launcher: String,
        tool: String,
        command: String,
        cwd: String,
        secretNames: [String],
        reason: String,
        risks: [ApprovalRisk],
        details: [ApprovalDetailSection]
    ) throws {
        guard version == 1,
              !macName.isEmpty,
              !launcher.isEmpty,
              !tool.isEmpty,
              !command.isEmpty,
              !reason.isEmpty,
              !secretNames.contains(where: \.isEmpty),
              !risks.isEmpty else {
            throw ApprovalProtocolError.invalidRequest
        }
        self.version = version
        self.id = id
        self.createdAtMilliseconds = createdAtMilliseconds
        self.macName = macName
        self.launcher = launcher
        self.tool = tool
        self.command = command
        self.cwd = cwd
        self.secretNames = secretNames
        self.reason = reason
        self.risks = Array(Set(risks)).sorted { $0.rawValue < $1.rawValue }
        self.details = details
        guard try canonicalData().count <= Self.maximumEncodedBytes else {
            throw ApprovalProtocolError.invalidRequest
        }
    }

    public var requiresFullReview: Bool { risks.contains(where: \.requiresFullReview) }

    public func canonicalData() throws -> Data {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
        return try encoder.encode(self)
    }

    public func digest() throws -> Data {
        Data(SHA256.hash(data: try canonicalData()))
    }
}

public enum PhoneApprovalOutcome: String, Codable, Sendable {
    case approved
    case denied
}

public struct PhoneApprovalResponse: Codable, Equatable, Sendable {
    public let version: UInt16
    public let requestID: UUID
    public let requestDigest: Data
    public let outcome: PhoneApprovalOutcome
    public let deviceID: String
    public let decidedAtMilliseconds: UInt64

    public init(
        request: PhoneApprovalRequest,
        outcome: PhoneApprovalOutcome,
        deviceID: String,
        decidedAtMilliseconds: UInt64 = UInt64(Date().timeIntervalSince1970 * 1_000)
    ) throws {
        guard !deviceID.isEmpty else { throw ApprovalProtocolError.invalidRequest }
        version = 1
        requestID = request.id
        requestDigest = try request.digest()
        self.outcome = outcome
        self.deviceID = deviceID
        self.decidedAtMilliseconds = decidedAtMilliseconds
    }

    public init(
        requestID: UUID,
        requestDigest: Data,
        outcome: PhoneApprovalOutcome,
        deviceID: String,
        decidedAtMilliseconds: UInt64 = UInt64(Date().timeIntervalSince1970 * 1_000)
    ) throws {
        guard requestDigest.count == SHA256.byteCount, !deviceID.isEmpty else {
            throw ApprovalProtocolError.invalidRequest
        }
        version = 1
        self.requestID = requestID
        self.requestDigest = requestDigest
        self.outcome = outcome
        self.deviceID = deviceID
        self.decidedAtMilliseconds = decidedAtMilliseconds
    }

    public func validate(for request: PhoneApprovalRequest) throws {
        guard version == 1,
              requestID == request.id,
              requestDigest == (try request.digest()) else {
            throw ApprovalProtocolError.mismatchedResponse
        }
    }
}

public struct PhoneApprovalTicket: Codable, Equatable, Sendable {
    public let version: UInt16
    public let requestID: UUID
    public let requestDigest: Data
    public let macName: String
    public let launcher: String
    public let tool: String
    public let command: String
    public let reason: String
    public let requiresFullReview: Bool

    public init(request: PhoneApprovalRequest) throws {
        version = 1
        requestID = request.id
        requestDigest = try request.digest()
        macName = request.macName
        launcher = request.launcher
        tool = request.tool
        command = request.command
        reason = request.reason
        requiresFullReview = request.requiresFullReview
    }
}

public enum ApprovalWireMessage: Codable, Equatable, Sendable {
    case request(PhoneApprovalRequest)
    case response(PhoneApprovalResponse)
    case cancel(UUID)

    private enum CodingKeys: String, CodingKey { case kind, request, response, requestID }
    private enum Kind: String, Codable { case request, response, cancel }

    public init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        switch try values.decode(Kind.self, forKey: .kind) {
        case .request: self = .request(try values.decode(PhoneApprovalRequest.self, forKey: .request))
        case .response: self = .response(try values.decode(PhoneApprovalResponse.self, forKey: .response))
        case .cancel: self = .cancel(try values.decode(UUID.self, forKey: .requestID))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var values = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .request(let request):
            try values.encode(Kind.request, forKey: .kind)
            try values.encode(request, forKey: .request)
        case .response(let response):
            try values.encode(Kind.response, forKey: .kind)
            try values.encode(response, forKey: .response)
        case .cancel(let requestID):
            try values.encode(Kind.cancel, forKey: .kind)
            try values.encode(requestID, forKey: .requestID)
        }
    }
}

private extension Data {
    func base64URLEncodedString() -> String {
        base64EncodedString()
            .replacingOccurrences(of: "+", with: "-")
            .replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: "=", with: "")
    }
}
