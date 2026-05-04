import AppKit

@main
enum AutomicVaultMenuMain {
    static func main() {
        let application = NSApplication.shared
        let delegate = MenuBarAppDelegate()
        application.setActivationPolicy(.accessory)
        application.delegate = delegate
        application.run()
    }
}
