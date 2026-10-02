import Foundation
import Testing
@testable import MenubarHelperCore

@Test func helperCatalogStartsEmptyAndRoundTripsDisabledEntries() throws {
    let defaults = VerifiedLauncherHelperConfiguration()
    #expect(defaults.helpers.isEmpty)
    #expect(!defaults.isEnabled(codexVerifiedLauncherHelper))
    #expect(!defaults.isEnabled(claudeCodeVerifiedLauncherHelper))
    let configured = VerifiedLauncherHelperConfiguration(disabledHelperIDs: ["codex", "future-helper"])
    #expect(decodeVerifiedLauncherHelperConfiguration(try JSONEncoder().encode(configured)) == configured)
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

@Test func enablingInvalidDiscoveredHelperDoesNotCorruptTheCatalog() throws {
    let valid = userApprovedHelper()
    let invalid = VerifiedLauncherHelper(
        id: "forged",
        name: valid.name,
        appName: valid.appName,
        appBundleIdentifier: valid.appBundleIdentifier,
        appTeamIdentifier: valid.appTeamIdentifier,
        helperSigningIdentifier: valid.helperSigningIdentifier,
        helperTeamIdentifier: valid.helperTeamIdentifier,
        relativePath: valid.relativePath
    )
    var configuration = VerifiedLauncherHelperConfiguration(disabledHelperIDs: [invalid.id])
    configuration.enable([invalid])

    #expect(configuration.userApprovedHelpers.isEmpty)
    #expect(configuration.disabledHelperIDs.contains(invalid.id))
    let data = try JSONEncoder().encode(configuration)
    #expect(decodeVerifiedLauncherHelperConfiguration(data) == configuration)
}

@Test func invalidHelperCannotEnableAMatchingAssociation() {
    let invalid = VerifiedLauncherHelper(
        id: codexVerifiedLauncherHelper.id,
        name: codexVerifiedLauncherHelper.name,
        appName: codexVerifiedLauncherHelper.appName,
        appBundleIdentifier: codexVerifiedLauncherHelper.appBundleIdentifier,
        appTeamIdentifier: codexVerifiedLauncherHelper.appTeamIdentifier,
        helperSigningIdentifier: codexVerifiedLauncherHelper.helperSigningIdentifier,
        helperTeamIdentifier: codexVerifiedLauncherHelper.helperTeamIdentifier,
        relativePath: "../codex"
    )
    var configuration = VerifiedLauncherHelperConfiguration(
        disabledHelperIDs: [codexVerifiedLauncherHelper.id],
        userApprovedHelpers: [codexVerifiedLauncherHelper]
    )
    configuration.enable([invalid])

    #expect(!configuration.isEnabled(codexVerifiedLauncherHelper))
}

@Test func vendorCLIRequiresExactUserApprovedAssociation() throws {
    var configuration = VerifiedLauncherHelperConfiguration()
    #expect(shouldPreselectVerifiedLauncherHelper(codexVerifiedLauncherHelper))
    #expect(!configuration.isEnabled(codexVerifiedLauncherHelper))
    configuration.enable([codexVerifiedLauncherHelper])
    #expect(configuration.isEnabled(codexVerifiedLauncherHelper))
    #expect(decodeVerifiedLauncherHelperConfiguration(try JSONEncoder().encode(configuration)) == configuration)
}

@Test func helperRelativePathRejectsResolvedSymlinkEscape() throws {
    let root = FileManager.default.temporaryDirectory
        .appendingPathComponent("av-helper-path-\(UUID().uuidString)", isDirectory: true)
    let app = root.appendingPathComponent("Example.app", isDirectory: true)
    let contents = app.appendingPathComponent("Contents", isDirectory: true)
    let outside = root.appendingPathComponent("Outside", isDirectory: true)
    let helper = outside.appendingPathComponent("helper")
    try FileManager.default.createDirectory(at: contents, withIntermediateDirectories: true)
    try FileManager.default.createDirectory(at: outside, withIntermediateDirectories: true)
    try Data().write(to: helper)
    try FileManager.default.createSymbolicLink(
        at: contents.appendingPathComponent("Helpers", isDirectory: true),
        withDestinationURL: outside
    )
    defer { try? FileManager.default.removeItem(at: root) }

    #expect(verifiedLauncherHelperRelativePath(
        for: contents.appendingPathComponent("Helpers/helper"),
        inside: app
    ) == nil)
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

@Test func outsideBundlePermissionDefaultsOffAndRoundTripsPerHelper() throws {
    let helper = userApprovedHelper()
    var configuration = VerifiedLauncherHelperConfiguration()
    configuration.enable([helper])
    #expect(configuration.allowedOutsideBundleHelperIDs.isEmpty)
    configuration.allowedOutsideBundleHelperIDs.insert(helper.id)
    let data = try JSONEncoder().encode(configuration)
    let restored = decodeVerifiedLauncherHelperConfiguration(data)
    #expect(restored == configuration)
    #expect(!restored.allowedOutsideBundleHelperIDs.contains(codexVerifiedLauncherHelper.id))
    configuration.disabledHelperIDs.insert(helper.id)
    #expect(!configuration.isEnabled(helper))
    configuration.allowedOutsideBundleHelperIDs.remove(helper.id)
    #expect(configuration.allowedOutsideBundleHelperIDs.isEmpty)
}

@Test func legacyAndMalformedOutsideBundlePermissionsNeverOptIn() {
    let legacy = decodeVerifiedLauncherHelperConfiguration(Data("{}".utf8))
    #expect(legacy.allowedOutsideBundleHelperIDs.isEmpty)
    let malformed = decodeVerifiedLauncherHelperConfiguration(
        Data(#"{"allowedOutsideBundleHelperIDs":true}"#.utf8)
    )
    #expect(malformed.allowedOutsideBundleHelperIDs.isEmpty)
    #expect(!malformed.isEnabled(codexVerifiedLauncherHelper))
}

@Test func removingUserApprovedHelperRevokesAssociationAndOutsideBundlePermission() throws {
    let helper = userApprovedHelper()
    var configuration = VerifiedLauncherHelperConfiguration()
    configuration.enable([helper])
    configuration.allowedOutsideBundleHelperIDs.insert(helper.id)
    configuration.disabledHelperIDs.insert(helper.id)
    configuration.remove(helper)

    #expect(configuration.catalogHelper(matching: helper) == nil)
    #expect(!configuration.isEnabled(helper))
    #expect(!configuration.disabledHelperIDs.contains(helper.id))
    #expect(!configuration.allowedOutsideBundleHelperIDs.contains(helper.id))
    configuration.enable([helper])
    #expect(configuration.isEnabled(helper))
    #expect(!configuration.allowedOutsideBundleHelperIDs.contains(helper.id))
    #expect(decodeVerifiedLauncherHelperConfiguration(try JSONEncoder().encode(configuration)) == configuration)
}

@Test func preselectionChecksBothVendorIdentitiesAndExactHelper() {
    #expect(shouldPreselectVerifiedLauncherHelper(claudeCodeVerifiedLauncherHelper))
    #expect(!shouldPreselectVerifiedLauncherHelper(userApprovedHelper()))
    for (app, appTeam, signing, helperTeam, path) in [
        ("other.app", "2DC432GLL2", "codex", "2DC432GLL2", "Contents/Resources/codex"),
        ("com.openai.codex", "OTHER", "codex", "OTHER", "Contents/Resources/codex"),
        ("com.openai.codex", "2DC432GLL2", "other-helper", "2DC432GLL2", "Contents/Resources/codex"),
        ("com.openai.codex", "2DC432GLL2", "codex", "OTHER", "Contents/Resources/codex"),
        ("com.openai.codex", "2DC432GLL2", "codex", "2DC432GLL2", "../codex"),
    ] {
        #expect(!shouldPreselectVerifiedLauncherHelper(vendorHelper(
            app: app, appTeam: appTeam, signing: signing, helperTeam: helperTeam, path: path
        )))
    }
}

private func vendorHelper(
    app: String, appTeam: String, signing: String, helperTeam: String, path: String
) -> VerifiedLauncherHelper {
    let helper = VerifiedLauncherHelper(
        id: "", name: "CLI", appName: "App", appBundleIdentifier: app,
        appTeamIdentifier: appTeam, helperSigningIdentifier: signing,
        helperTeamIdentifier: helperTeam, relativePath: path
    )
    return VerifiedLauncherHelper(
        id: userApprovedVerifiedLauncherHelperID(helper), name: helper.name, appName: helper.appName,
        appBundleIdentifier: app, appTeamIdentifier: appTeam, helperSigningIdentifier: signing,
        helperTeamIdentifier: helperTeam, relativePath: path
    )
}

private let codexVerifiedLauncherHelper = vendorHelper(
    app: "com.openai.codex", appTeam: "2DC432GLL2", signing: "codex",
    helperTeam: "2DC432GLL2", path: "Contents/Resources/codex"
)
private let claudeCodeVerifiedLauncherHelper = vendorHelper(
    app: "com.anthropic.claudefordesktop", appTeam: "Q6L2SF6YDW",
    signing: "com.anthropic.claude-code", helperTeam: "Q6L2SF6YDW",
    path: "Contents/Resources/claude-code"
)

@Test func legacyBuiltInSettingsDoNotEnrollHelpers() {
    let configuration = decodeVerifiedLauncherHelperConfiguration(
        Data(#"{"disabledHelperIDs":[],"allowedOutsideBundleHelperIDs":["codex","claude-code"]}"#.utf8)
    )
    #expect(configuration.helpers.isEmpty)
    #expect(!configuration.isEnabled(codexVerifiedLauncherHelper))
    #expect(!configuration.isEnabled(claudeCodeVerifiedLauncherHelper))
}

@Test func sameSigningIdentityAtAnotherPathNeedsSeparateApproval() {
    let otherPath = vendorHelper(
        app: "com.openai.codex", appTeam: "2DC432GLL2", signing: "codex",
        helperTeam: "2DC432GLL2", path: "Contents/Other/codex"
    )
    var configuration = VerifiedLauncherHelperConfiguration()
    configuration.enable([codexVerifiedLauncherHelper])
    #expect(!configuration.isEnabled(otherPath))
    configuration.enable([otherPath])
    #expect(configuration.isEnabled(otherPath))
    #expect(configuration.userApprovedHelpers.count == 2)
}

@Test func helperDiscoveryAcceptsOnlyExecutableMachOImages() throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: root) }
    func thin(_ type: UInt8, littleEndian: Bool = true, is64: Bool = true) -> Data {
        var bytes: [UInt8] = littleEndian
            ? [is64 ? 0xcf : 0xce, 0xfa, 0xed, 0xfe]
            : [0xfe, 0xed, 0xfa, is64 ? 0xcf : 0xce]
        bytes += Array(repeating: 0, count: 8)
        bytes += littleEndian ? [type, 0, 0, 0] : [0, 0, 0, type]
        return Data(bytes)
    }
    func universal(_ slices: [Data], is64: Bool = false) -> Data {
        var data = Data([0xca, 0xfe, 0xba, is64 ? 0xbf : 0xbe, 0, 0, 0, UInt8(slices.count)])
        var offset = 8 + slices.count * (is64 ? 32 : 20)
        for slice in slices {
            data.append(Data(repeating: 0, count: 8))
            if is64 { data.append(Data(repeating: 0, count: 4)) }
            data.append(contentsOf: [0, 0, UInt8(offset >> 8), UInt8(offset & 255)])
            data.append(Data(repeating: 0, count: is64 ? 16 : 8))
            offset += slice.count
        }
        for slice in slices { data.append(slice) }
        return data
    }
    let cases: [(Data, Bool)] = [
        (thin(2), true), (thin(2, is64: false), true),
        (thin(2, littleEndian: false), true), (thin(2, littleEndian: false, is64: false), true),
        (thin(6), false), (thin(8), false), (thin(1), false),
        (universal([thin(2), thin(2)]), true),
        (universal([thin(2), thin(2)], is64: true), true),
        (universal([thin(2), thin(6)]), false),
        (Data(universal([thin(2)]).prefix(12)), false),
        (Data([0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 0]), false),
        (Data("#!/bin/sh\n".utf8), false), (Data(), false),
    ]
    for (index, entry) in cases.enumerated() {
        let url = root.appendingPathComponent("image-\(index)")
        try entry.0.write(to: url)
        #expect(isLauncherHelperExecutable(at: url) == entry.1)
    }
    #expect(!isLauncherHelperExecutable(at: root.appendingPathComponent("missing")))
}
