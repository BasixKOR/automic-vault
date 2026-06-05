import Foundation

struct DotenvAutoEncryptionResult: Equatable {
    let filePath: String
    let encryptedKeys: [String]
}

final class DotenvFileWatcher {
    private struct Watch {
        let source: DispatchSourceFileSystemObject
    }

    private struct FileSignature: Equatable {
        let modificationTime: TimeInterval
        let size: UInt64
    }

    private struct CommandResult {
        let status: Int32
        let stdout: String
        let stderr: String
    }

    private struct DotenvAssignment {
        let key: String
        let value: String
    }

    private let queue = DispatchQueue(label: "com.automicvault.dotenv-file-watcher")
    private let fileManager: FileManager
    private let notify: (DotenvAutoEncryptionResult) -> Void
    private let resolveBinaryURL: () -> URL?
    private var watches: [String: Watch] = [:]
    private var signatures: [String: FileSignature] = [:]
    private var pendingWork: [String: DispatchWorkItem] = [:]
    private var isPolling = false

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
            isPolling = false
        }
    }

    private func watchUnlocked(path: String) {
        guard let path = normalizedPath(path), watches[path] == nil else {
            return
        }
        guard fileManager.fileExists(atPath: path) else {
            return
        }
        guard let signature = fileSignature(path: path) else {
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
        signatures[path] = signature
        source.resume()
        startPollingUnlocked()
    }

    private func cancelWatchUnlocked(path: String) {
        guard let watch = watches.removeValue(forKey: path) else {
            return
        }
        signatures.removeValue(forKey: path)
        watch.source.cancel()
        if watches.isEmpty {
            stopPollingUnlocked()
        }
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

    private func startPollingUnlocked() {
        guard isPolling == false else {
            return
        }
        isPolling = true
        scheduleNextPollUnlocked()
    }

    private func scheduleNextPollUnlocked() {
        queue.asyncAfter(deadline: .now() + 1.0) { [weak self] in
            self?.pollWatchedFiles()
        }
    }

    private func stopPollingUnlocked() {
        isPolling = false
    }

    private func pollWatchedFiles() {
        guard isPolling else {
            return
        }
        for path in Array(watches.keys) {
            guard let signature = fileSignature(path: path) else {
                cancelWatchUnlocked(path: path)
                continue
            }
            if signatures[path] != signature {
                signatures[path] = signature
                scheduleEncryptionCheck(path: path)
            }
        }
        if watches.isEmpty {
            stopPollingUnlocked()
        } else {
            scheduleNextPollUnlocked()
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
        guard fileManager.fileExists(atPath: path) else {
            return
        }
        guard let contents = try? String(contentsOfFile: path, encoding: .utf8) else {
            NSLog("Automic Vault dotenv auto-encrypt skipped for %@: file is not UTF-8", path)
            return
        }
        let secretKeys = Self.secretShapedPlaintextKeys(in: contents)
        guard secretKeys.isEmpty == false else {
            return
        }
        guard let binaryURL = resolveBinaryURL() else {
            NSLog("Automic Vault dotenv auto-encrypt skipped for %@: av binary not found", path)
            return
        }

        let check = runAv(
            binaryURL,
            arguments: encryptArguments(path: path, keys: secretKeys, check: true)
        )
        guard check.status != 0 else {
            return
        }

        let encrypt = runAv(
            binaryURL,
            arguments: encryptArguments(path: path, keys: secretKeys, check: false)
        )
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

    private func encryptArguments(path: String, keys: [String], check: Bool) -> [String] {
        var arguments = ["dotenv", "encrypt"]
        if check {
            arguments.append("--check")
        }
        arguments.append(contentsOf: ["--file", path, "--key"])
        arguments.append(contentsOf: keys)
        return arguments
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

    static func secretShapedPlaintextKeys(in contents: String) -> [String] {
        contents
            .split(separator: "\n", omittingEmptySubsequences: false)
            .compactMap(Self.parseDotenvAssignment)
            .filter { assignment in
                isPublicDotenvKey(assignment.key) == false
                    && assignment.value.hasPrefix("encrypted:") == false
                    && assignment.value.isEmpty == false
                    && isSecretShaped(key: assignment.key, value: assignment.value)
            }
            .map(\.key)
    }

    private static func parseDotenvAssignment(_ rawLine: Substring) -> DotenvAssignment? {
        var assignment = rawLine.trimmingCharacters(in: .whitespaces)
        guard assignment.isEmpty == false, assignment.hasPrefix("#") == false else {
            return nil
        }
        if assignment.hasPrefix("export ") {
            assignment.removeFirst("export ".count)
        }
        guard let separator = dotenvAssignmentSeparator(in: assignment) else {
            return nil
        }
        let key = assignment[..<separator]
            .trimmingCharacters(in: .whitespacesAndNewlines)
        guard isValidDotenvKey(key) else {
            return nil
        }
        let valueStart = assignment.index(after: separator)
        let value = parseDotenvValue(assignment[valueStart...])
        return DotenvAssignment(key: key, value: value)
    }

    private static func dotenvAssignmentSeparator(in assignment: String) -> String.Index? {
        let equals = assignment.firstIndex(of: "=")
        let colon = assignment.indices.first { index in
            guard assignment[index] == ":" else { return false }
            let next = assignment.index(after: index)
            return next < assignment.endIndex && assignment[next].isWhitespace
        }
        switch (equals, colon) {
        case (.some(let equals), .some(let colon)):
            return equals < colon ? equals : colon
        case (.some(let equals), .none):
            return equals
        case (.none, .some(let colon)):
            return colon
        case (.none, .none):
            return nil
        }
    }

    private static func parseDotenvValue(_ rawValue: Substring) -> String {
        let value = rawValue.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let first = value.first else {
            return ""
        }
        if first == "'" || first == "\"" || first == "`" {
            return parseQuotedDotenvValue(value, quote: first)
        }
        return value
            .split(separator: "#", maxSplits: 1, omittingEmptySubsequences: false)
            .first?
            .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
    }

    private static func parseQuotedDotenvValue(_ value: String, quote: Character) -> String {
        var escaped = false
        var endIndex: String.Index?
        var index = value.index(after: value.startIndex)
        while index < value.endIndex {
            let character = value[index]
            if escaped {
                escaped = false
            } else if character == "\\", quote != "'" {
                escaped = true
            } else if character == quote {
                endIndex = index
                break
            }
            index = value.index(after: index)
        }
        let innerEnd = endIndex ?? value.endIndex
        var inner = String(value[value.index(after: value.startIndex)..<innerEnd])
        if quote == "\"" {
            inner = inner
                .replacingOccurrences(of: "\\n", with: "\n")
                .replacingOccurrences(of: "\\r", with: "\r")
                .replacingOccurrences(of: "\\t", with: "\t")
                .replacingOccurrences(of: "\\\"", with: "\"")
                .replacingOccurrences(of: "\\\\", with: "\\")
        }
        return inner
    }

    private static func isValidDotenvKey(_ key: String) -> Bool {
        guard let first = key.unicodeScalars.first,
              first == "_" || isAsciiLetter(first) else {
            return false
        }
        return key.unicodeScalars.dropFirst().allSatisfy { scalar in
            scalar == "_" || isAsciiLetter(scalar) || isAsciiDigit(scalar)
        }
    }

    private static func isAsciiLetter(_ scalar: UnicodeScalar) -> Bool {
        (65...90).contains(scalar.value) || (97...122).contains(scalar.value)
    }

    private static func isAsciiDigit(_ scalar: UnicodeScalar) -> Bool {
        (48...57).contains(scalar.value)
    }

    private static func isPublicDotenvKey(_ key: String) -> Bool {
        key == "DOTENV_PUBLIC_KEY" || key.hasPrefix("DOTENV_PUBLIC_KEY_")
    }

    private static func isSecretShaped(key: String, value: String) -> Bool {
        keyLooksSecret(key, value: value) || valueLooksSecret(value)
    }

    private static func keyLooksSecret(_ key: String, value: String) -> Bool {
        let upper = key.uppercased()
        if upper.hasPrefix("NEXT_PUBLIC_")
            || upper.hasPrefix("NUXT_PUBLIC_")
            || upper.hasPrefix("PUBLIC_")
            || upper.hasPrefix("VITE_")
            || upper.contains("PUBLISHABLE")
            || upper.contains("PUBLIC_KEY") {
            return false
        }
        if [
            "_ENDPOINT",
            "_HOST",
            "_PORT",
            "_URI",
            "_URL",
            "_VERSION",
            "_ENABLED",
        ].contains(where: upper.hasSuffix) {
            return valueLooksSecret(value)
        }

        let tokens = upper
            .split { $0.isLetter == false && $0.isNumber == false }
            .map(String.init)
        let tokenSet = Set(tokens)
        if tokenSet.contains("SECRET")
            || tokenSet.contains("TOKEN")
            || tokenSet.contains("PASSWORD")
            || tokenSet.contains("PASSWD") {
            return true
        }
        if tokenSet.contains("KEY")
            && !tokenSet.isDisjoint(with: [
                "ACCESS", "API", "AWS", "GITHUB", "GITLAB", "NPM", "OPENAI", "PRIVATE", "SECRET",
                "STRIPE",
            ]) {
            return true
        }

        let compact = upper.filter { $0.isLetter || $0.isNumber }
        if [
            "APIKEY",
            "ACCESSTOKEN",
            "AUTH_TOKEN",
            "BEARERTOKEN",
            "CLIENTSECRET",
            "PRIVATEKEY",
            "REFRESHTOKEN",
            "SECRETKEY",
            "SESSIONSECRET",
            "SIGNINGSECRET",
            "WEBHOOKSECRET",
        ].contains(where: { compact.contains($0.filter { $0 != "_" }) }) {
            return true
        }

        if (upper.hasSuffix("_URL") || upper.hasSuffix("_DSN"))
            && valueLooksCredentialURL(value) {
            return true
        }
        return false
    }

    private static func valueLooksSecret(_ value: String) -> Bool {
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.isEmpty == false, trimmed.hasPrefix("encrypted:") == false else {
            return false
        }
        if trimmed.contains("-----BEGIN ") && trimmed.contains("PRIVATE KEY-----") {
            return true
        }
        if valueLooksCredentialURL(trimmed) {
            return true
        }
        if matches(trimmed, #"^eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$"#) {
            return true
        }
        if [
            "sk-",
            "sk_live_",
            "sk_test_",
            "ghp_",
            "gho_",
            "ghu_",
            "ghs_",
            "github_pat_",
            "glpat-",
            "xoxb-",
            "xoxp-",
            "xapp-",
        ].contains(where: trimmed.hasPrefix) {
            return true
        }
        if matches(trimmed, #"^(AKIA|ASIA)[A-Z0-9]{16}$"#) {
            return true
        }
        return valueHasHighEntropySecretShape(trimmed)
    }

    private static func valueLooksCredentialURL(_ value: String) -> Bool {
        matches(value, #"^[A-Za-z][A-Za-z0-9+.-]*://[^/\s:@]+:[^/\s@]+@"#)
    }

    private static func valueHasHighEntropySecretShape(_ value: String) -> Bool {
        guard value.count >= 32,
              value.rangeOfCharacter(from: .whitespacesAndNewlines) == nil,
              value.hasPrefix("/") == false,
              value.hasPrefix("~/") == false,
              value.contains("://") == false,
              matches(value, #"^[0-9a-fA-F-]{32,}$"#) == false else {
            return false
        }
        let scalars = value.unicodeScalars
        let hasLower = scalars.contains { CharacterSet.lowercaseLetters.contains($0) }
        let hasUpper = scalars.contains { CharacterSet.uppercaseLetters.contains($0) }
        let hasDigit = scalars.contains { CharacterSet.decimalDigits.contains($0) }
        let hasSymbol = scalars.contains {
            CharacterSet.alphanumerics.contains($0) == false
        }
        let classCount = [hasLower, hasUpper, hasDigit, hasSymbol].filter { $0 }.count
        let uniqueCount = Set(scalars.map(\.value)).count
        return classCount >= 3 && uniqueCount >= 16
    }

    private static func matches(_ value: String, _ pattern: String) -> Bool {
        value.range(of: pattern, options: .regularExpression) != nil
    }

    private func normalizedPath(_ path: String) -> String? {
        guard path.isEmpty == false else {
            return nil
        }
        return URL(fileURLWithPath: path).standardizedFileURL.path
    }

    private func fileSignature(path: String) -> FileSignature? {
        guard let attributes = try? fileManager.attributesOfItem(atPath: path) else {
            return nil
        }
        let modificationTime = (attributes[.modificationDate] as? Date)?.timeIntervalSince1970 ?? 0
        let size = (attributes[.size] as? NSNumber)?.uint64Value ?? 0
        return FileSignature(modificationTime: modificationTime, size: size)
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

        return nil
    }
}
