import Foundation
import Testing
@testable import MenubarHelperCore

@Test func blessedScriptManifestParsesStrictCommentYAML() throws {
    let data = Data("""
    #!/usr/local/bin/av inject --replace-existing-env +B +A /bin/sh
    # --- automic-vault
    # capabilities:
    #   gh: read-only
    #   aws: write
    #   stripe: trusted
    # ---
    echo ok
    """.utf8)

    let declaration = try blessedScriptDeclaration(data: data)

    #expect(declaration.keys == ["A", "B"])
    #expect(declaration.target == "/bin/sh")
    #expect(declaration.replaceExistingEnv)
    #expect(!declaration.allowMissingKeys)
    #expect(declaration.manifest.capabilities == [
        "gh": .readOnly,
        "aws": .fullExceptSecretDumps,
        "stripe": .fullExceptSecretDumps,
    ])
    #expect(declaration.checksum.count == 64)
}

@Test func blessedScriptManifestIsOptional() throws {
    let data = Data("""
    #!/usr/local/bin/av inject +TOKEN /bin/sh
    echo ok
    """.utf8)

    let declaration = try blessedScriptDeclaration(data: data)

    #expect(declaration.keys == ["TOKEN"])
    #expect(declaration.manifest.capabilities.isEmpty)
}

@Test func blessedScriptCanDeclareCapabilitiesWithoutSecrets() throws {
    let data = Data("""
    #!/usr/local/bin/av inject -- /bin/sh
    # --- automic-vault
    # capabilities:
    #   gh: read-only
    # ---
    echo ok
    """.utf8)

    let declaration = try blessedScriptDeclaration(data: data)

    #expect(declaration.keys.isEmpty)
    #expect(declaration.manifest.capabilities == ["gh": .readOnly])
}

@Test func launcherlessBlessingRequiresOneManualApprovalPerExecution() {
    let requirement = #"identifier "com.apple.Terminal""#
    let script = BlessedScript(
        path: "/tmp/script",
        checksum: "checksum",
        keys: ["TOKEN"],
        target: "/bin/sh",
        replaceExistingEnv: false,
        allowMissingKeys: false,
        capabilities: ["gh": .fullExceptSecretDumps],
        launchers: []
    )
    let endorsedScript = BlessedScript(
        path: script.path,
        checksum: script.checksum,
        keys: script.keys,
        target: script.target,
        replaceExistingEnv: script.replaceExistingEnv,
        allowMissingKeys: script.allowMissingKeys,
        capabilities: script.capabilities,
        launchers: [BlessedScriptLauncher(
            bundleIdentifier: "com.apple.Terminal",
            requirement: requirement
        )]
    )
    func matches(_ script: BlessedScript, launcherRequirement: String?, checksum: String = "checksum") -> Bool {
        script.matchesExecution(
            path: "/tmp/script",
            checksum: checksum,
            keys: ["TOKEN"],
            target: "/bin/sh",
            replaceExistingEnv: false,
            allowMissingKeys: false,
            launcherRequirement: launcherRequirement
        )
    }
    func executionMatches(_ script: BlessedScript, checksum: String = "checksum") -> Bool {
        script.matchesExecution(
            path: "/tmp/script",
            checksum: checksum,
            keys: ["TOKEN"],
            target: "/bin/sh",
            replaceExistingEnv: false,
            allowMissingKeys: false
        )
    }

    #expect(executionMatches(endorsedScript))
    #expect(!executionMatches(endorsedScript, checksum: "changed"))
    #expect(matches(script, launcherRequirement: nil))
    #expect(!matches(script, launcherRequirement: requirement))
    #expect(matches(endorsedScript, launcherRequirement: requirement))
    #expect(!matches(endorsedScript, launcherRequirement: nil))
    #expect(!matches(script, launcherRequirement: nil, checksum: "changed"))
}

@Test func blessingIdentityRequiresPathAndChecksum() {
    let script = BlessedScript(
        path: "/tmp/script",
        checksum: "checksum",
        keys: [],
        target: "/bin/sh",
        replaceExistingEnv: false,
        allowMissingKeys: false,
        capabilities: [:],
        launchers: []
    )

    #expect(script.matchesBlessing(path: "/tmp/script", checksum: "checksum"))
    #expect(!script.matchesBlessing(path: "/tmp/other", checksum: "checksum"))
    #expect(!script.matchesBlessing(path: "/tmp/script", checksum: "changed"))
}

@Test func reblessingPreservesLauncherEndorsementsAndAddsTheRequestedLauncher() {
    let terminal = BlessedScriptLauncher(
        bundleIdentifier: "com.apple.Terminal",
        requirement: #"identifier "com.apple.Terminal""#
    )
    let codex = BlessedScriptLauncher(
        bundleIdentifier: "com.openai.codex",
        requirement: #"identifier "com.openai.codex""#
    )
    let visualStudioCode = BlessedScriptLauncher(
        bundleIdentifier: "com.microsoft.VSCode",
        requirement: #"identifier "com.microsoft.VSCode""#
    )
    let previouslyEndorsed = [terminal, visualStudioCode]

    #expect(launcherEndorsementsForReblessing(
        previouslyEndorsed: previouslyEndorsed,
        requestedLauncher: nil
    ) == previouslyEndorsed)
    #expect(launcherEndorsementsForReblessing(
        previouslyEndorsed: previouslyEndorsed,
        requestedLauncher: codex
    ) == [terminal, visualStudioCode, codex])
    #expect(launcherEndorsementsForReblessing(
        previouslyEndorsed: previouslyEndorsed,
        requestedLauncher: terminal
    ) == previouslyEndorsed)
}

@Test(arguments: [
    """
    #!/bin/sh
    # --- automic-vault
    # capabilities:
    #   gh: read-only
    # ---
    """,
    """
    #!/bin/sh inject +A /bin/sh
    # --- automic-vault
    # capabilities:
    #   gh: read-only
    # ---
    """,
    """
    #!/usr/local/bin/av inject +A /bin/sh
    # --- automic-vault
    # capabilities:
    #   gh: read-only
    #   gh: trusted
    # ---
    """,
    """
    #!/usr/local/bin/av inject +A /bin/sh
    # --- automic-vault
    # capabilities:
    #   gh: anything
    # ---
    """,
    """
    #!/usr/local/bin/av inject +A sh
    # --- automic-vault
    # capabilities:
    #   gh: read-only
    # ---
    """,
])
func malformedBlessedScriptManifestsFailClosed(_ source: String) {
    #expect(throws: (any Error).self) {
        try blessedScriptDeclaration(data: Data(source.utf8))
    }
}

@Test func blessedScriptReadsAreBoundedAndRejectSymlinks() throws {
    let directory = FileManager.default.temporaryDirectory
        .appendingPathComponent("av-blessed-script-\(UUID().uuidString)", isDirectory: true)
    let script = directory.appendingPathComponent("script")
    let link = directory.appendingPathComponent("link")
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: directory) }
    let canonicalPath = script.resolvingSymlinksInPath().path

    try Data(repeating: 0, count: 1024 * 1024 + 1).write(to: script)
    #expect(throws: (any Error).self) { try readBlessedScript(path: canonicalPath) }

    try Data("ok".utf8).write(to: script)
    try FileManager.default.createSymbolicLink(at: link, withDestinationURL: script)
    #expect(throws: (any Error).self) { try readBlessedScript(path: link.path) }
    #expect(try readBlessedScript(path: canonicalPath) == Data("ok".utf8))
}
