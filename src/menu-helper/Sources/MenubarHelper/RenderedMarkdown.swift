import MarkdownUI
import MenubarHelperCore
import SwiftUI

struct RenderedMarkdown: View {
    let markdown: String

    var body: some View {
        Markdown(markdownDroppingInitialHeadingMarker(markdown))
            .textSelection(.enabled)
    }
}
