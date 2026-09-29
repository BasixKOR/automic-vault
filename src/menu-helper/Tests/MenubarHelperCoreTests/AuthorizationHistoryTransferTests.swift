import Foundation
import Testing
@testable import MenubarHelperCore

@Test func historyTransferPreservesLargeSnapshotsAndRejectsInvalidContinuations() throws {
    let transfer = AuthorizationHistoryTransfer()
    let otherConnection = AuthorizationHistoryTransfer()
    let data = Data(String(repeating: "界", count: 400_000).utf8)
    #expect(transfer.next(offset: 0) == nil)
    #expect(!transfer.prepare(data))
    #expect(transfer.begin())
    #expect(!transfer.begin())
    #expect(transfer.prepare(data))
    #expect(!transfer.prepare(data))
    #expect(otherConnection.next(offset: 0) == nil)
    var received = Data()
    while received.count < data.count {
        #expect(transfer.next(offset: received.count + 1) == nil)
        let chunk = try #require(transfer.next(offset: received.count))
        #expect(chunk.total == data.count)
        #expect(chunk.offset == received.count)
        #expect(chunk.bytes.count <= AuthorizationHistoryTransfer.chunkBytes)
        received.append(chunk.bytes)
        #expect(transfer.next(offset: chunk.offset) == nil)
    }
    #expect(received == data)
    #expect(transfer.next(offset: data.count) == nil)
    #expect(!transfer.prepare(data))
}

@Test func historyTransferCancellationDiscardsPreparedAndPendingReads() {
    let data = Data(repeating: 1, count: AuthorizationHistoryTransfer.chunkBytes + 1)
    for prepared in [false, true] {
        let transfer = AuthorizationHistoryTransfer()
        #expect(transfer.begin())
        if prepared {
            #expect(transfer.prepare(data))
            #expect(transfer.next(offset: 0) != nil)
        }
        transfer.cancel()
        #expect(!transfer.prepare(data))
        #expect(transfer.next(offset: AuthorizationHistoryTransfer.chunkBytes) == nil)
        #expect(!transfer.begin())
    }
}
