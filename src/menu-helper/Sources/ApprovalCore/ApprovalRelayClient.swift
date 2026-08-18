import Foundation

public enum ApprovalRelayClientError: Error, Equatable {
    case invalidEndpoint
    case disconnected
    case unexpectedMessage
    case invalidResponse(Int)
    case notificationTooLarge
}

public struct ApprovalRegistrationStatus: Codable, Equatable, Sendable {
    public let count: Int
    public let mostRecentMilliseconds: UInt64?
}

public struct ApprovalDeviceRegistration: Codable, Equatable, Sendable {
    public enum Environment: String, Codable, Sendable { case sandbox, production }

    public let token: String
    public let environment: Environment
    public let proof: String

    public init(token: String, environment: Environment, proof: String) {
        self.token = token
        self.environment = environment
        self.proof = proof
    }
}

private struct ApprovalRelayPublication: Codable {
    let message: ApprovalCiphertext
    let notification: ApprovalCiphertext
}

public actor ApprovalRelayClient {
    public static let maximumNotificationBytes = 2_500

    private let endpoint: URL
    private let crypto: ApprovalCrypto
    private let address: ApprovalRelayAddress
    private let session: URLSession
    private var socket: URLSessionWebSocketTask?
    private var peerID: String?

    public init(endpoint: URL, rootKeyData: Data, session: URLSession = .shared) throws {
        self.endpoint = endpoint
        crypto = try ApprovalCrypto(rootKeyData: rootKeyData)
        address = crypto.address
        self.session = session
    }

    public func connect(peerID: String) throws {
        guard socket == nil else { return }
        guard var components = URLComponents(url: endpoint, resolvingAgainstBaseURL: false) else {
            throw ApprovalRelayClientError.invalidEndpoint
        }
        components.scheme = endpoint.scheme == "http" ? "ws" : "wss"
        components.path = endpoint.path + "/v1/connect/\(address.room)/\(peerID)"
        guard let url = components.url else { throw ApprovalRelayClientError.invalidEndpoint }
        var request = authorizedRequest(url: url)
        request.timeoutInterval = 60
        let socket = session.webSocketTask(with: request)
        self.socket = socket
        self.peerID = peerID
        socket.resume()
    }

    public func receive() async throws -> ApprovalWireMessage {
        guard let socket else { throw ApprovalRelayClientError.disconnected }
        guard case .data(let data) = try await socket.receive() else {
            throw ApprovalRelayClientError.unexpectedMessage
        }
        let envelope = try JSONDecoder().decode(ApprovalCiphertext.self, from: data)
        let plaintext = try crypto.open(envelope, purpose: "transport")
        return try JSONDecoder().decode(ApprovalWireMessage.self, from: plaintext)
    }

    public func publish(_ request: PhoneApprovalRequest) async throws {
        guard let peerID else { throw ApprovalRelayClientError.disconnected }
        let messageData = try JSONEncoder().encode(ApprovalWireMessage.request(request))
        let ticketData = try JSONEncoder().encode(PhoneApprovalTicket(request: request))
        let notification = try crypto.seal(ticketData, purpose: "notification")
        let notificationData = try JSONEncoder().encode(notification)
        guard notificationData.count <= Self.maximumNotificationBytes else {
            throw ApprovalRelayClientError.notificationTooLarge
        }
        let publication = ApprovalRelayPublication(
            message: try crypto.seal(messageData, purpose: "transport"),
            notification: notification
        )
        try await post(
            publication,
            path: ["v1", "request", address.room, peerID],
            accepted: 204
        )
    }

    public func send(_ message: ApprovalWireMessage) async throws {
        guard let peerID else { throw ApprovalRelayClientError.disconnected }
        let plaintext = try JSONEncoder().encode(message)
        let envelope = try crypto.seal(plaintext, purpose: "transport")
        try await post(envelope, path: ["v1", "send", address.room, peerID], accepted: 204)
    }

    public func register(deviceID: String, token: Data, environment: ApprovalDeviceRegistration.Environment) async throws {
        let registration = ApprovalDeviceRegistration(
            token: token.map { String(format: "%02x", $0) }.joined(),
            environment: environment,
            proof: crypto.registrationProof(deviceID: deviceID)
        )
        try await put(registration, path: ["v1", "register", address.room, deviceID], accepted: 204)
    }

    public func registrationStatus() async throws -> ApprovalRegistrationStatus {
        let url = endpoint.appending(path: ["v1", "registrations", address.room])
        let (data, response) = try await session.data(for: authorizedRequest(url: url))
        try validate(response, accepted: 200)
        return try JSONDecoder().decode(ApprovalRegistrationStatus.self, from: data)
    }

    public func revokeRoom() async throws {
        var request = authorizedRequest(url: endpoint.appending(path: ["v1", "room", address.room]))
        request.httpMethod = "DELETE"
        let (_, response) = try await session.data(for: request)
        try validate(response, accepted: 204)
    }

    public func openNotification(_ data: Data) throws -> PhoneApprovalTicket {
        let envelope = try JSONDecoder().decode(ApprovalCiphertext.self, from: data)
        return try JSONDecoder().decode(
            PhoneApprovalTicket.self,
            from: crypto.open(envelope, purpose: "notification")
        )
    }

    public func disconnect() {
        socket?.cancel(with: .goingAway, reason: nil)
        socket = nil
        peerID = nil
    }

    private func post<T: Encodable>(_ value: T, path: [String], accepted: Int) async throws {
        var request = authorizedRequest(url: endpoint.appending(path: path))
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONEncoder().encode(value)
        let (_, response) = try await session.data(for: request)
        try validate(response, accepted: accepted)
    }

    private func put<T: Encodable>(_ value: T, path: [String], accepted: Int) async throws {
        var request = authorizedRequest(url: endpoint.appending(path: path))
        request.httpMethod = "PUT"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONEncoder().encode(value)
        let (_, response) = try await session.data(for: request)
        try validate(response, accepted: accepted)
    }

    private func authorizedRequest(url: URL) -> URLRequest {
        var request = URLRequest(url: url)
        request.setValue("Bearer \(address.credential)", forHTTPHeaderField: "Authorization")
        return request
    }

    private func validate(_ response: URLResponse, accepted: Int) throws {
        guard let response = response as? HTTPURLResponse, response.statusCode == accepted else {
            throw ApprovalRelayClientError.invalidResponse((response as? HTTPURLResponse)?.statusCode ?? -1)
        }
    }
}

private extension URL {
    func appending(path components: [String]) -> URL {
        components.reduce(self) { $0.appendingPathComponent($1) }
    }
}
