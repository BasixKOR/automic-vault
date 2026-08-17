# Automic Vault for Varlock

Use Automic Vault Secrets from a Varlock schema without invoking the `av` CLI
at runtime.

## Install

Install Automic Vault in `/Applications`, then add Varlock and the plugin:

```sh
$ npm install --save-dev varlock @automicvault/varlock-plugin
$ av save API_TOKEN
# Store the Secret Value in Automic Vault
```

## Use

Add `.env.schema`:

```dotenv
# @plugin(@automicvault/varlock-plugin)
# @disableProcessEnvInjection
# ---
# @sensitive @required
API_TOKEN=automicVault()
```

Load Varlock before importing `ENV`, then read Secrets through `ENV` rather
than `process.env`:

```js
import 'varlock/auto-load';
import { ENV } from 'varlock/env';

const response = await fetch('https://api.example.com/me', {
  headers: { Authorization: `Bearer ${ENV.API_TOKEN}` },
});
```

The resolver infers the Secret Name from the item name. Pass an explicit name
when they differ: `API_TOKEN=automicVault(ACTUAL_SECRET_NAME)`. Secret Names
must be static so the Approval can show the complete set before any Secret Value
is released. The plugin requires `@disableProcessEnvInjection`; applications
should read values through `ENV` from `varlock/env`.

The signed bridge ships inside `Automic Vault.app`; it talks directly to the
approval service over XPC. One Approval covers the complete active Secret set
for that Varlock run.

> [!IMPORTANT]
> Varlock currently requires Approval on every run. Automic Authorization and
> Blessings are not supported for this plugin yet.

The sample's `npm test` uses a mock bridge to verify that two Secret Names are
sent through one helper request without requesting real Secrets. `npm start`
exercises the signed XPC bridge and shows one Approval.
