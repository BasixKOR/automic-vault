import AppKit
import AppUpdater

@MainActor
final class AppUpdateCoordinator {
    enum InstallReadiness {
        case ready
        case busy(String)
    }

    private static let checkInterval: TimeInterval = 60 * 60

    private let updater = AppUpdater(owner: "automic-vault", repo: "automic-vault")
    private let statusStore: NucleusStatusStore
    private var availableUpdate: Update?
    private var checkTask: Task<Void, Never>?
    private var checkTimer: Timer?
    private var installInFlight = false

    var onStateChange: (() -> Void)?
    var onError: ((String) -> Void)?

    init(statusStore: NucleusStatusStore) {
        self.statusStore = statusStore
    }

    var hasAvailableUpdate: Bool {
        availableUpdate != nil
    }

    var isInstalling: Bool {
        installInFlight
    }

    func startAutomaticChecks() {
        checkForUpdates()
        let timer = Timer(
            timeInterval: Self.checkInterval,
            repeats: true
        ) { [weak self] _ in
            Task { @MainActor in
                self?.checkForUpdates()
            }
        }
        checkTimer = timer
        RunLoop.main.add(timer, forMode: .common)
    }

    func stop() {
        checkTimer?.invalidate()
        checkTimer = nil
        checkTask?.cancel()
    }

    func checkForUpdates() {
        guard checkTask == nil else { return }
        checkTask = Task { [weak self] in
            guard let self else { return }
            defer { checkTask = nil }

            do {
                availableUpdate = try await updater.check()
                publishState(error: nil)
            } catch is CancellationError {
            } catch {
                publishState(error: error.localizedDescription)
            }
        }
    }

    func installWhenReady(
        readiness: @escaping () -> InstallReadiness,
        prepareForInstall: @escaping () -> Void
    ) {
        guard installInFlight == false else { return }

        guard let update = availableUpdate else {
            checkForUpdates()
            return
        }

        switch readiness() {
        case .ready:
            install(update, prepareForInstall: prepareForInstall)
        case .busy(let reason):
            publishState(error: reason)
            onError?(reason)
        }
    }

    private func install(
        _ update: Update,
        prepareForInstall: @escaping () -> Void
    ) {
        installInFlight = true
        publishState(error: nil)

        Task { [weak self] in
            guard let self else { return }
            do {
                prepareForInstall()
                try await update.installAndRelaunch()
            } catch {
                installInFlight = false
                publishState(error: error.localizedDescription)
                onError?(error.localizedDescription)
            }
        }
    }

    private func publishState(error: String?) {
        try? statusStore.saveAppUpdateSnapshot(
            AppUpdateSnapshot(
                updateAvailable: availableUpdate != nil,
                updatedAt: Date(),
                lastError: error
            )
        )
        onStateChange?()
    }
}
