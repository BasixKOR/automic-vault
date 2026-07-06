public func markdownDroppingInitialHeadingMarker(_ markdown: String) -> String {
    guard markdown.hasPrefix("# ") else { return markdown }
    return markdown.split(separator: "\n", maxSplits: 1, omittingEmptySubsequences: false)
        .dropFirst()
        .first
        .map(String.init) ?? ""
}
