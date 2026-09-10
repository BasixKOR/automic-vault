import Foundation

/// Translate app-owned presentation labels only. Never pass Secret Names, paths,
/// commands, user content, or values used for policy, persistence, or transport.
func localizedUIString(_ english: String) -> String {
    Bundle.main.localizedString(forKey: english, value: english, table: nil)
}
