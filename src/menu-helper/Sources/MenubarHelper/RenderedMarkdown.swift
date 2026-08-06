import MarkdownUI
import MenubarHelperCore
import SwiftUI

struct RenderedMarkdown: View {
    let markdown: String

    var body: some View {
        Markdown(markdownDroppingInitialHeadingMarker(markdown))
            .markdownTheme(.basic.listItem { configuration in
                configuration.label.markdownMargin(top: .em(0.15))
            })
            .textSelection(.enabled)
    }
}
