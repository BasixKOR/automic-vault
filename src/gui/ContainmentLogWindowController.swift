import AppKit

final class ContainmentLogWindowController: NSWindowController {
    private let sessionID: String
    private let titleLabel = NSTextField(labelWithString: "")
    private let subtitleLabel = NSTextField(labelWithString: "")
    private let detailLabel = NSTextField(labelWithString: "")
    private let logView = NSTextView(frame: .zero)
    private let dateFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateStyle = .none
        formatter.timeStyle = .medium
        return formatter
    }()

    init(snapshot: VaultContainmentLogSnapshot) {
        sessionID = snapshot.session.id
        let contentView = NSView(frame: NSRect(x: 0, y: 0, width: 760, height: 560))
        let window = NSWindow(
            contentRect: contentView.frame,
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )
        window.title = "Contained Entity"
        window.backgroundColor = UIStyle.background
        window.isOpaque = true
        window.isReleasedWhenClosed = false
        window.contentView = contentView
        super.init(window: window)
        configureContentView(contentView)
        apply(snapshot: snapshot)
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func matches(sessionID: String) -> Bool {
        self.sessionID == sessionID
    }

    func apply(snapshot: VaultContainmentLogSnapshot) {
        titleLabel.stringValue = snapshot.session.agentID
        subtitleLabel.stringValue = ([snapshot.session.command] + snapshot.session.args)
            .joined(separator: " ")
        detailLabel.stringValue = [
            "PID \(snapshot.session.pid)",
            snapshot.session.cwd,
            snapshot.session.initialExecutablePath
        ].joined(separator: "  |  ")
        logView.string = snapshot.entries.map(render(entry:)).joined(separator: "\n\n")
        logView.scrollToEndOfDocument(nil)
    }

    private func configureContentView(_ contentView: NSView) {
        titleLabel.font = NSFont.systemFont(ofSize: 18, weight: .semibold)
        titleLabel.textColor = UIStyle.text
        titleLabel.lineBreakMode = .byTruncatingMiddle

        subtitleLabel.font = UIStyle.monoFont(size: 12, weight: .regular)
        subtitleLabel.textColor = UIStyle.dimText
        subtitleLabel.lineBreakMode = .byTruncatingMiddle

        detailLabel.font = NSFont.systemFont(ofSize: 11, weight: .regular)
        detailLabel.textColor = UIStyle.dimText
        detailLabel.lineBreakMode = .byTruncatingMiddle

        let scrollView = NSScrollView(frame: .zero)
        scrollView.hasVerticalScroller = true
        scrollView.borderType = .noBorder
        scrollView.drawsBackground = false
        logView.isEditable = false
        logView.isSelectable = true
        logView.isRichText = false
        logView.drawsBackground = false
        logView.textColor = UIStyle.text
        logView.font = UIStyle.monoFont(size: 12, weight: .regular)
        logView.textContainerInset = NSSize(width: 14, height: 14)
        scrollView.documentView = logView

        [titleLabel, subtitleLabel, detailLabel, scrollView].forEach {
            $0.translatesAutoresizingMaskIntoConstraints = false
            contentView.addSubview($0)
        }

        NSLayoutConstraint.activate([
            titleLabel.topAnchor.constraint(equalTo: contentView.topAnchor, constant: 24),
            titleLabel.leadingAnchor.constraint(equalTo: contentView.leadingAnchor, constant: 24),
            titleLabel.trailingAnchor.constraint(equalTo: contentView.trailingAnchor, constant: -24),

            subtitleLabel.topAnchor.constraint(equalTo: titleLabel.bottomAnchor, constant: 8),
            subtitleLabel.leadingAnchor.constraint(equalTo: titleLabel.leadingAnchor),
            subtitleLabel.trailingAnchor.constraint(equalTo: titleLabel.trailingAnchor),

            detailLabel.topAnchor.constraint(equalTo: subtitleLabel.bottomAnchor, constant: 6),
            detailLabel.leadingAnchor.constraint(equalTo: titleLabel.leadingAnchor),
            detailLabel.trailingAnchor.constraint(equalTo: titleLabel.trailingAnchor),

            scrollView.topAnchor.constraint(equalTo: detailLabel.bottomAnchor, constant: 18),
            scrollView.leadingAnchor.constraint(equalTo: contentView.leadingAnchor),
            scrollView.trailingAnchor.constraint(equalTo: contentView.trailingAnchor),
            scrollView.bottomAnchor.constraint(equalTo: contentView.bottomAnchor)
        ])
    }

    private func render(entry: VaultContainmentLogEntry) -> String {
        let time = dateFormatter.string(from: entry.createdAt)
        let detail = entry.detail.isEmpty ? "" : "\n\(entry.detail)"
        return "[\(time)] \(label(for: entry.kind)) \(entry.title)\(detail)"
    }

    private func label(for kind: VaultContainmentLogEntry.Kind) -> String {
        switch kind {
        case .sessionStarted:
            return "session"
        case .command:
            return "command"
        case .approval:
            return "approval"
        case .completion:
            return "complete"
        case .error:
            return "error"
        }
    }
}
