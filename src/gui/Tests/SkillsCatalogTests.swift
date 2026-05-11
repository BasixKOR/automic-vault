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

    func testRemoteInstallUsesSourceAtSkillCommand() async throws {
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

        _ = await catalog.installSkill(
            name: "vercel-labs/agent-skills@web-design-guidelines",
            progress: { _ in }
        )

        XCTAssertEqual(invocations, [
            ["add", "-g", "-y", "vercel-labs/agent-skills@web-design-guidelines"],
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

    func testDecodeSearchResponseBuildsInstallableSkillRows() throws {
        let json = """
        {
          "query": "browser",
          "searchType": "fuzzy",
          "skills": [
            {
              "id": "vercel-labs/agent-browser/agent-browser",
              "skillId": "agent-browser",
              "name": "agent-browser",
              "source": "vercel-labs/agent-browser",
              "installs": 259419,
              "sourceType": "github",
              "installUrl": "https://github.com/vercel-labs/agent-browser",
              "url": "https://skills.sh/vercel-labs/agent-browser/agent-browser"
            }
          ],
          "count": 1
        }
        """

        let response = try SkillsCatalog.decodeSearchResponse(from: Data(json.utf8))
        let result = try XCTUnwrap(response.skills.first?.searchResult)
        let detail = result.fallbackDetail

        XCTAssertEqual(result.name, "npm:skills:vercel-labs/agent-browser@agent-browser")
        XCTAssertEqual(result.managementBackend, .npmSkills)
        XCTAssertEqual(detail.skillName, "vercel-labs/agent-browser@agent-browser")
    }

    func testRemoteSearchFallsBackToPublicAPI() async throws {
        let json = """
        {
          "query": "browser",
          "searchType": "fuzzy",
          "skills": [
            {
              "id": "vercel-labs/agent-browser/agent-browser",
              "skillId": "agent-browser",
              "name": "agent-browser",
              "source": "vercel-labs/agent-browser",
              "installs": 259419
            }
          ],
          "count": 1
        }
        """
        var paths: [String] = []
        let catalog = SkillsCatalog(apiDataFetcher: { url in
            paths.append(url.path)
            if url.path == "/api/v1/skills/search" {
                throw SkillsCatalogError.apiUnavailable("auth required")
            }
            return Data(json.utf8)
        })

        let page = await catalog.searchPackages(
            query: "browser",
            offset: 0,
            limit: 10,
            excludingInstalledSkillNames: []
        )

        XCTAssertEqual(paths, ["/api/v1/skills/search", "/api/search"])
        XCTAssertEqual(page.packages.map(\.name), [
            "npm:skills:vercel-labs/agent-browser@agent-browser",
        ])
    }

    func testDecodePulseResponseBuildsTrendingPage() async throws {
        let json = """
        {
          "data": [
            {
              "id": "vercel-labs/agent-skills/web-design-guidelines",
              "slug": "web-design-guidelines",
              "name": "web-design-guidelines",
              "source": "vercel-labs/agent-skills",
              "installs": 310589,
              "sourceType": "github",
              "installUrl": "https://github.com/vercel-labs/agent-skills",
              "url": "https://skills.sh/vercel-labs/agent-skills/web-design-guidelines"
            }
          ],
          "pagination": {
            "page": 0,
            "perPage": 1,
            "total": 23,
            "hasMore": true
          }
        }
        """
        let catalog = SkillsCatalog(apiDataFetcher: { _ in Data(json.utf8) })

        let page = await catalog.fetchPulsePackages(
            offset: 0,
            limit: 1,
            excludingInstalledSkillNames: []
        )

        XCTAssertEqual(page.totalCount, 23)
        XCTAssertEqual(page.nextOffset, 1)
        XCTAssertEqual(page.packages.map(\.name), [
            "npm:skills:vercel-labs/agent-skills@web-design-guidelines",
        ])
        XCTAssertEqual(page.packages.first?.pulseKind, "updated")
    }

    func testPulseFallsBackToTrendingHTML() async throws {
        let html = """
        <script>self.__next_f.push([1,"{\\"source\\":\\"vercel-labs/skills\\",\\"skillId\\":\\"find-skills\\",\\"name\\":\\"find-skills\\",\\"installs\\":182300},{\\"source\\":\\"anthropic/skills\\",\\"skillId\\":\\"pdf\\",\\"name\\":\\"pdf\\",\\"installs\\":113}\\"totalSkills\\":9672,\\"view\\":\\"trending\\"}"])</script>
        """
        var paths: [String] = []
        let catalog = SkillsCatalog(apiDataFetcher: { url in
            paths.append(url.path)
            if url.path == "/api/v1/skills" {
                throw SkillsCatalogError.apiUnavailable("auth required")
            }
            return Data(html.utf8)
        })

        let page = await catalog.fetchPulsePackages(
            offset: 0,
            limit: 1,
            excludingInstalledSkillNames: []
        )

        XCTAssertEqual(paths, ["/api/v1/skills", "/trending"])
        XCTAssertEqual(page.totalCount, 9672)
        XCTAssertEqual(page.nextOffset, 1)
        XCTAssertEqual(page.packages.map(\.name), [
            "npm:skills:vercel-labs/skills@find-skills",
        ])
    }

    func testRemoteSearchFiltersInstalledSkillNames() async throws {
        let json = """
        {
          "query": "design",
          "searchType": "fuzzy",
          "skills": [
            {
              "id": "vercel-labs/agent-skills/web-design-guidelines",
              "slug": "web-design-guidelines",
              "name": "web-design-guidelines",
              "source": "vercel-labs/agent-skills",
              "installs": 310589
            },
            {
              "id": "vercel-labs/agent-skills/vercel-composition-patterns",
              "slug": "vercel-composition-patterns",
              "name": "vercel-composition-patterns",
              "source": "vercel-labs/agent-skills",
              "installs": 168437
            }
          ],
          "count": 2
        }
        """
        let catalog = SkillsCatalog(apiDataFetcher: { _ in Data(json.utf8) })

        let page = await catalog.searchPackages(
            query: "design",
            offset: 0,
            limit: 10,
            excludingInstalledSkillNames: ["web-design-guidelines"]
        )

        XCTAssertEqual(page.packages.map(\.name), [
            "npm:skills:vercel-labs/agent-skills@vercel-composition-patterns",
        ])
    }

    func testAPIFailureReturnsEmptyPage() async {
        let catalog = SkillsCatalog(apiDataFetcher: { _ in
            throw SkillsCatalogError.apiUnavailable("offline")
        })

        let page = await catalog.searchPackages(
            query: "browser",
            offset: 0,
            limit: 10,
            excludingInstalledSkillNames: []
        )

        XCTAssertEqual(page.packages, [])
        XCTAssertEqual(page.totalCount, 0)
        XCTAssertNil(page.nextOffset)

        let pulsePage = await catalog.fetchPulsePackages(
            offset: 0,
            limit: 10,
            excludingInstalledSkillNames: []
        )

        XCTAssertEqual(pulsePage.packages, [])
        XCTAssertEqual(pulsePage.totalCount, 0)
        XCTAssertNil(pulsePage.nextOffset)
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
