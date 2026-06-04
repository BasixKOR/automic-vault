import Foundation

struct DotenvAutoEncryptionResult: Equatable {
    let filePath: String
    let encryptedKeys: [String]
}

final class DotenvFileWatcher {
    private struct Watch {
        let source: DispatchSourceFileSystemObject
    }

    private struct CommandResult {
        let status: Int32
        let stdout: String
        let stderr: String
    }

    private let queue = DispatchQueue(label: "com.automicvault.dotenv-file-watcher")
    private let fileManager: FileManager
    private let notify: (DotenvAutoEncryptionResult) -> Void
    private let resolveBinaryURL: () -> URL?
    private var watches: [String: Watch] = [:]
    private var pendingWork: [String: DispatchWorkItem] = [:]

    init(
        fileManager: FileManager = .default,
        notify: @escaping (DotenvAutoEncryptionResult) -> Void,
        resolveBinaryURL: @escaping () -> URL? = DotenvFileWatcher.defaultBinaryURL
    ) {
        self.fileManager = fileManager
        self.notify = notify
        self.resolveBinaryURL = resolveBinaryURL
    }

    func watch(paths: [String]) {
        queue.async { [weak self] in
            for path in paths {
                self?.watchUnlocked(path: path)
            }
        }
    }

    func watch(path: String) {
        watch(paths: [path])
    }

    func stop() {
        queue.sync {
            for work in pendingWork.values {
                work.cancel()
            }
            pendingWork.removeAll()
            for path in Array(watches.keys) {
                cancelWatchUnlocked(path: path)
            }
        }
    }

    private func watchUnlocked(path: String) {
        guard let path = normalizedPath(path), watches[path] == nil else {
            return
        }
        guard fileManager.fileExists(atPath: path) else {
            return
        }

        let fd = open(path, O_EVTONLY)
        guard fd >= 0 else {
            return
        }

        let source = DispatchSource.makeFileSystemObjectSource(
            fileDescriptor: fd,
            eventMask: [.write, .extend, .attrib, .delete, .rename],
            queue: queue
        )
        source.setEventHandler { [weak self] in
            self?.handleEvent(path: path, events: source.data)
        }
        source.setCancelHandler {
            close(fd)
        }
        watches[path] = Watch(source: source)
        source.resume()
    }

    private func cancelWatchUnlocked(path: String) {
        guard let watch = watches.removeValue(forKey: path) else {
            return
        }
        watch.source.cancel()
    }

    private func handleEvent(path: String, events: DispatchSource.FileSystemEvent) {
        scheduleEncryptionCheck(path: path)
        if events.contains(.delete) || events.contains(.rename) {
            cancelWatchUnlocked(path: path)
            queue.asyncAfter(deadline: .now() + 1.0) { [weak self] in
                self?.watchUnlocked(path: path)
            }
        }
    }

    private func scheduleEncryptionCheck(path: String) {
        pendingWork[path]?.cancel()
        let work = DispatchWorkItem { [weak self] in
            self?.encryptIfNeeded(path: path)
        }
        pendingWork[path] = work
        queue.asyncAfter(deadline: .now() + 0.45, execute: work)
    }

    private func encryptIfNeeded(path: String) {
        pendingWork[path] = nil
        guard fileManager.fileExists(atPath: path),
              let binaryURL = resolveBinaryURL() else {
            return
        }

        let check = runAv(binaryURL, arguments: ["dotenv", "encrypt", "--check", "--file", path])
        guard check.status != 0 else {
            return
        }

        let encrypt = runAv(binaryURL, arguments: ["dotenv", "encrypt", "--file", path])
        guard encrypt.status == 0 else {
            NSLog("Automic Vault dotenv auto-encrypt failed for %@: %@", path, encrypt.stderr)
            return
        }

        let keys = encryptedKeys(from: encrypt.stdout)
        guard keys.isEmpty == false else {
            return
        }

        DispatchQueue.main.async { [notify] in
            notify(DotenvAutoEncryptionResult(filePath: path, encryptedKeys: keys))
        }
    }

    private func runAv(_ binaryURL: URL, arguments: [String]) -> CommandResult {
        let process = Process()
        process.executableURL = binaryURL
        process.arguments = arguments

        let stdout = Pipe()
        let stderr = Pipe()
        process.standardOutput = stdout
        process.standardError = stderr

        do {
            try process.run()
            process.waitUntilExit()
        } catch {
            return CommandResult(status: 127, stdout: "", stderr: error.localizedDescription)
        }

        let stdoutData = stdout.fileHandleForReading.readDataToEndOfFile()
        let stderrData = stderr.fileHandleForReading.readDataToEndOfFile()
        return CommandResult(
            status: process.terminationStatus,
            stdout: String(data: stdoutData, encoding: .utf8) ?? "",
            stderr: String(data: stderrData, encoding: .utf8) ?? ""
        )
    }

    private func encryptedKeys(from stdout: String) -> [String] {
        let prefix = "encrypted "
        guard let line = stdout
            .split(separator: "\n", omittingEmptySubsequences: true)
            .first(where: { $0.hasPrefix(prefix) }) else {
            return []
        }
        return line
            .dropFirst(prefix.count)
            .split(separator: ",")
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { $0.isEmpty == false }
    }

    private func normalizedPath(_ path: String) -> String? {
        guard path.isEmpty == false else {
            return nil
        }
        return URL(fileURLWithPath: path).standardizedFileURL.path
    }

    private static func defaultBinaryURL() -> URL? {
        resolveBinaryURL(named: "av")
    }

    private static func resolveBinaryURL(named binaryName: String) -> URL? {
        if let bundled = Bundle.main.url(forResource: binaryName, withExtension: nil),
           FileManager.default.isExecutableFile(atPath: bundled.path) {
            return bundled
        }

        let repositoryRoot = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()

        let release = repositoryRoot.appendingPathComponent("target/release/\(binaryName)")
        if FileManager.default.isExecutableFile(atPath: release.path) {
            return release
        }

        let debug = repositoryRoot.appendingPathComponent("target/debug/\(binaryName)")
        if FileManager.default.isExecutableFile(atPath: debug.path) {
            return debug
        }

        let installed = URL(fileURLWithPath: "/usr/local/bin/\(binaryName)")
        if FileManager.default.isExecutableFile(atPath: installed.path) {
            return installed
        }

        return nil
    }
}
