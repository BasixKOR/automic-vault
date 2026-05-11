import AppKit

final class IsotopeApprovalView: NSView {
    private enum Palette {
        static let background = NSColor(calibratedWhite: 0.075, alpha: 1)
        static let panel = NSColor(calibratedWhite: 0.105, alpha: 1)
        static let panelRaised = NSColor(calibratedWhite: 0.13, alpha: 1)
        static let stroke = NSColor(calibratedWhite: 1, alpha: 0.09)
        static let strongText = NSColor(calibratedWhite: 0.94, alpha: 1)
        static let text = NSColor(calibratedWhite: 0.82, alpha: 1)
        static let dimText = NSColor(calibratedWhite: 0.64, alpha: 1)
        static let quietText = NSColor(calibratedWhite: 0.48, alpha: 1)
        static let accent = NSColor(calibratedRed: 0.44, green: 0.78, blue: 0.60, alpha: 1)
        static let amber = NSColor(calibratedRed: 0.86, green: 0.65, blue: 0.27, alpha: 1)
        static let red = NSColor(calibratedRed: 0.86, green: 0.34, blue: 0.36, alpha: 1)
    }

    private let approval: IsotopeApprovalRequestSnapshot

    init(approval: IsotopeApprovalRequestSnapshot) {
        self.approval = approval
        super.init(frame: NSRect(x: 0, y: 0, width: 760, height: 510))
        wantsLayer = true
        layer?.backgroundColor = Palette.background.cgColor
        layer?.cornerRadius = 18
        layer?.borderWidth = 1
        layer?.borderColor = Palette.stroke.cgColor
        build()
    }

    required init?(coder: NSCoder) {
        nil
    }

    private func build() {
        let root = NSStackView()
        root.orientation = .vertical
        root.spacing = 14
        root.edgeInsets = NSEdgeInsets(top: 22, left: 22, bottom: 22, right: 22)
        root.translatesAutoresizingMaskIntoConstraints = false
        addSubview(root)

        NSLayoutConstraint.activate([
            root.leadingAnchor.constraint(equalTo: leadingAnchor),
            root.trailingAnchor.constraint(equalTo: trailingAnchor),
            root.topAnchor.constraint(equalTo: topAnchor),
            root.bottomAnchor.constraint(equalTo: bottomAnchor)
        ])

        root.addArrangedSubview(headerView())
        root.addArrangedSubview(requesterView())
        root.addArrangedSubview(targetView())
        root.addArrangedSubview(secretsView())
        root.addArrangedSubview(commandView())
    }

    private func headerView() -> NSView {
        let view = NSView()
        view.translatesAutoresizingMaskIntoConstraints = false

        let stack = NSStackView()
        stack.orientation = .horizontal
        stack.alignment = .top
        stack.spacing = 18
        stack.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(stack)

        let titleStack = NSStackView()
        titleStack.orientation = .vertical
        titleStack.spacing = 5
        titleStack.addArrangedSubview(label("Key injection request", size: 22, weight: .semibold, color: Palette.strongText))
        titleStack.addArrangedSubview(label(headerSummary, size: 12, weight: .regular, color: Palette.dimText))

        stack.addArrangedSubview(titleStack)
        stack.addArrangedSubview(spacer())
        stack.addArrangedSubview(pill(
            "\(approval.keys.count) secret\(approval.keys.count == 1 ? "" : "s") requested",
            color: Palette.accent,
            fillAlpha: 0.12
        ))

        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            stack.topAnchor.constraint(equalTo: view.topAnchor),
            stack.bottomAnchor.constraint(equalTo: view.bottomAnchor)
        ])
        return view
    }

    private func requesterView() -> NSView {
        let parentName = approval.parentProcess.displayName
            ?? approval.parentProcess.executablePath
            ?? "unknown process"
        let parentPath = approval.parentProcess.executablePath ?? "unknown executable"
        return panel(
            title: "Requesting process",
            rows: [
                ("Process", "\(parentName)  pid \(approval.parentProcess.pid)", nil),
                ("Executable", parentPath, nil),
                ("Working directory", approval.cwd, nil)
            ]
        )
    }

    private func targetView() -> NSView {
        let scriptPath = displayScriptPath
        var rows: [(String, String, Status?)] = [
            ("Requested executable", requestedExecutablePath, nil),
            ("Audited executable", approval.executablePath, rootStatus(approval.executableRootControlled))
        ]

        if isInterpreter {
            rows.append((
                "Interpreter script",
                scriptPath ?? "No script operand detected; interpreter flags or inline code are in use",
                scriptPath.map { _ in rootStatus(scriptRootControlled) }
                    ?? Status(title: "not a script file", color: Palette.amber)
            ))
        } else {
            rows.append(("Invocation type", "Direct executable; no interpreter script detected", nil))
        }

        rows.append((
            "Always allow scope",
            approval.canAlwaysAllow
                ? alwaysAllowDescription
                : "Unavailable because every executable script boundary is not root-controlled",
            approval.canAlwaysAllow
                ? Status(title: "root-controlled", color: Palette.accent)
                : Status(title: "manual only", color: Palette.amber)
        ))

        return panel(title: "Execution target", rows: rows)
    }

    private func secretsView() -> NSView {
        let view = makePanel()
        let stack = panelStack(in: view)
        stack.addArrangedSubview(sectionTitle("Secrets injected"))

        let flow = NSStackView()
        flow.orientation = .horizontal
        flow.alignment = .leading
        flow.spacing = 8
        for key in approval.keys {
            flow.addArrangedSubview(pill(key, color: Palette.accent, fillAlpha: 0.10, monospaced: true))
        }
        flow.addArrangedSubview(spacer())
        stack.addArrangedSubview(flow)
        stack.addArrangedSubview(label(
            "Only names are shown here. Values stay in the Keychain until Automic Vault injects them into the approved child process environment.",
            size: 11,
            weight: .regular,
            color: Palette.quietText
        ))
        return view
    }

    private func commandView() -> NSView {
        let view = makePanel()
        let stack = panelStack(in: view)
        stack.addArrangedSubview(sectionTitle("Command line"))

        let text = NSTextField(wrappingLabelWithString: commandLine)
        text.font = UIStyle.monoFont(size: 11, weight: .regular)
        text.textColor = Palette.text
        text.maximumNumberOfLines = 3
        text.lineBreakMode = .byTruncatingMiddle
        text.translatesAutoresizingMaskIntoConstraints = false

        let commandPanel = NSView()
        commandPanel.wantsLayer = true
        commandPanel.layer?.backgroundColor = Palette.panelRaised.cgColor
        commandPanel.layer?.cornerRadius = 8
        commandPanel.layer?.borderWidth = 1
        commandPanel.layer?.borderColor = Palette.stroke.cgColor
        commandPanel.translatesAutoresizingMaskIntoConstraints = false
        commandPanel.addSubview(text)
        NSLayoutConstraint.activate([
            text.leadingAnchor.constraint(equalTo: commandPanel.leadingAnchor, constant: 12),
            text.trailingAnchor.constraint(equalTo: commandPanel.trailingAnchor, constant: -12),
            text.topAnchor.constraint(equalTo: commandPanel.topAnchor, constant: 10),
            text.bottomAnchor.constraint(equalTo: commandPanel.bottomAnchor, constant: -10)
        ])
        stack.addArrangedSubview(commandPanel)
        return view
    }

    private func panel(
        title: String,
        rows: [(String, String, Status?)]
    ) -> NSView {
        let view = makePanel()
        let stack = panelStack(in: view)
        stack.addArrangedSubview(sectionTitle(title))
        for row in rows {
            stack.addArrangedSubview(infoRow(label: row.0, value: row.1, status: row.2))
        }
        return view
    }

    private func makePanel() -> NSView {
        let view = NSView()
        view.wantsLayer = true
        view.layer?.backgroundColor = Palette.panel.cgColor
        view.layer?.cornerRadius = 12
        view.layer?.borderWidth = 1
        view.layer?.borderColor = Palette.stroke.cgColor
        view.translatesAutoresizingMaskIntoConstraints = false
        return view
    }

    private func panelStack(in view: NSView) -> NSStackView {
        let stack = NSStackView()
        stack.orientation = .vertical
        stack.spacing = 8
        stack.edgeInsets = NSEdgeInsets(top: 13, left: 14, bottom: 13, right: 14)
        stack.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            stack.topAnchor.constraint(equalTo: view.topAnchor),
            stack.bottomAnchor.constraint(equalTo: view.bottomAnchor)
        ])
        return stack
    }

    private func infoRow(label title: String, value: String, status: Status?) -> NSView {
        let row = NSStackView()
        row.orientation = .horizontal
        row.alignment = .firstBaseline
        row.spacing = 10

        let titleLabel = label(title.uppercased(), size: 9, weight: .medium, color: Palette.quietText, monospaced: true)
        titleLabel.widthAnchor.constraint(equalToConstant: 128).isActive = true
        row.addArrangedSubview(titleLabel)

        let valueLabel = label(value, size: 12, weight: .regular, color: Palette.text, monospaced: value.contains("/"))
        valueLabel.lineBreakMode = .byTruncatingMiddle
        valueLabel.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        row.addArrangedSubview(valueLabel)

        if let status {
            row.addArrangedSubview(pill(status.title, color: status.color, fillAlpha: 0.12))
        }
        return row
    }

    private func sectionTitle(_ text: String) -> NSTextField {
        label(text.uppercased(), size: 10, weight: .semibold, color: Palette.dimText, monospaced: true, tracking: 1.1)
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
        field.maximumNumberOfLines = 2
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

    private func pill(
        _ text: String,
        color: NSColor,
        fillAlpha: CGFloat,
        monospaced: Bool = false
    ) -> NSView {
        let container = NSView()
        container.wantsLayer = true
        container.layer?.backgroundColor = color.withAlphaComponent(fillAlpha).cgColor
        container.layer?.borderColor = color.withAlphaComponent(0.34).cgColor
        container.layer?.borderWidth = 1
        container.layer?.cornerRadius = 10
        container.translatesAutoresizingMaskIntoConstraints = false

        let field = label(text, size: 10, weight: .medium, color: color, monospaced: monospaced)
        field.maximumNumberOfLines = 1
        field.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(field)

        NSLayoutConstraint.activate([
            field.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 8),
            field.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -8),
            field.topAnchor.constraint(equalTo: container.topAnchor, constant: 3),
            field.bottomAnchor.constraint(equalTo: container.bottomAnchor, constant: -3)
        ])
        return container
    }

    private func spacer() -> NSView {
        let view = NSView()
        view.setContentHuggingPriority(.defaultLow, for: .horizontal)
        return view
    }

    private var headerSummary: String {
        let requester = approval.parentProcess.displayName ?? "a parent process"
        return "\(requester) wants Automic Vault to place Keychain secrets into a command environment."
    }

    private var requestedExecutablePath: String {
        approval.requestedExecutablePath ?? approval.executablePath
    }

    private var commandLine: String {
        ([requestedExecutablePath] + approval.argv).joined(separator: " ")
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
