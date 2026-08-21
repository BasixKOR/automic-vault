import Foundation
import Security

public let gpgDefaultPrivateKeySecretName = "AUTOMIC_GPG_SIGNING_PRIVATE_KEY"
public let gpgDefaultPassphraseSecretName = "AUTOMIC_GPG_SIGNING_PASSPHRASE"
public let gpgAlternatePrivateKeySecretName = "AUTOMIC_GPG_AGENT_SIGNING_PRIVATE_KEY"
public let gpgAlternatePassphraseSecretName = "AUTOMIC_GPG_AGENT_SIGNING_PASSPHRASE"

private let gpgSigningConfigurationService = "com.automicvault.gpg-signing-configuration"
private let gpgSigningConfigurationAccount = "configuration"

public struct GPGSigningConfiguration: Codable, Equatable, Sendable {
    public var alternateKeyLaunchers: [BlessedScriptLauncher]

    public init(alternateKeyLaunchers: [BlessedScriptLauncher] = []) {
        self.alternateKeyLaunchers = alternateKeyLaunchers
    }
}

public func loadGPGSigningConfiguration() -> GPGSigningConfiguration {
    loadGPGSigningConfiguration(
        service: gpgSigningConfigurationService,
        account: gpgSigningConfigurationAccount
    )
}

func loadGPGSigningConfiguration(service: String, account: String) -> GPGSigningConfiguration {
    guard case .success(let data) = loadKeychainDataResult(service: service, account: account),
          let configuration = try? JSONDecoder().decode(GPGSigningConfiguration.self, from: data)
    else { return GPGSigningConfiguration() }
    return configuration
}

@discardableResult
public func saveGPGSigningConfiguration(
    _ configuration: GPGSigningConfiguration
) -> OSStatus {
    saveGPGSigningConfiguration(
        configuration,
        service: gpgSigningConfigurationService,
        account: gpgSigningConfigurationAccount
    )
}

@discardableResult
func saveGPGSigningConfiguration(
    _ configuration: GPGSigningConfiguration,
    service: String,
    account: String
) -> OSStatus {
    guard let data = try? JSONEncoder().encode(configuration) else { return errSecParam }
    return saveKeychainData(
        data,
        service: service,
        account: account,
        accessibility: .whenUnlocked
    )
}

public func gpgSigningSecretNames(
    configuration: GPGSigningConfiguration,
    launcherRequirements: [String]
) -> [String] {
    let useAlternate = configuration.alternateKeyLaunchers.contains { launcher in
        launcherRequirements.contains(launcher.requirement)
    }
    return useAlternate
        ? [gpgAlternatePrivateKeySecretName, gpgAlternatePassphraseSecretName]
        : [gpgDefaultPrivateKeySecretName, gpgDefaultPassphraseSecretName]
}

@discardableResult
public func saveGPGSigningCredential(
    privateKey: String,
    passphrase: String,
    alternate: Bool
) -> OSStatus {
    let keyName = alternate ? gpgAlternatePrivateKeySecretName : gpgDefaultPrivateKeySecretName
    let passphraseName = alternate
        ? gpgAlternatePassphraseSecretName : gpgDefaultPassphraseSecretName
    let keyStatus = saveStoredSecret(account: keyName, value: privateKey)
    guard keyStatus == errSecSuccess else { return keyStatus }
    return saveStoredSecret(account: passphraseName, value: passphrase)
}

public func hasGPGSigningCredential(alternate: Bool) -> Bool {
    let name = alternate ? gpgAlternatePrivateKeySecretName : gpgDefaultPrivateKeySecretName
    return loadStoredSecrets().contains { $0.account == name }
}

public func configureGitForGPGSigning(
    gitURL: URL = URL(fileURLWithPath: "/usr/bin/git"),
    programURL: URL,
    environment: [String: String]? = nil
) throws {
    guard programURL.path.hasPrefix("/"), FileManager.default.isExecutableFile(atPath: programURL.path)
    else { throw GPGSigningConfigurationError.programUnavailable(programURL.path) }
    for arguments in [
        ["config", "--global", "gpg.program", programURL.path],
        ["config", "--global", "commit.gpgSign", "true"],
        ["config", "--global", "gpg.format", "openpgp"],
    ] {
        let process = Process()
        process.executableURL = gitURL
        process.arguments = arguments
        if let environment { process.environment = environment }
        let errorPipe = Pipe()
        process.standardError = errorPipe
        try process.run()
        process.waitUntilExit()
        guard process.terminationStatus == 0 else {
            let detail = String(
                decoding: errorPipe.fileHandleForReading.readDataToEndOfFile(),
                as: UTF8.self
            ).trimmingCharacters(in: .whitespacesAndNewlines)
            throw GPGSigningConfigurationError.gitFailed(detail)
        }
    }
}

public enum GPGSigningConfigurationError: LocalizedError {
    case programUnavailable(String)
    case gitFailed(String)

    public var errorDescription: String? {
        switch self {
        case .programUnavailable(let path): "The bundled Git signing adapter is unavailable at \(path)."
        case .gitFailed(let detail): "Git configuration failed\(detail.isEmpty ? "." : ": \(detail)")"
        }
    }
}
