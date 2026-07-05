import MarkdownUI
import SwiftUI

struct RenderedMarkdown: View {
    let markdown: String

    var body: some View {
        Markdown(markdown)
            .textSelection(.enabled)
    }
}
