import LocalAuthentication
import Observation
import SwiftUI
@preconcurrency import UserNotifications

@main
struct AutomicVaultApprovalApp: App {
    @UIApplicationDelegateAdaptor(ApprovalAppDelegate.self) private var delegate

    var body: some Scene {
        WindowGroup {
            ApprovalRootView(model: .shared)
                .task { await ApprovalModel.shared.start() }
        }
    }
}

final class ApprovalAppDelegate: NSObject, UIApplicationDelegate, @preconcurrency UNUserNotificationCenterDelegate {
    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil
    ) -> Bool {
        let center = UNUserNotificationCenter.current()
        center.delegate = self
        Self.registerNotificationCategories()
        return true
    }

    func application(_ application: UIApplication, didRegisterForRemoteNotificationsWithDeviceToken token: Data) {
        Task { await ApprovalModel.shared.register(deviceToken: token) }
    }

    func application(_ application: UIApplication, didFailToRegisterForRemoteNotificationsWithError error: Error) {
        Task { ApprovalModel.shared.registrationFailed(error) }
    }

    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification
    ) async -> UNNotificationPresentationOptions {
        [.banner, .list, .sound]
    }

    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse,
        withCompletionHandler completionHandler: @escaping () -> Void
    ) {
        Task {
            await ApprovalModel.shared.handleNotificationResponse(response)
            completionHandler()
        }
    }

    static func registerNotificationCategories() {
        let deny = UNNotificationAction(identifier: "AV_DENY", title: "Deny")
        let approvalOptions: UNNotificationActionOptions = ApprovalModel.biometricProtectionEnabled
            ? [.foreground, .authenticationRequired]
            : [.authenticationRequired]
        let approve = UNNotificationAction(
            identifier: "AV_APPROVE",
            title: "Approve Once",
            options: approvalOptions
        )
        let review = UNNotificationAction(
            identifier: "AV_REVIEW",
            title: "Review",
            options: [.foreground, .authenticationRequired]
        )
        UNUserNotificationCenter.current().setNotificationCategories([
            UNNotificationCategory(identifier: "AV_ROUTINE", actions: [deny, approve], intentIdentifiers: []),
            UNNotificationCategory(identifier: "AV_REVIEW", actions: [deny, review], intentIdentifiers: []),
        ])
    }
}

@MainActor @Observable
final class ApprovalModel {
    static let shared = ApprovalModel()
    static let biometricDefaultsKey = "approvalBiometricProtection"

    enum ConnectionState: Equatable {
        case setup
        case connecting
        case connected
        case unavailable(String)
    }

    static var biometricProtectionEnabled: Bool {
        UserDefaults.standard.bool(forKey: biometricDefaultsKey)
    }

    private(set) var pending: [PhoneApprovalRequest] = []
    private(set) var state: ConnectionState = .setup
    var errorMessage: String?
    var biometricProtectionEnabled = UserDefaults.standard.bool(forKey: biometricDefaultsKey) {
        didSet {
            UserDefaults.standard.set(biometricProtectionEnabled, forKey: Self.biometricDefaultsKey)
            ApprovalAppDelegate.registerNotificationCategories()
        }
    }

    private let endpoint = URL(string: Bundle.main.object(forInfoDictionaryKey: "ApprovalRelayURL") as? String ?? "")
    private let deviceID: String
    private var relay: ApprovalRelayClient?
    private var deviceToken: Data?
    private var receiveTask: Task<Void, Never>?
    private var started = false

    private init() {
        if let existing = UserDefaults.standard.string(forKey: "approvalDeviceID") {
            deviceID = existing
        } else {
            let value = UUID().uuidString.lowercased()
            UserDefaults.standard.set(value, forKey: "approvalDeviceID")
            deviceID = value
        }
    }

    func start() async {
        guard !started else { return }
        started = true
        let settings = await UNUserNotificationCenter.current().notificationSettings()
        guard settings.authorizationStatus == .authorized || settings.authorizationStatus == .provisional else {
            state = .setup
            return
        }
        UIApplication.shared.registerForRemoteNotifications()
        await connect()
    }

    func enable() async {
        guard ICloudApprovalRootKey.hasActiveICloudAccount() else {
            state = .unavailable("Sign in to iCloud and enable iCloud Keychain to use iPhone Approval.")
            return
        }
        do {
            guard try await UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .badge, .sound]) else {
                state = .unavailable("Notifications are required so Approval requests can reach this iPhone.")
                return
            }
            UIApplication.shared.registerForRemoteNotifications()
            await connect()
        } catch {
            state = .unavailable(error.localizedDescription)
        }
    }

    func register(deviceToken: Data) async {
        self.deviceToken = deviceToken
        await connect()
        await registerIfPossible()
    }

    func registrationFailed(_ error: Error) {
        state = .unavailable("Push registration failed: \(error.localizedDescription)")
    }

    func approve(_ request: PhoneApprovalRequest) async {
        if biometricProtectionEnabled {
            guard await authenticateBiometrically() else { return }
        }
        await respond(to: request, outcome: .approved)
    }

    func deny(_ request: PhoneApprovalRequest) async {
        await respond(to: request, outcome: .denied)
    }

    func denyAll() async {
        for request in pending { await deny(request) }
    }

    func setBiometricProtection(_ enabled: Bool) async {
        guard enabled != biometricProtectionEnabled else { return }
        guard await authenticateBiometrically() else { return }
        biometricProtectionEnabled = enabled
    }

    func handleNotificationResponse(_ response: UNNotificationResponse) async {
        guard let ticket = await ticket(from: response.notification.request.content.userInfo) else { return }
        switch response.actionIdentifier {
        case "AV_DENY": await respond(to: ticket, outcome: .denied)
        case "AV_APPROVE" where !ticket.requiresFullReview:
            if biometricProtectionEnabled {
                guard await authenticateBiometrically() else { return }
            }
            await respond(to: ticket, outcome: .approved)
        default: break
        }
    }

    private func connect() async {
        guard receiveTask == nil, let endpoint else {
            if endpoint == nil { state = .unavailable("The Approval relay URL is invalid.") }
            return
        }
        do {
            state = .connecting
            let key = try ICloudApprovalRootKey().loadOrCreate()
            let relay = try ApprovalRelayClient(endpoint: endpoint, rootKeyData: key)
            try await relay.connect(peerID: "phone-\(deviceID)")
            self.relay = relay
            state = .connected
            await registerIfPossible()
            receiveTask = Task { [weak self] in await self?.receive(relay) }
        } catch {
            state = .unavailable(error.localizedDescription)
        }
    }

    private func registerIfPossible() async {
        guard let relay, let deviceToken else { return }
        do {
            #if DEBUG
            let environment = ApprovalDeviceRegistration.Environment.sandbox
            #else
            let environment = ApprovalDeviceRegistration.Environment.production
            #endif
            try await relay.register(deviceID: deviceID, token: deviceToken, environment: environment)
            state = .connected
        } catch {
            state = .unavailable("Could not register this iPhone: \(error.localizedDescription)")
        }
    }

    private func receive(_ relay: ApprovalRelayClient) async {
        while !Task.isCancelled {
            do {
                switch try await relay.receive() {
                case .request(let request):
                    if !pending.contains(where: { $0.id == request.id }) { pending.append(request) }
                case .response(let response):
                    pending.removeAll { $0.id == response.requestID }
                case .cancel(let requestID):
                    pending.removeAll { $0.id == requestID }
                }
            } catch {
                state = .unavailable("Relay disconnected. Reopen the app to reconnect.")
                receiveTask = nil
                return
            }
        }
    }

    private func respond(to request: PhoneApprovalRequest, outcome: PhoneApprovalOutcome) async {
        do {
            guard let relay else { throw ApprovalRelayClientError.disconnected }
            let response = try PhoneApprovalResponse(request: request, outcome: outcome, deviceID: deviceID)
            try await relay.send(.response(response))
            pending.removeAll { $0.id == request.id }
        } catch {
            errorMessage = "The response was not delivered. The request remains pending."
        }
    }

    private func respond(to ticket: PhoneApprovalTicket, outcome: PhoneApprovalOutcome) async {
        do {
            if relay == nil { await connect() }
            guard let relay else { throw ApprovalRelayClientError.disconnected }
            let response = try PhoneApprovalResponse(
                requestID: ticket.requestID,
                requestDigest: ticket.requestDigest,
                outcome: outcome,
                deviceID: deviceID
            )
            try await relay.send(.response(response))
            pending.removeAll { $0.id == ticket.requestID }
        } catch {
            errorMessage = "The response was not delivered. Open Automic Vault and try again."
        }
    }

    private func ticket(from userInfo: [AnyHashable: Any]) async -> PhoneApprovalTicket? {
        do {
            guard let value = userInfo["av"] else { return nil }
            let data = try JSONSerialization.data(withJSONObject: value)
            let envelope = try JSONDecoder().decode(ApprovalCiphertext.self, from: data)
            let key = try ICloudApprovalRootKey().loadOrCreate()
            let plaintext = try ApprovalCrypto(rootKeyData: key).open(envelope, purpose: "notification")
            return try JSONDecoder().decode(PhoneApprovalTicket.self, from: plaintext)
        } catch {
            errorMessage = "This Approval notification could not be authenticated."
            return nil
        }
    }

    private func authenticateBiometrically() async -> Bool {
        let context = LAContext()
        context.localizedFallbackTitle = ""
        var error: NSError?
        guard context.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: &error) else {
            errorMessage = "Face ID or Touch ID is unavailable. Turn off biometric protection deliberately in Settings to approve without it."
            return false
        }
        do {
            return try await context.evaluatePolicy(
                .deviceOwnerAuthenticationWithBiometrics,
                localizedReason: "Approve this Automic Vault request"
            )
        } catch {
            errorMessage = "Approval was not authenticated."
            return false
        }
    }
}
