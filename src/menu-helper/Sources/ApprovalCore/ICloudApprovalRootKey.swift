import Foundation
import Security

public enum ICloudApprovalRootKeyError: Error, Equatable {
    case unavailable(OSStatus)
    case invalidKey
    case randomGenerationFailed(OSStatus)
}

public struct ICloudApprovalRootKey: Sendable {
    public static let service = "com.automicvault.approval"
    public static let account = "account-root-key-v1"
    public static let accessGroup = "ZU76A67LGU.com.automicvault.approval"

    public static func hasActiveICloudAccount(
        identityToken: Any? = FileManager.default.ubiquityIdentityToken
    ) -> Bool {
        identityToken != nil
    }

    public init() {}

    public func loadOrCreate() throws -> Data {
        switch load() {
        case .success(let key): return key
        case .failure(.unavailable(errSecItemNotFound)):
            let key = try generate()
            let status = add(key)
            if status == errSecSuccess { return key }
            if status == errSecDuplicateItem, case .success(let winner) = load() { return winner }
            throw ICloudApprovalRootKeyError.unavailable(status)
        case .failure(let error): throw error
        }
    }

    public func rotate() throws -> Data {
        let key = try generate()
        let status = SecItemUpdate(primaryKey as CFDictionary, [
            kSecValueData as String: key,
        ] as CFDictionary)
        if status == errSecItemNotFound {
            let addStatus = add(key)
            guard addStatus == errSecSuccess else {
                throw ICloudApprovalRootKeyError.unavailable(addStatus)
            }
        } else if status != errSecSuccess {
            throw ICloudApprovalRootKeyError.unavailable(status)
        }
        guard case .success(let stored) = load(), stored == key else {
            throw ICloudApprovalRootKeyError.invalidKey
        }
        return key
    }

    private func load() -> Result<Data, ICloudApprovalRootKeyError> {
        var query = primaryKey
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        var result: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        guard status == errSecSuccess else { return .failure(.unavailable(status)) }
        guard let key = result as? Data, key.count == ApprovalCrypto.rootKeyByteCount else {
            return .failure(.invalidKey)
        }
        return .success(key)
    }

    private func add(_ key: Data) -> OSStatus {
        var query = primaryKey
        query[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlock
        query[kSecValueData as String] = key
        return SecItemAdd(query as CFDictionary, nil)
    }

    private var primaryKey: [String: Any] {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: Self.service,
            kSecAttrAccount as String: Self.account,
            kSecAttrAccessGroup as String: Self.accessGroup,
            kSecAttrSynchronizable as String: true,
        ]
#if os(macOS)
        query[kSecUseDataProtectionKeychain as String] = true
#endif
        return query
    }

    private func generate() throws -> Data {
        var data = Data(count: ApprovalCrypto.rootKeyByteCount)
        let status = data.withUnsafeMutableBytes {
            SecRandomCopyBytes(kSecRandomDefault, $0.count, $0.baseAddress!)
        }
        guard status == errSecSuccess else {
            throw ICloudApprovalRootKeyError.randomGenerationFailed(status)
        }
        return data
    }
}
