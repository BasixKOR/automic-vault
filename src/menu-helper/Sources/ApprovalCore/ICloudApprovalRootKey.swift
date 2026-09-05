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
        try loadOrCreateApprovalRootKey(read: read, generate: generate, add: add)
    }

    public func load() throws -> Data {
        switch read() {
        case .success(let key): key
        case .failure(let error): throw error
        }
    }

    public func rotate() throws -> Data {
        try rotateApprovalRootKey(
            read: read,
            generate: generate,
            update: { key in
                SecItemUpdate(self.primaryKey as CFDictionary, [
                    kSecValueData as String: key,
                ] as CFDictionary)
            },
            add: add
        )
    }

    private func read() -> Result<Data, ICloudApprovalRootKeyError> {
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

func loadOrCreateApprovalRootKey(
    read: () -> Result<Data, ICloudApprovalRootKeyError>,
    generate: () throws -> Data,
    add: (Data) -> OSStatus
) throws -> Data {
    switch read() {
    case .success(let key): return key
    case .failure(.unavailable(errSecItemNotFound)):
        let key = try generate()
        let status = add(key)
        if status == errSecSuccess { return key }
        if status == errSecDuplicateItem, case .success(let winner) = read() { return winner }
        throw ICloudApprovalRootKeyError.unavailable(status)
    case .failure(let error): throw error
    }
}

func rotateApprovalRootKey(
    read: () -> Result<Data, ICloudApprovalRootKeyError>,
    generate: () throws -> Data,
    update: (Data) -> OSStatus,
    add: (Data) -> OSStatus
) throws -> Data {
    let key = try generate()
    let status = update(key)
    if status == errSecItemNotFound {
        let addStatus = add(key)
        guard addStatus == errSecSuccess else {
            throw ICloudApprovalRootKeyError.unavailable(addStatus)
        }
    } else if status != errSecSuccess {
        throw ICloudApprovalRootKeyError.unavailable(status)
    }
    guard case .success(let stored) = read(), stored == key else {
        throw ICloudApprovalRootKeyError.invalidKey
    }
    return key
}

public enum ApprovalNotificationPreferencesError: Error, Equatable {
    case unavailable(OSStatus)
    case invalidData
}

public struct ApprovalNotificationPreferences: Equatable, Sendable {
    public var showsHost: Bool
    public var showsApprovalType: Bool

    public init(showsHost: Bool = false, showsApprovalType: Bool = false) {
        self.showsHost = showsHost
        self.showsApprovalType = showsApprovalType
    }

    public static func load() throws -> Self {
        var query = primaryKey
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        var result: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        if status == errSecItemNotFound { return Self() }
        guard status == errSecSuccess else {
            throw ApprovalNotificationPreferencesError.unavailable(status)
        }
        guard let data = result as? Data, data.count == 1, let value = data.first else {
            throw ApprovalNotificationPreferencesError.invalidData
        }
        return Self(showsHost: value & 1 != 0, showsApprovalType: value & 2 != 0)
    }

    public func save() throws {
        let value = Data([(showsHost ? 1 : 0) | (showsApprovalType ? 2 : 0)])
        var status = SecItemUpdate(
            Self.primaryKey as CFDictionary,
            [kSecValueData as String: value] as CFDictionary
        )
        if status == errSecItemNotFound {
            var query = Self.primaryKey
            query[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
            query[kSecValueData as String] = value
            status = SecItemAdd(query as CFDictionary, nil)
        }
        guard status == errSecSuccess else {
            throw ApprovalNotificationPreferencesError.unavailable(status)
        }
    }

    private static var primaryKey: [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: "com.automicvault.approval.preferences",
            kSecAttrAccount as String: "notification-details-v1",
            kSecAttrAccessGroup as String: ICloudApprovalRootKey.accessGroup,
            kSecAttrSynchronizable as String: false,
        ]
    }
}

public enum PhoneApprovalActivityInboxError: Error, Equatable {
    case unavailable(OSStatus)
    case invalidData
}

public enum PhoneApprovalActivityInbox {
    private static let service = "com.automicvault.approval.activity-inbox"

    public static func load() throws -> [PhoneApprovalActivity] {
        var query = baseQuery
        query[kSecReturnAttributes as String] = true
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitAll
        var result: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        if status == errSecItemNotFound { return [] }
        guard status == errSecSuccess else {
            throw PhoneApprovalActivityInboxError.unavailable(status)
        }
        guard let rows = result as? [[String: Any]] else {
            throw PhoneApprovalActivityInboxError.invalidData
        }
        let decoder = JSONDecoder()
        return try rows.map { row in
            guard let data = row[kSecValueData as String] as? Data else {
                throw PhoneApprovalActivityInboxError.invalidData
            }
            return try decoder.decode(PhoneApprovalActivity.self, from: data)
        }.sorted { $0.respondedAtMilliseconds > $1.respondedAtMilliseconds }
    }

    public static func save(_ item: PhoneApprovalActivity) throws {
        let query = itemQuery(item.id)
        let data = try JSONEncoder().encode(item)
        var status = SecItemUpdate(
            query as CFDictionary,
            [kSecValueData as String: data] as CFDictionary
        )
        if status == errSecItemNotFound {
            var addQuery = query
            addQuery[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
            addQuery[kSecValueData as String] = data
            status = SecItemAdd(addQuery as CFDictionary, nil)
            if status == errSecDuplicateItem {
                status = SecItemUpdate(
                    query as CFDictionary,
                    [kSecValueData as String: data] as CFDictionary
                )
            }
        }
        guard status == errSecSuccess else {
            throw PhoneApprovalActivityInboxError.unavailable(status)
        }
        try remove(Array(load().dropFirst(PhoneApprovalActivity.maximumItems)))
    }

    public static func remove(_ items: [PhoneApprovalActivity]) throws {
        for item in items {
            let status = SecItemDelete(itemQuery(item.id) as CFDictionary)
            guard status == errSecSuccess || status == errSecItemNotFound else {
                throw PhoneApprovalActivityInboxError.unavailable(status)
            }
        }
    }

    private static var baseQuery: [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccessGroup as String: ICloudApprovalRootKey.accessGroup,
            kSecAttrSynchronizable as String: false,
        ]
    }

    private static func itemQuery(_ requestID: UUID) -> [String: Any] {
        var query = baseQuery
        query[kSecAttrAccount as String] = requestID.uuidString.lowercased()
        return query
    }
}
