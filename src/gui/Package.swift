// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "AutomicVaultGUI",
    platforms: [
        .macOS(.v13),
    ],
    products: [
        .executable(name: "AutomicVaultApp", targets: ["AutomicVaultApp"]),
    ],
    dependencies: [
        .package(
            url: "https://github.com/mxcl/AppUpdater.git",
            from: "2.1.1"
        ),
    ],
    targets: [
        .executableTarget(
            name: "AutomicVaultApp",
            dependencies: [
                .product(name: "AppUpdater", package: "AppUpdater"),
            ],
            path: ".",
            exclude: [
                "AutomicVault.entitlements",
                "MenuBarAppDelegate.swift",
                "MenuBarHazardEffect.swift",
                "MenuBarMain.swift",
                "VaultDaemon.swift",
            ],
            sources: [
                "PackageModels.swift",
                "SecurityCatalog.swift",
                "NucleusBridge.swift",
                "NukeHelperBridge.swift",
                "HomebrewUpdateChecker.swift",
                "NucleusStatusStore.swift",
                "VaultApprovalStore.swift",
                "ContainmentLogStore.swift",
                "PostHogTelemetry.swift",
                "AppMain.swift",
                "AppDelegate.swift",
                "IsotopeApprovalView.swift",
                "AppUpdateCoordinator.swift",
                "PackageNodeHazardEffect.swift",
                "RootViewController.swift",
                "PackageFieldView.swift",
                "DossierView.swift",
                "ExternalSurfaceView.swift",
                "UpdateProgressViewController.swift",
                "ContainmentLogWindowController.swift",
                "UIStyle.swift",
            ]
        ),
    ],
    swiftLanguageModes: [.v5]
)
