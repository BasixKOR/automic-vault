import AppKit
import ApprovalCore
import Foundation
import LocalAuthentication

let phoneApprovalEnabledDefaultsKey = "phoneApprovalEnabled"
private let phoneApprovalMacIDDefaultsKey = "phoneApprovalMacID"
private let phoneApprovalRelayURL = URL(string: "https://approval-relay.automicvault.com")!

nonisolated func phoneApprovalIsEnabled() -> Bool {
    UserDefaults.standard.bool(forKey: phoneApprovalEnabledDefaultsKey)
}

enum PhoneApprovalResult {
    case approved
    case denied
    case canceled
}

enum PhoneApprovalSetupError: LocalizedError {
    case iCloudUnavailable
    case noRegisteredPhone
    case pendingLimit

    var errorDescription: String? {
        switch self {
        case .iCloudUnavailable:
            "Sign in to iCloud and enable iCloud Keychain before enabling iPhone Approval."
        case .noRegisteredPhone:
            "No iPhone has registered recently. Open Automic Vault on an iPhone using this iCloud account, allow notifications, then try again."
        case .pendingLimit:
            "This Mac already has 100 pending iPhone Approvals."
        }
    }
}

@MainActor
final class PhoneApprovalCoordinator {
    static let shared = PhoneApprovalCoordinator()

    var isEnabled: Bool { phoneApprovalIsEnabled() }
    var pendingCount: Int { pending.count }

    private struct Pending {
        let request: PhoneApprovalRequest
        let completion: (PhoneApprovalResult) -> Void
    }

    private let macID: String
    private var pending: [UUID: Pending] = [:]
    private var relay: ApprovalRelayClient?
    private var connectionTask: Task<Void, Never>?

    private init() {
        if let existing = UserDefaults.standard.string(forKey: phoneApprovalMacIDDefaultsKey) {
            macID = existing
        } else {
            let value = UUID().uuidString.lowercased()
            UserDefaults.standard.set(value, forKey: phoneApprovalMacIDDefaultsKey)
            macID = value
        }
    }

    func registrationStatus() async throws -> ApprovalRegistrationStatus {
        guard ICloudApprovalRootKey.hasActiveICloudAccount() else {
            throw PhoneApprovalSetupError.iCloudUnavailable
        }
        let key = try ICloudApprovalRootKey().loadOrCreate()
        return try await ApprovalRelayClient(
            endpoint: phoneApprovalRelayURL,
            rootKeyData: key
        ).registrationStatus()
    }

    func enable() async throws {
        guard try await registrationStatus().count > 0 else {
            throw PhoneApprovalSetupError.noRegisteredPhone
        }
        UserDefaults.standard.set(true, forKey: phoneApprovalEnabledDefaultsKey)
        startConnectionIfNeeded()
    }

    func submit(
        _ request: PhoneApprovalRequest,
        completion: @escaping (PhoneApprovalResult) -> Void
    ) throws {
        guard pending.count < 100 else { throw PhoneApprovalSetupError.pendingLimit }
        pending[request.id] = Pending(request: request, completion: completion)
        startConnectionIfNeeded()
        if let relay {
            Task { [weak self] in
                do { try await relay.publish(request) }
                catch { self?.restartConnection() }
            }
        }
    }

    func approveAuthorityChange(
        title: String,
        detail: String,
        completion: @escaping (Bool) -> Void
    ) {
        guard isEnabled else {
            completion(true)
            return
        }
        do {
            let request = try PhoneApprovalRequest(
                macName: Host.current().localizedName ?? ProcessInfo.processInfo.hostName,
                launcher: "Automic Vault",
                tool: "Settings",
                command: title,
                cwd: NSHomeDirectory(),
                secretNames: [],
                reason: detail,
                risks: [.securityWarning],
                details: [ApprovalDetailSection(
                    title: "Authority Change",
                    rows: [.init(label: "Change", value: title), .init(label: "Effect", value: detail)]
                )]
            )
            try submit(request) { result in completion(result == .approved) }
        } catch {
            completion(false)
        }
    }

    func requestDisable(completion: @escaping (Bool) -> Void) {
        approveAuthorityChange(
            title: "Disable iPhone Approval",
            detail: "Future human Approvals will return to this Mac. Existing requests will be canceled."
        ) { [weak self] approved in
            if approved { self?.disableAfterPhoneApproval() }
            completion(approved)
        }
    }

    func cancel(_ requestID: UUID) {
        guard let item = pending.removeValue(forKey: requestID) else { return }
        item.completion(.canceled)
        if let relay { Task { try? await relay.send(.cancel(requestID)) } }
    }

    func disableAfterPhoneApproval() {
        UserDefaults.standard.set(false, forKey: phoneApprovalEnabledDefaultsKey)
        stopConnection(cancelPending: true)
    }

    func recoverWithoutIPhone() async throws {
        let context = LAContext()
        guard try await context.evaluatePolicy(
            .deviceOwnerAuthentication,
            localizedReason: "Disable iPhone Approval and invalidate every enrolled device"
        ) else { return }
        let keyStore = ICloudApprovalRootKey()
        let oldKey = try keyStore.load()
        try await ApprovalRelayClient(endpoint: phoneApprovalRelayURL, rootKeyData: oldKey).revokeRoom()
        _ = try keyStore.rotate()
        UserDefaults.standard.set(false, forKey: phoneApprovalEnabledDefaultsKey)
        stopConnection(cancelPending: true)
    }

    private func startConnectionIfNeeded() {
        guard isEnabled, connectionTask == nil else { return }
        connectionTask = Task { [weak self] in
            guard let self else { return }
            var retrySeconds: UInt64 = 1
            while self.isEnabled && !Task.isCancelled {
                do {
                    let key = try ICloudApprovalRootKey().loadOrCreate()
                    let relay = try ApprovalRelayClient(endpoint: phoneApprovalRelayURL, rootKeyData: key)
                    try await relay.connect(peerID: "mac-\(self.macID)")
                    self.relay = relay
                    try await relay.send(.presence(try self.presence()))
                    for item in self.pending.values { try await relay.publish(item.request) }
                    retrySeconds = 1
                    while self.isEnabled && !Task.isCancelled {
                        try await self.handle(try await relay.receive())
                    }
                } catch {
                    self.relay = nil
                    try? await Task.sleep(for: .seconds(retrySeconds))
                    retrySeconds = min(retrySeconds * 2, 30)
                }
            }
            self.relay = nil
            self.connectionTask = nil
        }
    }

    private func handle(_ message: ApprovalWireMessage) async throws {
        switch message {
        case .response(let response):
            guard let item = pending[response.requestID] else { return }
            try response.validate(for: item.request)
            pending.removeValue(forKey: response.requestID)
            item.completion(response.outcome == .approved ? .approved : .denied)
        case .sync:
            guard let relay else { return }
            try await relay.send(.presence(try presence()))
            for item in pending.values { try await relay.publish(item.request) }
        case .request, .cancel, .presence:
            return
        }
    }

    private func presence() throws -> ApprovalMacPresence {
        try ApprovalMacPresence(
            macID: macID,
            macName: Host.current().localizedName ?? ProcessInfo.processInfo.hostName
        )
    }

    private func restartConnection() {
        connectionTask?.cancel()
        connectionTask = nil
        if let relay { Task { await relay.disconnect() } }
        relay = nil
        startConnectionIfNeeded()
    }

    private func stopConnection(cancelPending: Bool) {
        connectionTask?.cancel()
        connectionTask = nil
        if let relay { Task { await relay.disconnect() } }
        relay = nil
        if cancelPending {
            let items = Array(pending.values)
            pending.removeAll()
            items.forEach { $0.completion(.canceled) }
        }
    }
}
