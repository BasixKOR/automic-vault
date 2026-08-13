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

private func sampleRequest(
    id: UUID = UUID(uuidString: "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE")!,
    command: String
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
        details: []
    )
}
