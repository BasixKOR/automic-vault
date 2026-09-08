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

let (snapshotStarted, started) = AsyncStream<Void>.makeStream()
let finishSnapshot = DispatchSemaphore(value: 0)

struct DashboardSnapshot: Sendable {
    var detectorFindings = ["preserved"]
    static func load() -> Self {
        started.yield(())
        finishSnapshot.wait()
        return Self(detectorFindings: [])
    }
}
enum CLIInstallState { case current, outdated }
func currentCLIInstallState() -> CLIInstallState { .current }
func loadLauncherBundleEnrollments() -> [String] { [] }

@MainActor final class Model {
    var reloadTask: Task<Void, Never>?
    var isReloading = false
    var snapshot = DashboardSnapshot()
    var cliInstallState = CLIInstallState.outdated
    var launcherBundles: [String] = []
    func normalizeSelection() {}
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
finishSnapshot.signal()
await model.reloadTask!.value
assert(!model.isReloading)
assert(model.snapshot.detectorFindings == ["preserved"])
print("PASS: CLI status refreshes before the dashboard finishes loading")
"""

with tempfile.TemporaryDirectory(prefix="av-cli-refresh-") as directory:
    path = Path(directory) / "main.swift"
    path.write_text(fixture)
    result = subprocess.run(["swift", "-swift-version", "6", str(path)])
    sys.exit(result.returncode)
