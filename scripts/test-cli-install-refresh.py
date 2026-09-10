#!/usr/bin/env python3
"""Exercise the dashboard's real reload method with a blocked snapshot load."""
from pathlib import Path
import subprocess
import sys
import tempfile

source = (Path(__file__).resolve().parents[1] /
          "src/menu-helper/Sources/MenubarHelper/MainWindow.swift").read_text()
reload_method = source.split("final class DashboardModel:", 1)[1].split(
    "    func reload() {", 1
)[1].split(
    "    private func reloadAuthorizationState()", 1
)[0]

fixture = """
import Foundation
import Synchronization

let loads = Mutex((count: 0, active: 0, peak: 0))

let (snapshotStarted, started) = AsyncStream<Void>.makeStream()
let finishSnapshot = DispatchSemaphore(value: 0)

struct DashboardSnapshot: Sendable {
    var policy = "loaded"
    var accessRequests = [String]()
    var detectorFindings = ["preserved"]
    static func load() -> Self {
        loads.withLock {
            $0.count += 1
            $0.active += 1
            $0.peak = max($0.peak, $0.active)
        }
        defer { loads.withLock { $0.active -= 1 } }
        started.yield(())
        finishSnapshot.wait()
        return Self(detectorFindings: [])
    }
}
enum CLIInstallState { case current, outdated }
func currentCLIInstallState() -> CLIInstallState { .current }
func loadLauncherBundleEnrollments() -> [String] { [] }
func loadAccessRequestRecords() -> [String] { ["latest"] }

@MainActor final class Model {
    var reloadTask: Task<Void, Never>?
    var reloadPending = false
    var isReloading = false
    var snapshot = DashboardSnapshot()
    var cliInstallState = CLIInstallState.outdated
    var launcherBundles: [String] = []
    func normalizeSelection() {}
    func invalidateForTest() { invalidateReload() }
    func reload() {
""" + reload_method + """
}

DispatchQueue.global().asyncAfter(deadline: .now() + 10) {
    print("FAIL: reload timed out")
    exit(1)
}
let model = Model()
model.reload()
for await _ in snapshotStarted { break }
guard model.cliInstallState == .current, model.isReloading else {
    print("FAIL: Update av CLI remains visible while dashboard loading is blocked")
    exit(1)
}
let first = model.reloadTask!
for _ in 0..<100 { model.reload() }
assert(!first.isCancelled, "refresh must not cancel a usable result")
finishSnapshot.signal()
await first.value
assert(model.snapshot.accessRequests == ["latest"])
assert(model.snapshot.detectorFindings == ["preserved"])
// A burst queues exactly one follow-up, after the first load finishes.
for await _ in snapshotStarted { break }
let second = model.reloadTask!
finishSnapshot.signal()
await second.value
assert(!model.isReloading)
assert(model.snapshot.detectorFindings == ["preserved"])
assert(loads.withLock { $0.count == 2 && $0.peak == 1 })
assert(model.reloadTask == nil)

model.reload()
for await _ in snapshotStarted { break }
let invalidated = model.reloadTask!
model.reload() // A pending refresh must also be invalidated by a policy edit.
model.invalidateForTest()
model.snapshot.policy = "edited"
model.reloadAccessRequests()
assert(model.snapshot.accessRequests == ["latest"])
assert(invalidated.isCancelled)
assert(!model.isReloading)
assert(!model.reloadPending)
assert(loads.withLock { $0.count == 3 })
model.reload() // Wait for the invalidated work instead of overlapping it.
finishSnapshot.signal()
await invalidated.value
assert(model.snapshot.policy == "edited", "stale policy result was applied")
for await _ in snapshotStarted { break }
let replacement = model.reloadTask!
finishSnapshot.signal()
await replacement.value
assert(loads.withLock { $0.count == 4 && $0.peak == 1 })
assert(model.reloadTask == nil && !model.isReloading)
print("PASS: early CLI status, coalesced refreshes, fresh history, and stale policy rejection")
"""

with tempfile.TemporaryDirectory(prefix="av-cli-refresh-") as directory:
    path = Path(directory) / "main.swift"
    path.write_text(fixture)
    result = subprocess.run(["swift", "-swift-version", "6", str(path)])
    sys.exit(result.returncode)
