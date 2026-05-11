import AppKit

final class IsotopeApprovalView: NSView {
    private enum Metrics {
        static let width: CGFloat = 640
        static let height: CGFloat = 314
        static let panelRadius: CGFloat = 9
        static let labelWidth: CGFloat = 130
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
        static let red = NSColor(calibratedRed: 0.82, green: 0.36, blue: 0.38, alpha: 1)
    }

    private let approval: IsotopeApprovalRequestSnapshot

    override var intrinsicContentSize: NSSize {
        NSSize(width: Metrics.width, height: Metrics.height)
    }

    init(approval: IsotopeApprovalRequestSnapshot) {
        self.approval = approval
        super.init(frame: NSRect(x: 0, y: 0, width: Metrics.width, height: Metrics.height))
        translatesAutoresizingMaskIntoConstraints = false
        build()
    }

    required init?(coder: NSCoder) {
        nil
    }

    private func build() {
        let root = NSStackView()
        root.orientation = .vertical
        root.alignment = .width
        root.distribution = .fill
        root.spacing = 9
        root.translatesAutoresizingMaskIntoConstraints = false
        addSubview(root)

        let secrets = secretsPanel()
        let command = commandPanel()
        let requester = requesterPanel()
        let target = targetPanel()

        root.addArrangedSubview(secrets)
        root.addArrangedSubview(command)
        root.addArrangedSubview(requester)
        root.addArrangedSubview(target)

        NSLayoutConstraint.activate([
            root.leadingAnchor.constraint(equalTo: leadingAnchor),
            root.trailingAnchor.constraint(equalTo: trailingAnchor),
            root.topAnchor.constraint(equalTo: topAnchor),
            root.bottomAnchor.constraint(equalTo: bottomAnchor),
            secrets.heightAnchor.constraint(equalToConstant: 62),
            command.heightAnchor.constraint(equalToConstant: 66),
            requester.heightAnchor.constraint(equalToConstant: 42)
        ])
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

    private func targetPanel() -> NSView {
        let scriptPath = displayScriptPath
        var rows: [InfoRow] = [
            InfoRow("Requested executable", abbreviatedPath(requestedExecutablePath), nil),
            InfoRow("Audited executable", abbreviatedPath(approval.executablePath), rootStatus(approval.executableRootControlled))
        ]

        if isInterpreter {
            rows.append(InfoRow(
                "Interpreter script",
                scriptPath.map(abbreviatedPath) ?? "No script file detected; flags or inline code are in use",
                scriptPath.map { _ in rootStatus(scriptRootControlled) }
                    ?? Status(title: "not a script", color: Palette.amber)
            ))
        } else {
            rows.append(InfoRow("Invocation type", "Direct executable; no interpreter script detected", nil))
        }

        rows.append(InfoRow(
            "Always allow",
            approval.canAlwaysAllow
                ? alwaysAllowDescription
                : "Manual approval only; not every executable boundary is root-controlled",
            approval.canAlwaysAllow
                ? Status(title: "root-controlled", color: Palette.accent)
                : Status(title: "manual only", color: Palette.amber)
        ))

        return sectionPanel(title: "Execution target", rows: rows)
    }

    private func secretsPanel() -> NSView {
        let view = makePanel()
        let title = sectionTitle("Secrets")
        let note = label(
            "Names only. Values stay in Keychain until this one child process is approved.",
            size: 10,
            weight: .regular,
            color: Palette.quietText
        )
        note.maximumNumberOfLines = 1
        note.lineBreakMode = .byTruncatingTail

        let keyStack = NSStackView()
        keyStack.orientation = .horizontal
        keyStack.alignment = .centerY
        keyStack.spacing = 6
        keyStack.distribution = .fill
        keyStack.translatesAutoresizingMaskIntoConstraints = false
        for key in approval.keys {
            keyStack.addArrangedSubview(pill(key, color: Palette.accent, monospaced: true))
        }

        [title, keyStack, note].forEach {
            $0.translatesAutoresizingMaskIntoConstraints = false
            view.addSubview($0)
        }

        NSLayoutConstraint.activate([
            title.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: Metrics.innerPadding),
            title.topAnchor.constraint(equalTo: view.topAnchor, constant: 9),

            keyStack.leadingAnchor.constraint(equalTo: title.trailingAnchor, constant: 16),
            keyStack.centerYAnchor.constraint(equalTo: title.centerYAnchor),
            keyStack.trailingAnchor.constraint(lessThanOrEqualTo: view.trailingAnchor, constant: -Metrics.innerPadding),

            note.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: Metrics.innerPadding),
            note.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -Metrics.innerPadding),
            note.topAnchor.constraint(equalTo: title.bottomAnchor, constant: 8)
        ])

        return view
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

    private func sectionPanel(title: String, rows: [InfoRow]) -> NSView {
        let view = makePanel()
        let titleLabel = sectionTitle(title)
        let rowStack = NSStackView()
        rowStack.orientation = .vertical
        rowStack.alignment = .width
        rowStack.distribution = .fillEqually
        rowStack.spacing = 4
        rowStack.translatesAutoresizingMaskIntoConstraints = false
        rows.forEach { rowStack.addArrangedSubview(infoRow($0)) }

        [titleLabel, rowStack].forEach {
            $0.translatesAutoresizingMaskIntoConstraints = false
            view.addSubview($0)
        }

        NSLayoutConstraint.activate([
            titleLabel.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: Metrics.innerPadding),
            titleLabel.topAnchor.constraint(equalTo: view.topAnchor, constant: 10),

            rowStack.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: Metrics.innerPadding),
            rowStack.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -Metrics.innerPadding),
            rowStack.topAnchor.constraint(equalTo: titleLabel.bottomAnchor, constant: 8),
            rowStack.bottomAnchor.constraint(equalTo: view.bottomAnchor, constant: -Metrics.innerPadding)
        ])

        return view
    }

    private func infoRow(_ row: InfoRow) -> NSView {
        let view = NSView()
        view.translatesAutoresizingMaskIntoConstraints = false

        let title = label(row.title.uppercased(), size: 9, weight: .medium, color: Palette.quietText, monospaced: true)
        let value = label(row.value, size: 11, weight: .regular, color: Palette.text, monospaced: row.value.contains("/"))
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

    private func pill(
        _ text: String,
        color: NSColor,
        monospaced: Bool = false
    ) -> NSView {
        let container = NSView()
        container.wantsLayer = true
        container.layer?.backgroundColor = color.withAlphaComponent(0.12).cgColor
        container.layer?.borderColor = color.withAlphaComponent(0.36).cgColor
        container.layer?.borderWidth = 1
        container.layer?.cornerRadius = 8
        container.translatesAutoresizingMaskIntoConstraints = false

        let field = label(text, size: 10, weight: .medium, color: color, monospaced: monospaced)
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
        let parentName = approval.parentProcess.displayName
            ?? approval.parentProcess.executablePath
            ?? "unknown process"
        let result = NSMutableAttributedString()
        result.append(bold(parentName))
        result.append(plain("; pid "))
        result.append(bold("\(approval.parentProcess.pid)"))
        result.append(plain("; cwd: "))
        result.append(code(abbreviatedPath(approval.cwd)))
        return result
    }

    private var requestedExecutablePath: String {
        approval.requestedExecutablePath ?? approval.executablePath
    }

    private var commandLine: String {
        ([requestedExecutablePath] + approval.argv).joined(separator: " ")
    }

    private var displayCommandLine: String {
        abbreviatedPath(commandLine)
    }

    private var isInterpreter: Bool {
        Self.isScriptInterpreter(URL(fileURLWithPath: approval.executablePath).lastPathComponent)
    }

    private var displayScriptPath: String? {
        approval.requestedScriptPath ?? approval.scriptPath ?? Self.scriptPathFromArguments(
            approval.argv,
            cwd: approval.cwd,
            executablePath: approval.executablePath
        )
    }

    private var scriptRootControlled: Bool? {
        approval.scriptRootControlled
            ?? ((approval.scriptPath != nil && approval.scriptPath == displayScriptPath) ? true : nil)
    }

    private var alwaysAllowDescription: String {
        if displayScriptPath != nil {
            return "Available for this root-controlled interpreter and script"
        }
        return "Available for this root-controlled executable"
    }

    private func rootStatus(_ value: Bool?) -> Status {
        switch value {
        case .some(true):
            return Status(title: "root-controlled", color: Palette.accent)
        case .some(false):
            return Status(title: "not root-controlled", color: Palette.red)
        case .none:
            return Status(title: "not verified", color: Palette.amber)
        }
    }

    private static func isScriptInterpreter(_ name: String) -> Bool {
        if name.hasPrefix("python"),
           name.dropFirst("python".count).allSatisfy({ $0 == "." || $0.isNumber }) {
            return name.count > "python".count
        }
        return [
            "bash", "dash", "env", "ksh", "node", "osascript", "perl",
            "python", "python3", "ruby", "sh", "zsh"
        ].contains(name)
    }

    private static func scriptPathFromArguments(
        _ args: [String],
        cwd: String,
        executablePath: String
    ) -> String? {
        guard isScriptInterpreter(URL(fileURLWithPath: executablePath).lastPathComponent),
              URL(fileURLWithPath: executablePath).lastPathComponent != "env" else {
            return nil
        }

        var index = 0
        while index < args.count {
            let arg = args[index]
            if arg == "--" {
                return pathForDisplay(args[safe: index + 1], cwd: cwd)
            }
            if arg == "-" || arg.hasPrefix("-") == false {
                return pathForDisplay(arg, cwd: cwd)
            }
            index += optionTakesValue(arg) ? 2 : 1
        }
        return nil
    }

    private static func optionTakesValue(_ arg: String) -> Bool {
        [
            "-c", "-m", "-S", "-e", "-I", "-l", "-x", "-C", "-M", "-d", "-r"
        ].contains(arg)
    }

    private static func pathForDisplay(_ value: String?, cwd: String) -> String? {
        guard let value else { return nil }
        if value.hasPrefix("/") {
            return value
        }
        return URL(fileURLWithPath: cwd).appendingPathComponent(value).path
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

        init(_ title: String, _ value: String, _ status: Status?) {
            self.title = title
            self.value = value
            self.status = status
        }
    }

    private struct Status {
        let title: String
        let color: NSColor
    }
}

private extension Array {
    subscript(safe index: Index) -> Element? {
        indices.contains(index) ? self[index] : nil
    }
}
