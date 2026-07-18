import Testing
@testable import MenubarHelperCore

@Test func graphQLQueriesAreReadOnly() {
    #expect(graphQLRequestIsReadOnly(query: "query { viewer { login } }"))
    #expect(graphQLRequestIsReadOnly(query: "{ viewer { login } }"))
    #expect(graphQLRequestIsReadOnly(query: """
    query PullRequest($owner: String!, $repo: String!, $number: Int!) {
      repository(owner: $owner, name: $repo) {
        pullRequest(number: $number) { body bodyHTML }
      }
    }
    """))
}

@Test func graphQLMutationsAndSubscriptionsAreNotReadOnly() {
    #expect(!graphQLRequestIsReadOnly(query: "mutation { addStar(input: {}) { clientMutationId } }"))
    #expect(!graphQLRequestIsReadOnly(query: "subscription { viewer { login } }"))
}

@Test func graphQLSelectedOperationDeterminesClassification() {
    let document = """
    query Read { viewer { login } }
    mutation Write { addStar(input: {}) { clientMutationId } }
    """

    #expect(graphQLRequestIsReadOnly(query: document, operationName: "Read"))
    #expect(!graphQLRequestIsReadOnly(query: document, operationName: "Write"))
    #expect(!graphQLRequestIsReadOnly(query: document, operationName: "Missing"))
    #expect(!graphQLRequestIsReadOnly(query: document))
}

@Test func graphQLClassificationFailsClosed() {
    #expect(!graphQLRequestIsReadOnly(query: ""))
    #expect(!graphQLRequestIsReadOnly(query: "not graphql"))
    #expect(!graphQLRequestIsReadOnly(query: "fragment Login on User { login }"))
}
