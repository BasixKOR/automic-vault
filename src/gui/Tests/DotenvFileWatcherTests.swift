import XCTest
@testable import AutomicVaultApp

final class DotenvFileWatcherTests: XCTestCase {
    func testSecretShapedPlaintextKeysIgnoresOrdinaryConfiguration() {
        let contents = """
        DOTENV_PUBLIC_KEY=abc
        MIN_MACOS_VERSION=26.0
        NUKE_HELPER_VERSION=12
        TEAM_COMMON_NAME="Developer ID Application: Example"
        TEAM_IDENTIFIER=ZU76A67LGU
        API_BASE_URL=https://api.example.test
        AUTH_TOKEN_URL=https://auth.example.test/oauth/token
        NEXT_PUBLIC_TOKEN=visible
        STRIPE_PUBLISHABLE_KEY=pk_live_abcdefghijklmnopqrstuvwxyz
        VITE_API_KEY=public-browser-config
        ALREADY_SECRET=encrypted:abc
        """

        XCTAssertEqual(DotenvFileWatcher.secretShapedPlaintextKeys(in: contents), [])
    }

    func testSecretShapedPlaintextKeysFindsSensitiveNamesAndValues() {
        let contents = #"""
        OPENAI_API_KEY=sk-proj-abcdefghijklmnopqrstuvwxyz123456
        export NPM_TOKEN="npm_abcdefghijklmnopqrstuvwxyz"
        STRIPE_SECRET_KEY='sk_live_abcdefghijklmnopqrstuvwxyz'
        DATABASE_URL=postgres://user:password@example.test/app
        PLAIN_VALUE=github_pat_1234567890abcdefghijklmnopqrstuvwxyz
        PRIVATE_MATERIAL="-----BEGIN PRIVATE KEY-----\nabc\n-----END PRIVATE KEY-----"
        JWT_VALUE=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjMifQ.signature123
        """#

        XCTAssertEqual(
            DotenvFileWatcher.secretShapedPlaintextKeys(in: contents),
            [
                "OPENAI_API_KEY",
                "NPM_TOKEN",
                "STRIPE_SECRET_KEY",
                "DATABASE_URL",
                "PLAIN_VALUE",
                "PRIVATE_MATERIAL",
                "JWT_VALUE",
            ]
        )
    }

    func testSecretShapedPlaintextKeysParsesColonAssignmentsAndComments() {
        let contents = """
        DB_PASSWORD: password # comment
        FEATURE_FLAG: enabled
        BAD-NAME=secret
        # COMMENTED_TOKEN=secret
        """

        XCTAssertEqual(
            DotenvFileWatcher.secretShapedPlaintextKeys(in: contents),
            ["DB_PASSWORD"]
        )
    }
}
