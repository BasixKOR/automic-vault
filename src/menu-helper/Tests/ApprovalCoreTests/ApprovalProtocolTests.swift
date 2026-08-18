import ApprovalCore
import CryptoKit
import Foundation
import Testing

@Test func requestMutationCannotReuseResponse() throws {
    let key = Data(repeating: 7, count: ApprovalCrypto.rootKeyByteCount)
    let crypto = try ApprovalCrypto(rootKeyData: key)
    let request = try sampleRequest(command: "aws s3 ls")
    let response = try PhoneApprovalResponse(
        request: request,
        outcome: .approved,
        deviceID: "phone"
    )
    try response.validate(for: request)

    let changed = try sampleRequest(id: request.id, command: "aws s3 rm s3://bucket --recursive")
    #expect(throws: ApprovalProtocolError.mismatchedResponse) {
        try response.validate(for: changed)
    }

    let plaintext = try JSONEncoder().encode(ApprovalWireMessage.request(request))
    let sealed = try crypto.seal(plaintext, purpose: "transport")
    #expect(try crypto.open(sealed, purpose: "transport") == plaintext)
    #expect(throws: (any Error).self) {
        try crypto.open(sealed, purpose: "notification")
    }

    let ticket = try PhoneApprovalTicket(request: request)
    let notificationResponse = try PhoneApprovalResponse(
        requestID: ticket.requestID,
        requestDigest: ticket.requestDigest,
        outcome: .approved,
        deviceID: "phone"
    )
    try notificationResponse.validate(for: request)
}

@Test func addressesAndRegistrationProofsAreStableAndSeparated() throws {
    let crypto = try ApprovalCrypto(rootKeyData: Data(repeating: 9, count: 32))
    #expect(crypto.address.room.count == 43)
    #expect(crypto.address.credential.count == 43)
    #expect(crypto.address.room != crypto.address.credential)
    #expect(crypto.registrationProof(deviceID: "a") != crypto.registrationProof(deviceID: "b"))
}

@Test func presenceAndSyncRoundTrip() throws {
    let messages: [ApprovalWireMessage] = [
        .sync,
        .presence(try ApprovalMacPresence(macID: "mac", macName: "Studio", sentAtMilliseconds: 42)),
    ]
    for message in messages {
        let data = try JSONEncoder().encode(message)
        #expect(try JSONDecoder().decode(ApprovalWireMessage.self, from: data) == message)
    }
}

@Test func temporaryWriteAccessRequiresExplicitRequestCapability() throws {
    let request = try sampleRequest(
        command: "gh repo create",
        temporaryAccessGrantScope: "Ghostty, GH Authorization Gate, and Codex task AAAAAAAA"
    )
    let response = try PhoneApprovalResponse(
        request: request,
        outcome: .temporaryWriteAccess,
        deviceID: "phone"
    )
    try response.validate(for: request)

    let ineligible = try sampleRequest(command: "aws s3 ls")
    #expect(throws: ApprovalProtocolError.invalidRequest) {
        try PhoneApprovalResponse(
            request: ineligible,
            outcome: .temporaryWriteAccess,
            deviceID: "phone"
        )
    }

    let forged = try JSONDecoder().decode(
        PhoneApprovalResponse.self,
        from: JSONSerialization.data(withJSONObject: [
            "version": 1,
            "requestID": ineligible.id.uuidString,
            "requestDigest": try ineligible.digest().base64EncodedString(),
            "outcome": "temporaryWriteAccess",
            "deviceID": "phone",
            "decidedAtMilliseconds": 1,
        ])
    )
    #expect(throws: ApprovalProtocolError.mismatchedResponse) {
        try forged.validate(for: ineligible)
    }
}

private func sampleRequest(
    id: UUID = UUID(uuidString: "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE")!,
    command: String,
    temporaryAccessGrantScope: String? = nil
) throws -> PhoneApprovalRequest {
    try PhoneApprovalRequest(
        id: id,
        createdAtMilliseconds: 1_700_000_000_000,
        macName: "MacBook Pro",
        launcher: "Terminal",
        tool: "aws",
        command: command,
        cwd: "/tmp",
        secretNames: ["AWS_ACCESS_KEY_ID"],
        reason: "Unknown operation requires Approval",
        risks: [.unknown],
        details: [],
        temporaryAccessGrantScope: temporaryAccessGrantScope
    )
}
