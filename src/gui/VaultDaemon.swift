import AppKit
import Foundation
#if canImport(Darwin)
import Darwin
#endif

private struct VaultClientApprovalRequest: Codable {
    let id: String
    let intent: VaultExecutionIntent
}

private struct VaultClientContainmentSession: Codable {
    let id: String
    let pid: UInt32
    let agentID: String
    let command: String
    let args: [String]
    let cwd: String
    let initialExecutablePath: String
    let toolchainRoot: String
    let binDir: String
    let sandboxProfilePath: String
    let socketPath: String

    enum CodingKeys: String, CodingKey {
        case id
        case pid
        case agentID = "agent_id"
        case command
        case args
        case cwd
        case initialExecutablePath = "initial_executable_path"
        case toolchainRoot = "toolchain_root"
        case binDir = "bin_dir"
        case sandboxProfilePath = "sandbox_profile_path"
        case socketPath = "socket_path"
    }
}

private enum VaultClientRequest: Codable {
    case containmentStarted(VaultClientContainmentSession)
    case approvalRequest(VaultClientApprovalRequest)

    enum CodingKeys: String, CodingKey {
        case type
        case session
        case id
        case intent
    }

    enum RequestType: String, Codable {
        case containmentStarted = "containment_started"
        case approvalRequest = "approval_request"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let type = try container.decode(RequestType.self, forKey: .type)
        switch type {
        case .containmentStarted:
            self = .containmentStarted(
                try container.decode(VaultClientContainmentSession.self, forKey: .session)
            )
        case .approvalRequest:
            self = .approvalRequest(
                VaultClientApprovalRequest(
                    id: try container.decode(String.self, forKey: .id),
                    intent: try container.decode(VaultExecutionIntent.self, forKey: .intent)
                )
            )
        }
    }

    func encode(to encoder: Encoder) throws {
        switch self {
        case .containmentStarted(let session):
            var container = encoder.container(keyedBy: CodingKeys.self)
            try container.encode(RequestType.containmentStarted, forKey: .type)
            try container.encode(session, forKey: .session)
        case .approvalRequest(let request):
            var container = encoder.container(keyedBy: CodingKeys.self)
            try container.encode(RequestType.approvalRequest, forKey: .type)
            try container.encode(request.id, forKey: .id)
            try container.encode(request.intent, forKey: .intent)
        }
    }
}

private enum VaultDaemonEvent: Encodable {
    case approvalResponse(id: String, approved: Bool, reason: String?)
    case execChunk(id: String, stream: String, data: String)
    case execComplete(id: String, exitCode: Int32)
    case error(id: String?, code: Int, message: String)

    enum CodingKeys: String, CodingKey {
        case type
        case id
        case approved
        case reason
        case stream
        case data
        case exitCode = "exit_code"
        case code
        case message
    }

    enum EventType: String, Codable {
        case approvalResponse = "approval_response"
        case execChunk = "exec_chunk"
        case execComplete = "exec_complete"
        case error
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .approvalResponse(let id, let approved, let reason):
            try container.encode(EventType.approvalResponse, forKey: .type)
            try container.encode(id, forKey: .id)
            try container.encode(approved, forKey: .approved)
            try container.encodeIfPresent(reason, forKey: .reason)
        case .execChunk(let id, let stream, let data):
            try container.encode(EventType.execChunk, forKey: .type)
            try container.encode(id, forKey: .id)
            try container.encode(stream, forKey: .stream)
            try container.encode(data, forKey: .data)
        case .execComplete(let id, let exitCode):
            try container.encode(EventType.execComplete, forKey: .type)
            try container.encode(id, forKey: .id)
            try container.encode(exitCode, forKey: .exitCode)
        case .error(let id, let code, let message):
            try container.encode(EventType.error, forKey: .type)
            try container.encodeIfPresent(id, forKey: .id)
            try container.encode(code, forKey: .code)
            try container.encode(message, forKey: .message)
        }
    }
}

final class VaultDaemon {
    struct Configuration {
        let socketURL: URL
    }

    private let configuration: Configuration
    private let approvalStore = VaultApprovalStore()
    private let containmentLogStore = ContainmentLogStore()
    private let statusStore = NucleusStatusStore()
    private let queue = DispatchQueue(label: "com.automicvault.vault.daemon", qos: .userInitiated)
    private let encoder = JSONEncoder()
    private let decoder = JSONDecoder()
    private let activeRequestLock = NSLock()
    private let stateLock = NSLock()
    private var activeRequestID: String?
    private var listeningSocket: Int32 = -1
    private var shouldRun = false
    private let openMainWindow: () -> Void
    private let notifyUser: () -> Void

    init(
        configuration: Configuration = .default,
        openMainWindow: @escaping () -> Void,
        notifyUser: @escaping () -> Void
    ) {
        self.configuration = configuration
        self.openMainWindow = openMainWindow
        self.notifyUser = notifyUser
    }

    func start() {
        guard beginRunning() else { return }
        queue.async {
            do {
                try self.startServer()
            } catch {
                NSLog("vaultd failed to start: %@", error.localizedDescription)
                self.endRunning()
            }
        }
    }

    func stop() {
        let socket = stopRunning()
        if socket >= 0 {
            Darwin.close(socket)
        }
        try? FileManager.default.removeItem(at: configuration.socketURL)
    }

    private func startServer() throws {
        try FileManager.default.createDirectory(
            at: configuration.socketURL.deletingLastPathComponent(),
            withIntermediateDirectories: true,
            attributes: nil
        )
        try? FileManager.default.removeItem(at: configuration.socketURL)

        let socketFD = socket(AF_UNIX, SOCK_STREAM, 0)
        guard socketFD >= 0 else {
            throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno))
        }

        var address = sockaddr_un()
        address.sun_family = sa_family_t(AF_UNIX)
        let pathBytes = Array(configuration.socketURL.path.utf8)
        guard pathBytes.count < MemoryLayout.size(ofValue: address.sun_path) else {
            Darwin.close(socketFD)
            throw NSError(domain: NSPOSIXErrorDomain, code: Int(ENAMETOOLONG))
        }
        #if os(macOS)
        address.sun_len = UInt8(MemoryLayout<sockaddr_un>.size)
        #endif
        withUnsafeMutablePointer(to: &address.sun_path) { pathPointer in
            pathPointer.withMemoryRebound(to: CChar.self, capacity: pathBytes.count + 1) { buffer in
                _ = strncpy(buffer, configuration.socketURL.path, pathBytes.count)
                buffer[pathBytes.count] = 0
            }
        }

        var bindAddress = address
        let bindResult = withUnsafePointer(to: &bindAddress) { pointer in
            pointer.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockaddrPointer in
                bind(
                    socketFD,
                    sockaddrPointer,
                    socklen_t(MemoryLayout<sockaddr_un>.stride)
                )
            }
        }
        guard bindResult == 0 else {
            let code = errno
            Darwin.close(socketFD)
            throw NSError(domain: NSPOSIXErrorDomain, code: Int(code))
        }

        guard listen(socketFD, 8) == 0 else {
            let code = errno
            Darwin.close(socketFD)
            throw NSError(domain: NSPOSIXErrorDomain, code: Int(code))
        }

        setListeningSocket(socketFD)
        defer {
            clearListeningSocket(socketFD)
            Darwin.close(socketFD)
            try? FileManager.default.removeItem(at: configuration.socketURL)
            endRunning()
        }

        while isRunning {
            let clientFD = accept(socketFD, nil, nil)
            if clientFD < 0 {
                if errno == EINTR || isRunning == false {
                    continue
                }
                NSLog("vaultd accept failed: %d", errno)
                continue
            }
            handleClient(clientFD)
        }
    }

    private func handleClient(_ clientFD: Int32) {
        defer { Darwin.close(clientFD) }
        guard let line = readLine(from: clientFD) else {
            return
        }
        let data = Data(line.utf8)
        let request: VaultClientRequest
        do {
            request = try decoder.decode(VaultClientRequest.self, from: data)
        } catch {
            send(.error(id: nil, code: 400, message: "invalid request"), to: clientFD)
            return
        }

        switch request {
        case .containmentStarted(let session):
            processContainmentStarted(session)
        case .approvalRequest(let request):
            processApprovalRequest(request, clientFD: clientFD)
        }
    }

    private func processContainmentStarted(_ session: VaultClientContainmentSession) {
        let snapshot = VaultContainmentSessionSnapshot(
            id: session.id,
            pid: session.pid,
            agentID: session.agentID,
            command: session.command,
            args: session.args,
            cwd: session.cwd,
            initialExecutablePath: session.initialExecutablePath,
            toolchainRoot: session.toolchainRoot,
            binDir: session.binDir,
            sandboxProfilePath: session.sandboxProfilePath,
            socketPath: session.socketPath,
            startedAt: Date()
        )
        try? containmentLogStore.startSession(snapshot)
    }

    private func processApprovalRequest(
        _ request: VaultClientApprovalRequest,
        clientFD: Int32
    ) {
        guard beginRequest(id: request.id) else {
            send(.error(id: request.id, code: 409, message: "vaultd is already processing a request"), to: clientFD)
            return
        }
        defer { endRequest(id: request.id) }

        do {
            try approvalStore.savePendingApproval(
                VaultApprovalRequestSnapshot(id: request.id, intent: request.intent)
            )
            appendCommandLog(for: request)
            appendApprovalPendingLog(for: request)
            routeApprovalPresentation()
            let decision = waitForDecision(id: request.id)
                ?? VaultApprovalDecision(id: request.id, approved: false, reason: "approval unavailable")
            appendApprovalLog(for: request, decision: decision)
            send(
                .approvalResponse(
                    id: decision.id,
                    approved: decision.approved,
                    reason: decision.reason
                ),
                to: clientFD
            )
            guard decision.approved else {
                approvalStore.clearPendingApproval(id: request.id)
                return
            }
            execute(intent: request.intent, id: request.id, clientFD: clientFD)
            approvalStore.clearPendingApproval(id: request.id)
        } catch {
            approvalStore.clearPendingApproval(id: request.id)
            send(
                .error(
                    id: request.id,
                    code: 500,
                    message: error.localizedDescription
                ),
                to: clientFD
            )
        }
    }

    private func routeApprovalPresentation() {
        if NSRunningApplication.runningApplications(withBundleIdentifier: "com.automicvault").isEmpty {
            notifyUser()
        }
    }

    private func waitForDecision(id: String) -> VaultApprovalDecision? {
        while isRunning {
            if let decision = approvalStore.loadDecision(id: id) {
                return decision
            }
            Thread.sleep(forTimeInterval: 0.2)
        }
        return nil
    }

    private var isRunning: Bool {
        stateLock.lock()
        defer { stateLock.unlock() }
        return shouldRun
    }

    private func beginRunning() -> Bool {
        stateLock.lock()
        defer { stateLock.unlock() }
        guard shouldRun == false else { return false }
        shouldRun = true
        return true
    }

    private func endRunning() {
        stateLock.lock()
        shouldRun = false
        stateLock.unlock()
    }

    private func stopRunning() -> Int32 {
        stateLock.lock()
        defer { stateLock.unlock() }
        shouldRun = false
        let socket = listeningSocket
        listeningSocket = -1
        return socket
    }

    private func setListeningSocket(_ socket: Int32) {
        stateLock.lock()
        listeningSocket = socket
        stateLock.unlock()
    }

    private func clearListeningSocket(_ socket: Int32) {
        stateLock.lock()
        if listeningSocket == socket {
            listeningSocket = -1
        }
        stateLock.unlock()
    }

    private func execute(intent: VaultExecutionIntent, id: String, clientFD: Int32) {
        let process = Process()
        guard let executableURL = resolveExecutableURL(for: intent.tool) else {
            appendLog(
                sessionID: intent.agentID,
                kind: .error,
                title: "Could not resolve command",
                detail: intent.tool
            )
            send(.error(id: id, code: 404, message: "unable to resolve \(intent.tool)"), to: clientFD)
            return
        }

        process.executableURL = executableURL
        process.arguments = intent.args
        process.currentDirectoryURL = URL(fileURLWithPath: intent.cwd, isDirectory: true)
        process.environment = hostEnvironment(from: intent.env)

        let stdoutPipe = Pipe()
        let stderrPipe = Pipe()
        process.standardOutput = stdoutPipe
        process.standardError = stderrPipe

        stdoutPipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            guard let self else { return }
            let data = handle.availableData
            guard data.isEmpty == false else { return }
            self.sendChunk(data, stream: "stdout", id: id, clientFD: clientFD)
        }

        stderrPipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            guard let self else { return }
            let data = handle.availableData
            guard data.isEmpty == false else { return }
            self.sendChunk(data, stream: "stderr", id: id, clientFD: clientFD)
        }

        do {
            try process.run()
        } catch {
            stdoutPipe.fileHandleForReading.readabilityHandler = nil
            stderrPipe.fileHandleForReading.readabilityHandler = nil
            appendLog(
                sessionID: intent.agentID,
                kind: .error,
                title: "Could not launch command",
                detail: error.localizedDescription
            )
            send(.error(id: id, code: 500, message: error.localizedDescription), to: clientFD)
            return
        }

        process.waitUntilExit()
        stdoutPipe.fileHandleForReading.readabilityHandler = nil
        stderrPipe.fileHandleForReading.readabilityHandler = nil

        let remainingStdout = stdoutPipe.fileHandleForReading.readDataToEndOfFile()
        if remainingStdout.isEmpty == false {
            sendChunk(remainingStdout, stream: "stdout", id: id, clientFD: clientFD)
        }
        let remainingStderr = stderrPipe.fileHandleForReading.readDataToEndOfFile()
        if remainingStderr.isEmpty == false {
            sendChunk(remainingStderr, stream: "stderr", id: id, clientFD: clientFD)
        }

        send(.execComplete(id: id, exitCode: process.terminationStatus), to: clientFD)
    }

    private func appendCommandLog(for request: VaultClientApprovalRequest) {
        appendLog(
            sessionID: request.intent.agentID,
            kind: .command,
            title: commandLine(for: request.intent),
            detail: "cwd: \(request.intent.cwd)"
        )
    }

    private func appendApprovalLog(
        for request: VaultClientApprovalRequest,
        decision: VaultApprovalDecision
    ) {
        let title = decision.approved ? "Approved" : "Denied"
        appendLog(
            sessionID: request.intent.agentID,
            kind: .approval,
            title: title,
            detail: commandLine(for: request.intent)
        )
    }

    private func appendApprovalPendingLog(for request: VaultClientApprovalRequest) {
        appendLog(
            sessionID: request.intent.agentID,
            kind: .approval,
            title: "Approval requested",
            detail: commandLine(for: request.intent)
        )
    }

    private func appendLog(
        sessionID: String?,
        kind: VaultContainmentLogEntry.Kind,
        title: String,
        detail: String
    ) {
        guard let sessionID, sessionID.isEmpty == false else {
            return
        }
        try? containmentLogStore.append(
            sessionID: sessionID,
            kind: kind,
            title: title,
            detail: detail
        )
    }

    private func commandLine(for intent: VaultExecutionIntent) -> String {
        ([intent.tool] + intent.args).joined(separator: " ")
    }

    private func sendChunk(_ data: Data, stream: String, id: String, clientFD: Int32) {
        guard let chunk = String(data: data, encoding: .utf8), chunk.isEmpty == false else {
            return
        }
        send(.execChunk(id: id, stream: stream, data: chunk), to: clientFD)
    }

    private func send(_ event: VaultDaemonEvent, to clientFD: Int32) {
        guard let data = try? encoder.encode(event) else {
            return
        }
        _ = data.withUnsafeBytes { bytes in
            Darwin.write(clientFD, bytes.baseAddress, bytes.count)
        }
        _ = "\n".utf8CString.withUnsafeBytes { bytes in
            Darwin.write(clientFD, bytes.baseAddress, bytes.count - 1)
        }
    }

    private func readLine(from clientFD: Int32) -> String? {
        var data = Data()
        var byte: UInt8 = 0
        while true {
            let count = Darwin.read(clientFD, &byte, 1)
            if count <= 0 {
                break
            }
            if byte == 0x0A {
                break
            }
            data.append(byte)
        }
        guard data.isEmpty == false else {
            return nil
        }
        return String(data: data, encoding: .utf8)
    }

    private func beginRequest(id: String) -> Bool {
        activeRequestLock.lock()
        defer { activeRequestLock.unlock() }
        guard activeRequestID == nil else { return false }
        activeRequestID = id
        return true
    }

    private func endRequest(id: String) {
        activeRequestLock.lock()
        defer { activeRequestLock.unlock() }
        if activeRequestID == id {
            activeRequestID = nil
        }
    }

    private func resolveExecutableURL(for tool: String) -> URL? {
        let searchRoots = [
            "/usr/local/bin",
            "/opt/homebrew/bin",
            "/usr/bin",
            "/bin",
            "/usr/sbin",
            "/sbin"
        ]
        for root in searchRoots {
            let candidate = URL(fileURLWithPath: root, isDirectory: true)
                .appendingPathComponent(tool, isDirectory: false)
            if FileManager.default.isExecutableFile(atPath: candidate.path) {
                return candidate
            }
        }
        return nil
    }

    private func hostEnvironment(from captured: [String: String]) -> [String: String] {
        var environment = ProcessInfo.processInfo.environment
        for (key, value) in captured {
            environment[key] = value
        }
        environment["PATH"] = "/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin"
        environment.removeValue(forKey: "VAULT_SOCKET_PATH")
        environment.removeValue(forKey: "VAULT_TOOLCHAIN_ROOT")
        return environment
    }
}

private extension VaultDaemon.Configuration {
    static var `default`: Self {
        Self(
            socketURL: FileManager.default.homeDirectoryForCurrentUser
                .appendingPathComponent(
                    "Library/Application Support/Automic Vault",
                    isDirectory: true
                )
                .appendingPathComponent("vault.sock", isDirectory: false)
        )
    }
}
