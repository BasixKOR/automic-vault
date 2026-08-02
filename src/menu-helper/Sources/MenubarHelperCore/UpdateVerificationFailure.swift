import Foundation

public let updateVerificationFailureText =
    "Update verification failed. Automic Vault refused to install the update."

public let updateVerificationIssuesURL: URL = {
    var components = URLComponents(
        string: "https://github.com/automic-vault/automic-vault/issues"
    )!
    components.queryItems = [
        URLQueryItem(name: "q", value: "is:issue \"\(updateVerificationFailureText)\"")
    ]
    return components.url!
}()
