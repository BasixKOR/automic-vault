# ADR 0020: Attribute app Launcher identity only to its main executable

Status: accepted

## Context

A signed executable may live inside an app bundle without being that app's
Launcher. For example, Git supplied by Xcode runs from
`Xcode.app/Contents/Developer/usr/bin/git` between Terminal and `av-gpg`.
Treating every bundle-contained executable as the containing app made this
intermediary appear to be the Xcode Launcher and forced full validation of the
large Xcode resource seal before each signing Approval.

That attribution also conflicts with the existing definition of a Launcher as
the app or executable at the root of the operation's verified launch chain.

## Decision

An ordinary app bundle is a Launcher candidate only when the live process path
or its code-signing main executable matches the bundle's declared main
executable. Automic Vault then performs the existing full static code-signature
and resource validation before accepting that app as a Verified Launcher.

Bundle-contained intermediary executables remain visible in the process chain
and retain their live code-signature and runtime-posture checks, but they do not
inherit the containing app's Launcher Identity. Existing explicit rules for
Automic Vault Launcher Bundle payloads and the Vaultty session bridge remain
unchanged. Eligible standalone Developer ID executables retain their existing
fallback identity.

## Consequences

Terminal, Portal, Codex, and other app main processes keep their existing app
Launcher identity and full bundle validation. Xcode's Git no longer claims
Xcode Launcher authority or causes Xcode's complete resource seal to be scanned
while authorizing a Git signature. A helper without its app's main process in
the live ancestry must qualify under an explicit helper association, Launcher
Bundle enrollment, or standalone Launcher rules instead of inheriting ambient
authority from its containing app.
