import Foundation

public enum UpdatePreflightError: Error {
    case invalidArguments
    case invalidDraft
}

public struct UpdatePreflightInput: Sendable {
    public static let releasesURL = URL(
        string: "https://api.github.com/repos/automic-vault/automic-vault/releases"
    )!

    public let expectedVersion: String
    public let releasesData: Data?
    public let assetURL: URL?
    public let assetFileURL: URL?
    public let assetSize: Int64?

    public init(arguments: [String]) throws {
        guard let flag = arguments.firstIndex(of: "--verify-update"),
              flag == 1,
              arguments.count == 3 || arguments.count == 5,
              Self.validVersion(arguments[2])
        else { throw UpdatePreflightError.invalidArguments }

        let expectedVersion = arguments[2]
        self.expectedVersion = expectedVersion
        guard arguments.count == 5 else {
            releasesData = nil
            assetURL = nil
            assetFileURL = nil
            assetSize = nil
            return
        }

        let releasesFile = URL(fileURLWithPath: arguments[3])
        let assetFile = URL(fileURLWithPath: arguments[4])
        let releasesValues = try releasesFile.resourceValues(
            forKeys: [.fileSizeKey, .isRegularFileKey, .isSymbolicLinkKey]
        )
        let assetValues = try assetFile.resourceValues(
            forKeys: [.fileSizeKey, .isRegularFileKey, .isSymbolicLinkKey]
        )
        guard releasesValues.isRegularFile == true,
              releasesValues.isSymbolicLink != true,
              let releasesSize = releasesValues.fileSize,
              releasesSize > 0,
              releasesSize <= 10 * 1024 * 1024,
              assetValues.isRegularFile == true,
              assetValues.isSymbolicLink != true,
              let assetSize = assetValues.fileSize,
              assetSize > 0
        else { throw UpdatePreflightError.invalidDraft }

        let data = try Data(contentsOf: releasesFile, options: .mappedIfSafe)
        let releases = try JSONDecoder().decode([Release].self, from: data)
        let expectedName = "Automic-Vault-\(expectedVersion).dmg"
        let matching = releases.filter { $0.tagName == expectedVersion && $0.draft }
        guard matching.count == 1,
              matching[0].targetCommitish.isFullGitCommit,
              matching[0].assets.count(where: { $0.name == expectedName }) == 1,
              let asset = matching[0].assets.first(where: { $0.name == expectedName }),
              asset.size == Int64(assetSize),
              asset.digest.range(of: #"^sha256:[0-9a-f]{64}$"#, options: .regularExpression) != nil,
              asset.browserDownloadURL.scheme == "https",
              asset.browserDownloadURL.host == "github.com",
              asset.browserDownloadURL.path.hasPrefix(
                "/automic-vault/automic-vault/releases/download/"
              )
        else { throw UpdatePreflightError.invalidDraft }

        releasesData = data
        assetURL = asset.browserDownloadURL
        assetFileURL = assetFile
        self.assetSize = Int64(assetSize)
    }

    public func fixture(for url: URL) -> (data: Data?, file: URL?, size: Int64)? {
        if url == Self.releasesURL, let releasesData {
            return (releasesData, nil, Int64(releasesData.count))
        }
        if url == assetURL, let assetFileURL, let assetSize {
            return (nil, assetFileURL, assetSize)
        }
        return nil
    }

    private static func validVersion(_ value: String) -> Bool {
        value.range(of: #"^[0-9]+\.[0-9]+\.[0-9]+$"#, options: .regularExpression) != nil
    }
}

private struct Release: Decodable {
    let tagName: String
    let targetCommitish: String
    let draft: Bool
    let assets: [Asset]

    enum CodingKeys: String, CodingKey {
        case tagName = "tag_name"
        case targetCommitish = "target_commitish"
        case draft
        case assets
    }

    struct Asset: Decodable {
        let name: String
        let browserDownloadURL: URL
        let size: Int64
        let digest: String

        enum CodingKeys: String, CodingKey {
            case name
            case browserDownloadURL = "browser_download_url"
            case size
            case digest
        }
    }
}

private extension String {
    var isFullGitCommit: Bool {
        range(of: #"^[0-9a-f]{40}$"#, options: .regularExpression) != nil
    }
}
