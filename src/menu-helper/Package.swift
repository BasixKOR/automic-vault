// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "AutomicVaultMenubar",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "AutomicVaultMenubar", targets: ["MenubarHelper"]),
    ],
    targets: [
        .target(name: "CProcessInfo", publicHeadersPath: "include"),
        .executableTarget(
            name: "MenubarHelper",
            dependencies: ["CProcessInfo"]
        ),
    ]
)
