import Foundation
import Testing
@testable import MenubarHelperCore

@Test func verifiedLauncherHelpersDefaultOnAndRoundTripDisabledEntries() throws {
    let defaults = VerifiedLauncherHelperConfiguration()
    #expect(defaults.isEnabled(codexVerifiedLauncherHelper))
    #expect(defaults.isEnabled(claudeCodeVerifiedLauncherHelper))

    let configured = VerifiedLauncherHelperConfiguration(
        disabledHelperIDs: [codexVerifiedLauncherHelper.id, "future-helper"]
    )
    let data = try JSONEncoder().encode(configured)
    #expect(decodeVerifiedLauncherHelperConfiguration(data) == configured)
}

@Test func malformedVerifiedLauncherHelperConfigurationFailsClosed() {
    let configuration = decodeVerifiedLauncherHelperConfiguration(Data("not json".utf8))
    #expect(!configuration.isEnabled(codexVerifiedLauncherHelper))
    #expect(!configuration.isEnabled(claudeCodeVerifiedLauncherHelper))
}

@Test func enablesAndRoundTripsAnExactUserApprovedHelper() throws {
    let helper = userApprovedHelper()
    var configuration = VerifiedLauncherHelperConfiguration(disabledHelperIDs: [helper.id])
    configuration.enable([helper])

    #expect(configuration.userApprovedHelpers == [helper])
    #expect(configuration.isEnabled(helper))
    #expect(configuration.catalogHelper(matching: helper) == helper)
    let data = try JSONEncoder().encode(configuration)
    #expect(decodeVerifiedLauncherHelperConfiguration(data) == configuration)
}

@Test func discoveredHelperRequiresUserApprovalBeforeItIsEnabled() {
    #expect(!VerifiedLauncherHelperConfiguration().isEnabled(userApprovedHelper()))
}

@Test func legacyConfigurationDecodesWithoutUserApprovedHelpers() {
    let configuration = decodeVerifiedLauncherHelperConfiguration(
        Data(#"{"disabledHelperIDs":["codex"]}"#.utf8)
    )
    #expect(configuration.disabledHelperIDs == ["codex"])
    #expect(configuration.userApprovedHelpers.isEmpty)
}

@Test func invalidUserApprovedHelperPathFailsClosed() throws {
    let valid = userApprovedHelper()
    let invalid = VerifiedLauncherHelper(
        id: "",
        name: valid.name,
        appName: valid.appName,
        appBundleIdentifier: valid.appBundleIdentifier,
        appTeamIdentifier: valid.appTeamIdentifier,
        helperSigningIdentifier: valid.helperSigningIdentifier,
        helperTeamIdentifier: valid.helperTeamIdentifier,
        relativePath: "../Other.app/Contents/MacOS/Other"
    )
    let encodedInvalid = VerifiedLauncherHelper(
        id: userApprovedVerifiedLauncherHelperID(invalid),
        name: invalid.name,
        appName: invalid.appName,
        appBundleIdentifier: invalid.appBundleIdentifier,
        appTeamIdentifier: invalid.appTeamIdentifier,
        helperSigningIdentifier: invalid.helperSigningIdentifier,
        helperTeamIdentifier: invalid.helperTeamIdentifier,
        relativePath: invalid.relativePath
    )
    let data = try JSONEncoder().encode(VerifiedLauncherHelperConfiguration(
        userApprovedHelpers: [encodedInvalid]
    ))
    let configuration = decodeVerifiedLauncherHelperConfiguration(data)
    #expect(!configuration.isEnabled(codexVerifiedLauncherHelper))
    #expect(configuration.userApprovedHelpers.isEmpty)
}

private func userApprovedHelper() -> VerifiedLauncherHelper {
    let helper = VerifiedLauncherHelper(
        id: "",
        name: "Package Manager Manager Menu",
        appName: "Package Manager Manager",
        appBundleIdentifier: "dev.mxcl.pmm",
        appTeamIdentifier: "ZU76A67LGU",
        helperSigningIdentifier: "dev.mxcl.pmm.menu",
        helperTeamIdentifier: "ZU76A67LGU",
        relativePath: "Contents/Library/LoginItems/Package Manager Manager Menu.app/Contents/MacOS/PMMMenuBar"
    )
    return VerifiedLauncherHelper(
        id: userApprovedVerifiedLauncherHelperID(helper),
        name: helper.name,
        appName: helper.appName,
        appBundleIdentifier: helper.appBundleIdentifier,
        appTeamIdentifier: helper.appTeamIdentifier,
        helperSigningIdentifier: helper.helperSigningIdentifier,
        helperTeamIdentifier: helper.helperTeamIdentifier,
        relativePath: helper.relativePath
    )
}

@Test(.enabled(if: FileManager.default.fileExists(
    atPath: "/Applications/Package Manager Manager.app"
)))
func discoversInstalledPackageManagerManagerHelpers() async {
    let helpers = await discoverVerifiedLauncherHelpers(
        in: URL(fileURLWithPath: "/Applications/Package Manager Manager.app", isDirectory: true)
    )
    #expect(helpers.contains { $0.helperSigningIdentifier == "dev.mxcl.pmm.menu" })
    #expect(helpers.contains { $0.helperSigningIdentifier == "pmmctl" })
}
