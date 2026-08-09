# Signed CLI Launchers

Automic Vault can bind approval policy to either a signed app bundle or a
Developer ID-signed standalone executable. It validates the live code signature
and stores the launcher's designated requirement, which identifies both the
binary and its signing team.

For a standalone executable to be eligible, it must:

- have a valid Developer ID Application signature and a stable identifier and
  Team ID;
- pass strict macOS code-signature validation; and
- enable Hardened Runtime before it can receive secret-gate access.

Unsigned and ad-hoc signed executables are rejected. Ad-hoc signing can protect
one build from modification, but it does not establish a vendor or Team identity.
Placing such an executable inside an unsigned app does not make it eligible.

## Allow a CLI launcher

1. Run `av doctor claude` or `av doctor codex` to inspect the corresponding
   executable selected by your current `PATH`.
2. In Automic Vault Settings, add a launcher to the relevant tool or blessed
   script policy.
3. Select the resolved native executable, not a shell or package-manager shim.
4. Review the identifier, Team ID, path, and designated requirement before
   allowing it.

Automic Vault resolves symlinks and verifies the selected executable itself. If
the signature is missing, ad-hoc, invalid, or not Developer ID for a standalone
executable, selection fails. If the identity cannot be verified later, automatic
approval fails closed and requires manual approval.

## Default and launcher-specific access

An Authorization Gate's default Access Level applies to every Verified Launcher
without a matching launcher-specific rule, including eligible standalone
executables. A launcher-specific rule takes precedence over the default.
Standalone executables receive default access only while their live Developer ID
identity and Hardened Runtime protections remain valid. New launcher-specific
rules enforce the same eligibility; persisted older rules retain their recorded
runtime requirement for compatibility. Unknown operations still require
Approval at every Access Level.

Some package formats start a signed native payload through a wrapper. Prefer a
standalone installer or Homebrew cask when available because the live launcher
identity and `av doctor` result are clearer. Always verify the installed command;
distribution details can change independently of Automic Vault.

Code signing proves identity and integrity, not intent. Only allow a signing team
you trust, and keep the terminal or agent app's TCC permissions minimal because
TCC remains app-scoped.
