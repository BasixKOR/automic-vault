import GraphQL

public func graphQLRequestIsReadOnly(
    query: String,
    operationName: String? = nil
) -> Bool {
    guard let document = try? parse(source: query) else { return false }

    let operations = document.definitions.compactMap { definition in
        definition as? OperationDefinition
    }

    let operation: OperationDefinition
    if let operationName {
        let matches = operations.filter { $0.name?.value == operationName }
        guard matches.count == 1, let match = matches.first else { return false }
        operation = match
    } else {
        guard operations.count == 1, let onlyOperation = operations.first else { return false }
        operation = onlyOperation
    }

    return operation.operation == .query
}
