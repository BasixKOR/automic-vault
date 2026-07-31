import Security
import Testing
@testable import MenubarHelperCore

@Test func hardenedRuntimeIsRequiredForNewSecretGateLaunchers() {
    #expect(launcherRuntimeProtection(
        signatureFlags: 0,
        enabledEntitlements: []
    ) == .hardenedRuntimeMissing)
    #expect(launcherRuntimeProtection(
        signatureFlags: SecCodeSignatureFlags.runtime.rawValue,
        enabledEntitlements: []
    ) == .hardened)
    #expect(launcherRuntimeProtection(
        signatureFlags: SecCodeSignatureFlags.runtime.rawValue,
        enabledEntitlements: ["com.apple.security.cs.allow-jit"]
    ) == .hardened)
}

@Test func hardenedRuntimeExceptionsPreventSecretGateAdmission() {
    let unsafe: Set<String> = [
        "com.apple.security.cs.allow-dyld-environment-variables",
        "com.apple.security.cs.allow-unsigned-executable-memory",
        "com.apple.security.cs.disable-executable-page-protection",
        "com.apple.security.cs.disable-library-validation",
        "com.apple.security.get-task-allow",
    ]
    #expect(launcherRuntimeProtection(
        signatureFlags: SecCodeSignatureFlags.runtime.rawValue,
        enabledEntitlements: unsafe
    ) == .unsafeEntitlements(unsafe.sorted()))
}
