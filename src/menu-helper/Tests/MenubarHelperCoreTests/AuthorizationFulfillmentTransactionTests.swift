import Testing
@testable import MenubarHelperCore

@Test func authorizationFulfillmentRecordsBeforeActivationObservationAndRelease() {
    var events: [String] = []
    let transaction = AuthorizationFulfillmentTransaction(material: "secret-material")

    let committed = transaction.commit(
        record: {
            events.append("record")
            return true
        },
        activate: { material in
            events.append("aws-registration:\(material)")
            events.append("authority-bookkeeping:\(material)")
        },
        observe: { material in events.append("live-use:\(material)") },
        release: { material in events.append("successful-reply:\(material)") }
    )

    #expect(committed)
    #expect(events == [
        "record",
        "aws-registration:secret-material",
        "authority-bookkeeping:secret-material",
        "live-use:secret-material",
        "successful-reply:secret-material",
    ])
}

@Test func authorizationFulfillmentRecordFailurePreventsEverySideEffect() {
    var events: [String] = []
    let transaction = AuthorizationFulfillmentTransaction(material: "secret-material")

    let committed = transaction.commit(
        record: {
            events.append("record-failed")
            return false
        },
        activate: { _ in
            events.append("aws-registration")
            events.append("grant-blessing-or-provenance")
        },
        observe: { _ in events.append("live-use") },
        release: { _ in events.append("successful-reply") }
    )

    #expect(!committed)
    #expect(events == ["record-failed"])
}
