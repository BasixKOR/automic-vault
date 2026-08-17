# Automic Vault for Varlock

Use Automic Vault Secrets from a Varlock schema without invoking the `av` CLI.
Every Varlock run requires one fresh Approval in Automic Vault for its complete
set of Secret Names.

```dotenv
# @plugin(@automicvault/varlock-plugin)
# @disableProcessEnvInjection
# ---
# @sensitive @required
API_TOKEN=automicVault()
```

The resolver infers the Secret Name from the item name. Pass an explicit name
when they differ: `API_TOKEN=automicVault(ACTUAL_SECRET_NAME)`. Secret Names
must be static so the Approval can show the complete set before any Secret Value
is released. The plugin requires `@disableProcessEnvInjection`; applications
should read values through `ENV` from `varlock/env`.

For local development, install the package with
`npm install ../path/to/integrations/varlock`. The signed bridge ships inside
`Automic Vault.app`; it talks directly to the approval service over XPC.

The sample's `npm test` uses a mock bridge to verify that two Secret Names are
sent through one helper request without requesting real Secrets. `npm start`
exercises the signed XPC bridge and shows one Approval.
