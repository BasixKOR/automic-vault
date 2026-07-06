public func markdownDroppingInitialHeadingMarker(_ markdown: String) -> String {
    markdown.hasPrefix("# ") ? String(markdown.dropFirst(2)) : markdown
}
