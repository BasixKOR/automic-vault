import Testing
@testable import MenubarHelperCore

@Test func approvalServiceRejectsGenericSecretLoad() {
    #expect(ApprovalServiceOperation(rawValue: "load") == nil)
}

@Test func conditionalSaveKeepsItsCompatibleWireValue() {
    #expect(ApprovalServiceOperation.saveIfAbsentOrEqual.rawValue == "save-if-absent")
}

@Test func approvalServiceOperationValuesAreUnique() {
    let values = ApprovalServiceOperation.allCases.map(\.rawValue)
    #expect(Set(values).count == values.count)
}
