import UserNotifications

final class NotificationService: UNNotificationServiceExtension {
    private var handler: ((UNNotificationContent) -> Void)?
    private var content: UNMutableNotificationContent?

    override func didReceive(
        _ request: UNNotificationRequest,
        withContentHandler contentHandler: @escaping (UNNotificationContent) -> Void
    ) {
        handler = contentHandler
        content = request.content.mutableCopy() as? UNMutableNotificationContent
        guard let content, let value = request.content.userInfo["av"] else {
            contentHandler(request.content)
            return
        }
        do {
            let data = try JSONSerialization.data(withJSONObject: value)
            let envelope = try JSONDecoder().decode(ApprovalCiphertext.self, from: data)
            let key = try ICloudApprovalRootKey().load()
            let plaintext = try ApprovalCrypto(rootKeyData: key).open(envelope, purpose: "notification")
            let ticket = try JSONDecoder().decode(PhoneApprovalTicket.self, from: plaintext)
            content.title = "Approval waiting"
            content.body = "Review the full request on your Mac or open Automic Vault."
            content.categoryIdentifier = ticket.requiresFullReview ? "AV_REVIEW" : "AV_ROUTINE"
            content.threadIdentifier = ticket.requestID.uuidString
            contentHandler(content)
            handler = nil
        } catch {
            content.title = "Approval waiting"
            content.body = "Open Automic Vault to review."
            content.categoryIdentifier = "AV_REVIEW"
            contentHandler(content)
            handler = nil
        }
    }

    override func serviceExtensionTimeWillExpire() {
        if let handler, let content { handler(content) }
        handler = nil
    }
}
