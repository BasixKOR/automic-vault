import AppKit

final class CommandExecutionApprovalView: NSView {
    private enum Metrics {
        static let width: CGFloat = 640
        static let height: CGFloat = 224
        static let panelRadius: CGFloat = 9
        static let labelWidth: CGFloat = 112
        static let innerPadding: CGFloat = 11
    }

    private enum Palette {
        static let panel = NSColor(calibratedWhite: 0.115, alpha: 1)
        static let panelRaised = NSColor(calibratedWhite: 0.145, alpha: 1)
        static let stroke = NSColor(calibratedWhite: 1, alpha: 0.10)
        static let strongText = NSColor(calibratedWhite: 0.92, alpha: 1)
        static let text = NSColor(calibratedWhite: 0.78, alpha: 1)
        static let dimText = NSColor(calibratedWhite: 0.60, alpha: 1)
        static let quietText = NSColor(calibratedWhite: 0.44, alpha: 1)
        static let accent = NSColor(calibratedRed: 0.44, green: 0.78, blue: 0.60, alpha: 1)
        static let amber = NSColor(calibratedRed: 0.84, green: 0.64, blue: 0.30, alpha: 1)
    }

    private let approval: VaultApprovalRequestSnapshot

    override var intrinsicContentSize: NSSize {
        let extraRows = max(environmentRowCount - 2, 0)
        return NSSize(
            width: Metrics.width,
            height: Metrics.height + CGFloat(extraRows * 23)
        )
    }

    init(approval: VaultApprovalRequestSnapshot) {
        self.approval = approval
        super.init(frame: NSRect(x: 0, y: 0, width: Metrics.width, height: Metrics.height))
        translatesAutoresizingMaskIntoConstraints = false
        build()
    }

    required init?(coder: NSCoder) {
        nil
    }

    private func build() {
        let command = commandPanel()
        let requester = requesterPanel()
        let environment = environmentPanel()

        [command, requester, environment].forEach {
            $0.translatesAutoresizingMaskIntoConstraints = false
            addSubview($0)
        }

        NSLayoutConstraint.activate([
            command.leadingAnchor.constraint(equalTo: leadingAnchor),
            command.trailingAnchor.constraint(equalTo: trailingAnchor),
            command.topAnchor.constraint(equalTo: topAnchor),
            command.heightAnchor.constraint(equalToConstant: 78),

            requester.leadingAnchor.constraint(equalTo: leadingAnchor),
            requester.trailingAnchor.constraint(equalTo: trailingAnchor),
            requester.topAnchor.constraint(equalTo: command.bottomAnchor, constant: 9),
            requester.heightAnchor.constraint(equalToConstant: 42),

            environment.leadingAnchor.constraint(equalTo: leadingAnchor),
            environment.trailingAnchor.constraint(equalTo: trailingAnchor),
            environment.topAnchor.constraint(equalTo: requester.bottomAnchor, constant: 9),
            environment.bottomAnchor.constraint(equalTo: bottomAnchor)
        ])
    }

    private func commandPanel() -> NSView {
        let view = makePanel()
        let title = sectionTitle("Command")
        let commandBox = makeCommandBox()
        let commandText = label(displayCommandLine, size: 10, weight: .regular, color: Palette.text, monospaced: true)
        commandText.maximumNumberOfLines = 2
        commandText.lineBreakMode = .byTruncatingMiddle

        [title, commandBox].forEach {
            $0.translatesAutoresizingMaskIntoConstraints = false
            view.addSubview($0)
        }
        commandText.translatesAutoresizingMaskIntoConstraints = false
        commandBox.addSubview(commandText)

        NSLayoutConstraint.activate([
            title.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: Metrics.innerPadding),
            title.topAnchor.constraint(equalTo: view.topAnchor, constant: 9),

            commandBox.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: Metrics.innerPadding),
            commandBox.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -Metrics.innerPadding),
            commandBox.topAnchor.constraint(equalTo: title.bottomAnchor, constant: 8),
            commandBox.bottomAnchor.constraint(equalTo: view.bottomAnchor, constant: -Metrics.innerPadding),

            commandText.leadingAnchor.constraint(equalTo: commandBox.leadingAnchor, constant: 9),
            commandText.trailingAnchor.constraint(equalTo: commandBox.trailingAnchor, constant: -9),
            commandText.centerYAnchor.constraint(equalTo: commandBox.centerYAnchor)
        ])

        return view
    }

    private func requesterPanel() -> NSView {
        let view = makePanel()
        let title = label("REQUESTED BY", size: 9, weight: .semibold, color: Palette.quietText, monospaced: true, tracking: 0.9)
        let summary = attributedLabel(requesterSummary)
        summary.lineBreakMode = .byTruncatingMiddle
        summary.maximumNumberOfLines = 1

        [title, summary].forEach {
            $0.translatesAutoresizingMaskIntoConstraints = false
            view.addSubview($0)
        }

        NSLayoutConstraint.activate([
            title.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: Metrics.innerPadding),
            title.centerYAnchor.constraint(equalTo: view.centerYAnchor),
            title.widthAnchor.constraint(equalToConstant: 92),

            summary.leadingAnchor.constraint(equalTo: title.trailingAnchor, constant: 10),
            summary.centerYAnchor.constraint(equalTo: view.centerYAnchor),
            summary.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -Metrics.innerPadding)
        ])

        return view
    }

    private func environmentPanel() -> NSView {
        let rows = environmentRows
        return sectionPanel(title: "Environment", rows: rows)
    }

    private func sectionPanel(title: String, rows: [InfoRow]) -> NSView {
        let view = makePanel()
        let titleLabel = sectionTitle(title)
        let rowViews = rows.map(infoRow)

        ([titleLabel] + rowViews).forEach {
            $0.translatesAutoresizingMaskIntoConstraints = false
            view.addSubview($0)
        }

        var constraints = [
            titleLabel.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: Metrics.innerPadding),
            titleLabel.topAnchor.constraint(equalTo: view.topAnchor, constant: 10)
        ]

        for (index, rowView) in rowViews.enumerated() {
            constraints.append(contentsOf: [
                rowView.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: Metrics.innerPadding),
                rowView.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -Metrics.innerPadding),
                rowView.heightAnchor.constraint(equalToConstant: 19)
            ])
            if index == 0 {
                constraints.append(rowView.topAnchor.constraint(equalTo: titleLabel.bottomAnchor, constant: 8))
            } else {
                constraints.append(rowView.topAnchor.constraint(equalTo: rowViews[index - 1].bottomAnchor, constant: 4))
            }
        }

        if let lastRow = rowViews.last {
            constraints.append(lastRow.bottomAnchor.constraint(lessThanOrEqualTo: view.bottomAnchor, constant: -Metrics.innerPadding))
        }

        NSLayoutConstraint.activate(constraints)
        return view
    }

    private func infoRow(_ row: InfoRow) -> NSView {
        let view = NSView()
        view.translatesAutoresizingMaskIntoConstraints = false

        let title = label(row.title.uppercased(), size: 9, weight: .medium, color: Palette.quietText, monospaced: true)
        let value = label(row.value, size: 11, weight: .regular, color: Palette.text, monospaced: row.monospaced)
        value.maximumNumberOfLines = 1
        value.lineBreakMode = .byTruncatingMiddle
        value.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)

        [title, value].forEach {
            $0.translatesAutoresizingMaskIntoConstraints = false
            view.addSubview($0)
        }

        var constraints = [
            title.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            title.centerYAnchor.constraint(equalTo: view.centerYAnchor),
            title.widthAnchor.constraint(equalToConstant: Metrics.labelWidth),
            value.leadingAnchor.constraint(equalTo: title.trailingAnchor, constant: 10),
            value.centerYAnchor.constraint(equalTo: view.centerYAnchor)
        ]

        if let status = row.status {
            let statusPill = pill(status.title, color: status.color)
            statusPill.translatesAutoresizingMaskIntoConstraints = false
            view.addSubview(statusPill)
            constraints.append(contentsOf: [
                value.trailingAnchor.constraint(lessThanOrEqualTo: statusPill.leadingAnchor, constant: -8),
                statusPill.trailingAnchor.constraint(equalTo: view.trailingAnchor),
                statusPill.centerYAnchor.constraint(equalTo: view.centerYAnchor)
            ])
        } else {
            constraints.append(value.trailingAnchor.constraint(equalTo: view.trailingAnchor))
        }

        NSLayoutConstraint.activate(constraints)
        return view
    }

    private func makePanel() -> NSView {
        let view = NSView()
        view.wantsLayer = true
        view.layer?.backgroundColor = Palette.panel.cgColor
        view.layer?.cornerRadius = Metrics.panelRadius
        view.layer?.borderWidth = 1
        view.layer?.borderColor = Palette.stroke.cgColor
        view.translatesAutoresizingMaskIntoConstraints = false
        return view
    }

    private func makeCommandBox() -> NSView {
        let view = NSView()
        view.wantsLayer = true
        view.layer?.backgroundColor = Palette.panelRaised.cgColor
        view.layer?.cornerRadius = 6
        view.layer?.borderWidth = 1
        view.layer?.borderColor = Palette.stroke.cgColor
        return view
    }

    private func sectionTitle(_ text: String) -> NSTextField {
        label(text.uppercased(), size: 9, weight: .semibold, color: Palette.dimText, monospaced: true, tracking: 0.9)
    }

    private func label(
        _ text: String,
        size: CGFloat,
        weight: NSFont.Weight,
        color: NSColor,
        monospaced: Bool = false,
        tracking: CGFloat = 0
    ) -> NSTextField {
        let field = NSTextField(wrappingLabelWithString: text)
        field.font = monospaced
            ? UIStyle.monoFont(size: size, weight: weight)
            : NSFont.systemFont(ofSize: size, weight: weight)
        field.textColor = color
        field.allowsDefaultTighteningForTruncation = false
        field.maximumNumberOfLines = 1
        field.setContentHuggingPriority(.defaultHigh, for: .vertical)
        if tracking != 0, let font = field.font {
            field.attributedStringValue = NSAttributedString(
                string: text,
                attributes: [
                    .font: font,
                    .foregroundColor: color,
                    .kern: tracking
                ]
            )
        }
        return field
    }

    private func attributedLabel(_ value: NSAttributedString) -> NSTextField {
        let field = NSTextField(wrappingLabelWithString: "")
        field.attributedStringValue = value
        field.allowsDefaultTighteningForTruncation = false
        field.maximumNumberOfLines = 1
        field.setContentHuggingPriority(.defaultHigh, for: .vertical)
        field.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        return field
    }

    private func pill(_ text: String, color: NSColor) -> NSView {
        let container = NSView()
        container.wantsLayer = true
        container.layer?.backgroundColor = color.withAlphaComponent(0.12).cgColor
        container.layer?.borderColor = color.withAlphaComponent(0.36).cgColor
        container.layer?.borderWidth = 1
        container.layer?.cornerRadius = 8
        container.translatesAutoresizingMaskIntoConstraints = false

        let field = label(text, size: 10, weight: .medium, color: color)
        field.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(field)

        NSLayoutConstraint.activate([
            field.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 7),
            field.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -7),
            field.topAnchor.constraint(equalTo: container.topAnchor, constant: 2),
            field.bottomAnchor.constraint(equalTo: container.bottomAnchor, constant: -2)
        ])

        return container
    }

    private var requesterSummary: NSAttributedString {
        let agent = approval.intent.agentID ?? "unknown agent"
        let result = NSMutableAttributedString()
        result.append(bold(agent))
        result.append(plain("; cwd: "))
        result.append(code(abbreviatedPath(approval.intent.cwd)))
        return result
    }

    private var displayCommandLine: String {
        abbreviatedPath(([approval.intent.tool] + approval.intent.args).joined(separator: " "))
    }

    private var environmentRows: [InfoRow] {
        guard approval.intent.env.isEmpty == false else {
            return [InfoRow("Overrides", "No explicit environment overrides", nil, false)]
        }

        return approval.intent.env.keys.sorted().map { key in
            InfoRow(
                key,
                abbreviatedPath(approval.intent.env[key] ?? ""),
                nil,
                true
            )
        }
    }

    private var environmentRowCount: Int {
        max(approval.intent.env.count, 1)
    }

    private func abbreviatedPath(_ value: String) -> String {
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        if value == home {
            return "~"
        }
        let homePrefix = home + "/"
        if value.hasPrefix(homePrefix) {
            return "~/" + String(value.dropFirst(homePrefix.count))
        }
        return value.replacingOccurrences(of: homePrefix, with: "~/")
    }

    private func plain(_ value: String) -> NSAttributedString {
        NSAttributedString(
            string: value,
            attributes: [
                .font: NSFont.systemFont(ofSize: 12, weight: .regular),
                .foregroundColor: Palette.text
            ]
        )
    }

    private func bold(_ value: String) -> NSAttributedString {
        NSAttributedString(
            string: value,
            attributes: [
                .font: NSFont.systemFont(ofSize: 12, weight: .semibold),
                .foregroundColor: Palette.strongText
            ]
        )
    }

    private func code(_ value: String) -> NSAttributedString {
        NSAttributedString(
            string: value,
            attributes: [
                .font: UIStyle.monoFont(size: 11, weight: .medium),
                .foregroundColor: Palette.text
            ]
        )
    }

    private struct InfoRow {
        let title: String
        let value: String
        let status: Status?
        let monospaced: Bool

        init(_ title: String, _ value: String, _ status: Status?, _ monospaced: Bool = true) {
            self.title = title
            self.value = value
            self.status = status
            self.monospaced = monospaced
        }
    }

    private struct Status {
        let title: String
        let color: NSColor
    }
}
