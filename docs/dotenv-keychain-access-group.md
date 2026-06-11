# Dotenv Keychain Access Group

Automic Vault stores dotenv private keys in the macOS Data Protection Keychain.
The public keychain item contract is:

- class: generic password
- service: `com.automicvault.dotenv`
- account: `DOTENV_PRIVATE_KEY:<public-key-fingerprint>`
- access group: `ZU76A67LGU.com.automicvault.dotenv`

Companion apps that need dotenv private-key access must be signed by Team ID
`ZU76A67LGU` and must include a `keychain-access-groups` entitlement containing
`ZU76A67LGU.com.automicvault.dotenv`. Notarization and the same Developer ID
team are not enough by themselves; macOS grants access only to signed binaries
with the shared keychain access-group entitlement.

Developer ID macOS builds that carry this restricted entitlement must also
include an eligible provisioning profile. Embed it in each entitled app bundle
as `Contents/embedded.provisionprofile` before signing. A loose command-line
executable cannot carry an embedded app-bundle provisioning profile; do not add
the keychain-sharing entitlement to loose helper binaries unless they are
packaged with their own eligible profile context.

Every dotenv private-key `SecItemAdd`, `SecItemCopyMatching`, `SecItemUpdate`,
and `SecItemDelete` query against the shared store must include:

```swift
kSecUseDataProtectionKeychain: true
kSecAttrAccessGroup: "ZU76A67LGU.com.automicvault.dotenv"
```

Queries must also include the stable class, service, and account fields above.
Do not use `kSecAttrAccess`, `SecAccess`, or `SecTrustedApplication` for the
shared dotenv private-key store. Those APIs belong only to the legacy
login-keychain fallback path.

Verify signed artifacts with:

```sh
codesign -d --entitlements - <path>
```

On newer macOS releases, `codesign -d --entitlements :-` may inspect the legacy
XML entitlement slot and warn even when the signed DER entitlement dictionary is
valid. Use the `-` form above when checking the effective signed entitlements.

During migration, `av dotenv` reads the shared Data Protection Keychain first.
If that item is not found, it falls back to the legacy login-keychain item. New
writes go only to the shared Data Protection Keychain.
