import Foundation
import MenubarHelperCore
import Testing

@Test func liveSecretUsesMergeNamesAndDisappearWithTheirProcessLifetime() throws {
    let controller = LiveSecretUseController<Int>()
    let startedAt = Date(timeIntervalSince1970: 1_000)
    controller.record(
        process: 7,
        launcherDesignatedRequirement: "identifier com.openai.codex",
        launcherName: "Codex",
        targetPath: "/usr/local/bin/node",
        processID: 42,
        secretNames: ["API_TOKEN"],
        startedAt: startedAt
    )
    controller.record(
        process: 7,
        launcherDesignatedRequirement: "identifier com.openai.codex",
        launcherName: "Codex",
        targetPath: "/usr/local/bin/node",
        processID: 42,
        secretNames: ["DATABASE_URL"],
        startedAt: startedAt.addingTimeInterval(1)
    )

    let use = try #require(controller.snapshots(isLive: { $0 == 7 }).first)
    #expect(use.launcherName == "Codex")
    #expect(use.targetPath == "/usr/local/bin/node")
    #expect(use.processID == 42)
    #expect(use.secretNames == ["API_TOKEN", "DATABASE_URL"])
    #expect(use.startedAt == startedAt)
    #expect(controller.snapshots(isLive: { _ in false }).isEmpty)
}

@Test func liveSecretUsesDoNotConflateLauncherIdentityOrTarget() {
    let controller = LiveSecretUseController<Int>()
    for (requirement, target) in [
        ("identifier com.example.one", "/bin/one"),
        ("identifier com.example.two", "/bin/one"),
        ("identifier com.example.one", "/bin/two"),
    ] {
        controller.record(
            process: 7,
            launcherDesignatedRequirement: requirement,
            launcherName: "Same Name",
            targetPath: target,
            processID: 42,
            secretNames: ["API_TOKEN"]
        )
    }
    #expect(controller.snapshots(isLive: { _ in true }).count == 3)
}
