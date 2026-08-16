# Automic Vault for Varlock

Use Automic Vault Secrets from a Varlock schema without invoking the `av` CLI.
Every resolution requires a fresh Approval in Automic Vault.

```dotenv
# @plugin(@automicvault/varlock-plugin)
# ---
# @sensitive @required
API_TOKEN=automicVault()
```

The resolver infers the Secret Name from the item name. Pass an explicit name
when they differ: `API_TOKEN=automicVault(ACTUAL_SECRET_NAME)`.

For local development, install the package with
`npm install ../path/to/integrations/varlock`. The signed bridge ships inside
`Automic Vault.app`; it talks directly to the approval service over XPC.

The sample's `npm test` uses `/usr/bin/printf` in place of the bridge to check
the Varlock resolver without requesting a real Secret. `npm start` exercises
the signed XPC bridge and shows an Approval.
