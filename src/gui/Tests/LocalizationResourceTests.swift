import XCTest

final class LocalizationResourceTests: XCTestCase {
    func testAllSupportedLocalizationsShareTheSameKeys() throws {
        let resourcesURL = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .appendingPathComponent("Resources", isDirectory: true)
        let languages = ["en", "ja", "de", "fr", "zh-Hans"]

        let keyedResources = try Dictionary(
            uniqueKeysWithValues: languages.map { language in
                (language, try Self.localizationKeys(language: language, resourcesURL: resourcesURL))
            }
        )
        let englishKeys = try XCTUnwrap(keyedResources["en"])

        for language in languages where language != "en" {
            let localizedKeys = try XCTUnwrap(keyedResources[language])
            XCTAssertEqual(
                localizedKeys,
                englishKeys,
                "\(language) Localizable.strings must match the English key set"
            )
        }
    }

    private static func localizationKeys(
        language: String,
        resourcesURL: URL
    ) throws -> Set<String> {
        let url = resourcesURL
            .appendingPathComponent("\(language).lproj", isDirectory: true)
            .appendingPathComponent("Localizable.strings", isDirectory: false)
        let dictionary = try XCTUnwrap(NSDictionary(contentsOf: url) as? [String: String])
        return Set(dictionary.keys)
    }
}
