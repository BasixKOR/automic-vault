# opencode Radioisotope Detector

This detector reports plaintext opencode auth state.

opencode stores provider credentials in `auth.json` and newer `account.json`
files under its XDG data directory. Those files can contain OAuth access and
refresh tokens or API keys.

This radioisotope is detect-only because the auth file is mutable application
state and sits beside non-secret opencode data. A safe fix should be a source
isotope or upstream keychain-backed account store.
