import XCTest
@testable import AutomicVaultApp

final class SkillsCatalogTests: XCTestCase {
    func testDecodeInstalledSkillsKeepsGlobalOnly() throws {
        let json = """
        [
          {"name":"codex-review","path":"/Users/me/.codex/skills/codex-review","scope":"global","agents":["Codex"]},
          {"name":"project-only","path":"/repo/.agents/skills/project-only","scope":"project","agents":["Codex"]},
          {"name":"","path":"/Users/me/.codex/skills/empty","scope":"global","agents":["Codex"]}
        ]
        """

        let records = try SkillsCatalog.decodeInstalledSkills(from: Data(json.utf8))

        XCTAssertEqual(records.map(\.name), ["codex-review"])
        XCTAssertEqual(records.first?.record.managementBackend, .npmSkills)
        XCTAssertEqual(records.first?.record.source, .npm(packageName: "skills"))
        XCTAssertEqual(records.first?.detail.skillName, "codex-review")
    }

    func testResolverPrefersDirectSkillsCommandOverNpx() throws {
        let binDirectory = try temporaryBinDirectory()
        let skillsPath = try installExecutable(named: "skills", in: binDirectory)
        _ = try installExecutable(named: "npx", in: binDirectory)

        let command = try XCTUnwrap(SkillsCatalog.resolveCommand(
            fileManager: .default,
            environment: ["PATH": binDirectory.path],
            includeFixedPaths: false
        ))

        XCTAssertEqual(command.executablePath, skillsPath.path)
        XCTAssertEqual(command.baseArguments, [])
        XCTAssertEqual(command.displayName, "skills")
    }

    func testResolverFallsBackToNpxSkills() throws {
        let binDirectory = try temporaryBinDirectory()
        let npxPath = try installExecutable(named: "npx", in: binDirectory)

        let command = try XCTUnwrap(SkillsCatalog.resolveCommand(
            fileManager: .default,
            environment: ["PATH": binDirectory.path],
            includeFixedPaths: false
        ))

        XCTAssertEqual(command.executablePath, npxPath.path)
        XCTAssertEqual(command.baseArguments, ["--yes", "skills"])
        XCTAssertEqual(command.displayName, "npx skills")
    }

    func testUnavailableWhenSkillsAndNpxAreMissing() {
        let command = SkillsCatalog.resolveCommand(
            fileManager: .default,
            environment: ["PATH": temporaryDirectory().path],
            includeFixedPaths: false
        )

        XCTAssertNil(command)
    }

    func testInstallAndRemoveUseGlobalSkillsCommands() async throws {
        var invocations: [[String]] = []
        let catalog = SkillsCatalog(
            commandResolver: {
                SkillsCatalog.Command(
                    executablePath: "/tmp/skills",
                    baseArguments: [],
                    displayName: "skills"
                )
            },
            commandRunner: { _, arguments in
                invocations.append(arguments)
                return Data("ok\n".utf8)
            }
        )

        _ = await catalog.installSkill(name: "browser-use", progress: { _ in })
        _ = await catalog.removeSkill(name: "browser-use", progress: { _ in })

        XCTAssertEqual(invocations, [
            ["add", "-g", "-y", "browser-use"],
            ["remove", "-g", "-y", "browser-use"],
        ])
    }

    func testSearchAddsInstallCandidateButOnlyKeepsInstalledGlobalRecords() throws {
        let installed = PackagePresentation(
            item: .installed(PackageRecord(
                name: "npm:skills:codex-review",
                source: .npm(packageName: "skills"),
                version: "global",
                description: "Globally installed for Codex.",
                securityState: nil,
                installRoot: "/Users/me/.codex/skills/codex-review",
                installPackageNames: ["codex-review"],
                managementBackend: .npmSkills
            )),
            detail: nil,
            freshness: 0.5
        )
        let catalog = SkillsCatalog(commandResolver: {
            SkillsCatalog.Command(
                executablePath: "/tmp/skills",
                baseArguments: [],
                displayName: "skills"
            )
        })

        let installedResults = catalog.searchInstalledPackages(
            query: "review",
            installedPackages: [installed]
        )
        let installResults = catalog.searchInstalledPackages(
            query: "browser-use",
            installedPackages: [installed]
        )

        XCTAssertEqual(installedResults.map(\.displayName), ["codex-review"])
        XCTAssertEqual(installResults.map(\.displayName), ["browser-use"])
        XCTAssertEqual(installResults.first?.isNpmSkillsManaged, true)
    }

    private func temporaryBinDirectory() throws -> URL {
        let directory = temporaryDirectory()
            .appendingPathComponent(UUID().uuidString)
            .appendingPathComponent("bin")
        try FileManager.default.createDirectory(
            at: directory,
            withIntermediateDirectories: true
        )
        return directory
    }

    private func temporaryDirectory() -> URL {
        FileManager.default.temporaryDirectory
            .appendingPathComponent("AutomicVaultGUITests")
    }

    private func installExecutable(named name: String, in directory: URL) throws -> URL {
        let url = directory.appendingPathComponent(name)
        FileManager.default.createFile(
            atPath: url.path,
            contents: Data("#!/bin/sh\nexit 0\n".utf8)
        )
        try FileManager.default.setAttributes(
            [.posixPermissions: 0o755],
            ofItemAtPath: url.path
        )
        return url
    }
}
