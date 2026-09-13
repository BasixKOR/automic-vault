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

@Test(.enabled(if: ProcessInfo.processInfo.environment["AV_BENCHMARK_AUTHORIZATION"] == "1"))
func rollingAuthorizationHistoryPerformance() throws {
    let now = Date(timeIntervalSince1970: 4_000_000)
    let directory = FileManager.default.temporaryDirectory
        .appendingPathComponent("av-history-benchmark-\(UUID().uuidString)", isDirectory: true)
    defer { try? FileManager.default.removeItem(at: directory) }
    let store = try AuthorizationHistoryStore(
        url: directory.appendingPathComponent("history.sqlite3"),
        keyData: Data(repeating: 7, count: 32),
        now: { now }
    )
    func record(_ index: Int, reason: String) -> AccessRequestRecord {
        AccessRequestRecord(
            date: now.addingTimeInterval(TimeInterval(index - 10_001)),
            tool: "fixture", command: "fixture \(index)", decision: "Approved",
            reason: reason, launcher: "Fixture", callerPath: "/fixture/av",
            target: "/fixture/tool", cwd: "/fixture", keys: [], detail: nil
        )
    }
    let payload = String(repeating: "x", count: 2_000)
    try store.importRecords((0..<10_000).map { record($0, reason: payload) })
    let start = ContinuousClock.now
    let cpuStart = clock()
    #expect(store.append(record(10_000, reason: "Allowed")))
    let cpu = Double(clock() - cpuStart) / Double(CLOCKS_PER_SEC)
    print("Rolling Authorization History: 10k rows / 20 MiB append, wall=\(start.duration(to: .now)), CPU=\(cpu)s")
    let readStart = ContinuousClock.now
    let readCPUStart = clock()
    let recordCount = try store.records().count
    #expect(recordCount == 10_001)
    let readCPU = Double(clock() - readCPUStart) / Double(CLOCKS_PER_SEC)
    print("Rolling Authorization History: 10k rows / 20 MiB read, wall=\(readStart.duration(to: .now)), CPU=\(readCPU)s")
    let compactStore = try AuthorizationHistoryStore(
        url: directory.appendingPathComponent("compact.sqlite3"),
        keyData: Data(repeating: 7, count: 32),
        now: { now }
    )
    try compactStore.importRecords((0..<50_000).map { record($0 % 10_000, reason: "Read Only") })
    let compactReadStart = ContinuousClock.now
    let compactReadCPUStart = clock()
    #expect(try compactStore.records().count == 50_000)
    let compactReadCPU = Double(clock() - compactReadCPUStart) / Double(CLOCKS_PER_SEC)
    print("Rolling Authorization History: 50k compact rows read, wall=\(compactReadStart.duration(to: .now)), CPU=\(compactReadCPU)s")
    let pageStart = ContinuousClock.now
    #expect(try compactStore.page().records.count == 50)
    print("Rolling Authorization History: first page from 50k rows, wall=\(pageStart.duration(to: .now))")
}
