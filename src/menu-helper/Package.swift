// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "AutomicVaultMenubar",
    platforms: [.macOS("26.0")],
    products: [
        .executable(name: "AutomicVaultMenubar", targets: ["MenubarHelper"]),
    ],
    dependencies: [
        .package(url: "https://github.com/mxcl/AppUpdater.git", from: "3.0.6"),
        .package(url: "https://github.com/GraphQLSwift/GraphQL.git", from: "4.2.0"),
        .package(url: "https://github.com/gonzalezreal/swift-markdown-ui", from: "2.4.1"),
    ],
    targets: [
        .target(
            name: "MenubarHelperCore",
            dependencies: [
                .product(name: "GraphQL", package: "GraphQL"),
            ]
        ),
        .target(name: "CProcessInfo", publicHeadersPath: "include"),
        .executableTarget(
            name: "MenubarHelper",
            dependencies: [
                "AppUpdater",
                "CProcessInfo",
                "MenubarHelperCore",
                .product(name: "MarkdownUI", package: "swift-markdown-ui"),
            ]
        ),
        .testTarget(
            name: "MenubarHelperCoreTests",
            dependencies: ["MenubarHelperCore"]
        ),
    ]
)
