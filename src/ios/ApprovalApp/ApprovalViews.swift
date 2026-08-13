import SwiftUI

struct ApprovalRootView: View {
    @Bindable var model: ApprovalModel

    var body: some View {
        NavigationStack {
            Group {
                if model.state == .setup {
                    setup
                } else if model.pending.count == 1, let request = model.pending.first {
                    ApprovalDetailView(request: request, model: model)
                } else if model.pending.isEmpty {
                    empty
                } else {
                    list
                }
            }
            .navigationTitle("Approvals")
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    NavigationLink { ApprovalSettingsView(model: model) } label: {
                        Label("Settings", systemImage: "gear")
                    }
                }
            }
            .alert("Approval Error", isPresented: Binding(
                get: { model.errorMessage != nil },
                set: { if !$0 { model.errorMessage = nil } }
            )) {
                Button("OK") { model.errorMessage = nil }
            } message: {
                Text(model.errorMessage ?? "")
            }
        }
    }

    private var setup: some View {
        ContentUnavailableView {
            Label("Approve Away from Your Mac", systemImage: "iphone.and.arrow.forward")
        } description: {
            Text("Use this iPhone for every human Approval on an enrolled Mac. iCloud Keychain and notifications are required.")
        } actions: {
            Button("Enable iPhone Approval") { Task { await model.enable() } }
                .buttonStyle(.borderedProminent)
        }
    }

    private var empty: some View {
        ContentUnavailableView {
            Label("No Pending Approvals", systemImage: "checkmark.shield")
        } description: {
            Text(connectionText)
        }
    }

    private var connectionText: String {
        switch model.state {
        case .setup: "Enable notifications to receive Approval requests."
        case .connecting: "Connecting to your Macs…"
        case .connected where model.connectedMacs.isEmpty:
            "Connected. Waiting to hear from an enrolled Mac."
        case .connected:
            "Connected Macs: \(model.connectedMacs.values.sorted().joined(separator: ", "))."
        case .unavailable(let reason): reason
        }
    }

    private var list: some View {
        List {
            Section {
                ForEach(model.pending) { request in
                    NavigationLink {
                        ApprovalDetailView(request: request, model: model)
                    } label: {
                        VStack(alignment: .leading, spacing: 4) {
                            Text(request.launcher).font(.headline)
                            Text(request.command).font(.callout.monospaced()).lineLimit(2)
                            Text("\(request.macName) · \(request.secretNames.count) secret\(request.secretNames.count == 1 ? "" : "s")")
                                .font(.caption).foregroundStyle(.secondary)
                        }
                    }
                }
            } header: {
                Text("\(model.pending.count) pending")
            } footer: {
                Button("Deny All", role: .destructive) { Task { await model.denyAll() } }
            }
        }
    }
}

struct ApprovalDetailView: View {
    let request: PhoneApprovalRequest
    @Bindable var model: ApprovalModel

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                VStack(alignment: .leading, spacing: 6) {
                    Text(request.launcher).font(.title2.bold())
                    Text("on \(request.macName)").foregroundStyle(.secondary)
                }

                LabeledContent("Tool", value: request.tool)
                VStack(alignment: .leading, spacing: 8) {
                    Text("Command").font(.caption).foregroundStyle(.secondary)
                    Text(request.command)
                        .font(.body.monospaced())
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(12)
                        .background(.secondary.opacity(0.1), in: RoundedRectangle(cornerRadius: 12))
                }
                LabeledContent("Working Directory", value: request.cwd)
                LabeledContent("Secrets", value: request.secretNames.joined(separator: ", "))
                HStack {
                    Text("Requested").foregroundStyle(.secondary)
                    Spacer()
                    Text(Date(timeIntervalSince1970: TimeInterval(request.createdAtMilliseconds) / 1_000), style: .relative)
                }

                Label(request.reason, systemImage: request.requiresFullReview ? "exclamationmark.shield.fill" : "shield")
                    .foregroundStyle(request.requiresFullReview ? .orange : .primary)

                ForEach(Array(request.details.enumerated()), id: \.offset) { _, section in
                    DisclosureGroup(section.title) {
                        VStack(spacing: 10) {
                            ForEach(Array(section.rows.enumerated()), id: \.offset) { _, row in
                                LabeledContent(row.label, value: row.value)
                            }
                        }
                        .padding(.top, 8)
                    }
                }

                HStack(spacing: 12) {
                    Button("Deny", role: .destructive) { Task { await model.deny(request) } }
                        .buttonStyle(.bordered).controlSize(.large).frame(maxWidth: .infinity)
                    Button("Approve Once") { Task { await model.approve(request) } }
                        .buttonStyle(.borderedProminent).controlSize(.large).frame(maxWidth: .infinity)
                }
            }
            .padding()
        }
        .navigationTitle("Approval")
        .navigationBarTitleDisplayMode(.inline)
    }
}

struct ApprovalSettingsView: View {
    @Bindable var model: ApprovalModel

    var body: some View {
        Form {
            Section("Protection") {
                Toggle("Require Face ID or Touch ID", isOn: Binding(
                    get: { model.biometricProtectionEnabled },
                    set: { enabled in Task { await model.setBiometricProtection(enabled) } }
                ))
                Text("When enabled, Approve requires biometrics on this iPhone. Passcode, Apple Watch, and a companion Mac cannot substitute.")
                    .font(.footnote).foregroundStyle(.secondary)
            }
            Section("Physical Separation") {
                Label("iPhone Mirroring and Show on Mac can put Approval controls back onto a Mac when biometric protection is off.", systemImage: "exclamationmark.triangle.fill")
                    .foregroundStyle(.orange)
                Text("Turn off Show on Mac for Automic Vault notifications and revoke iPhone Mirroring access if agents can control your Mac.")
                    .font(.footnote)
            }
            Section("Connection") { Text(connectionLabel) }
        }
        .navigationTitle("Settings")
    }

    private var connectionLabel: String {
        switch model.state {
        case .setup: "Not enabled"
        case .connecting: "Connecting…"
        case .connected: "Connected"
        case .unavailable(let reason): reason
        }
    }
}
