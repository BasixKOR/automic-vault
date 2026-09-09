import Darwin
import Foundation
import Testing
@testable import MenubarHelperCore

// Opt in and filter to this test so other tests do not contaminate process CPU time.
@Test(.enabled(if: ProcessInfo.processInfo.environment["AV_BENCHMARK_AUTHORIZATION"] == "1"))
func authorizationHistoryPerformance() throws {
    let suite = "com.automicvault.benchmark.\(UUID().uuidString)"
    let defaults = try #require(UserDefaults(suiteName: suite))
    defer { defaults.removePersistentDomain(forName: suite) }
    func record(_ index: Int) -> AccessRequestRecord {
        AccessRequestRecord(
            date: Date(timeIntervalSince1970: Double(index)), tool: "fixture",
            command: "fixture list --project synthetic-\(index)", decision: "Approved",
            approvalSource: "Auto", reason: "Read Only", launcher: "Fixture Launcher",
            callerPath: "/fixture/av", target: "/fixture/tool", cwd: "/fixture",
            keys: ["SYNTHETIC_TOKEN"], detail: nil
        )
    }
    for index in 0..<50 {
        try #require(appendAccessRequestRecord(record(index), defaults: defaults))
    }
    let start = ContinuousClock.now
    let cpuStart = clock()
    for index in 50..<550 {
        try #require(appendAccessRequestRecord(record(index), defaults: defaults))
    }
    let cpu = Double(clock() - cpuStart) / Double(CLOCKS_PER_SEC)
    print("Authorization History: 500 appends, wall=\(start.duration(to: .now)), CPU=\(cpu)s")
    let records = loadAccessRequestRecords(defaults: defaults)
    #expect(records.count == 50)
    #expect(records.first?.command == record(549).command)
    #expect(records.last?.command == record(500).command)
}
