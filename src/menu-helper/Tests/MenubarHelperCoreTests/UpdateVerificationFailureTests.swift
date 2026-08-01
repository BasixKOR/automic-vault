import Foundation
import Testing
@testable import MenubarHelperCore

@Test func updateVerificationFailureSearchUsesDialogText() {
    let components = URLComponents(url: updateVerificationIssuesURL, resolvingAgainstBaseURL: false)

    #expect(components?.host == "github.com")
    #expect(components?.path == "/automic-vault/automic-vault/issues")
    #expect(
        components?.queryItems == [
            URLQueryItem(name: "q", value: "is:issue \"\(updateVerificationFailureText)\"")
        ]
    )
}
