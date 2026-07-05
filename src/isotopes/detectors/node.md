# Node Radioisotope

This radioisotope modifies the Homebrew `node` package family, including
versioned formulae such as `node@24`, but only changes the installed `npm`
launcher. `node` and `npx` continue to run without isotope credential
injection.

## Security Model

Plaintext npm publishing tokens are commonly stored in `~/.npmrc` as
`_authToken` entries. The migration stores one token in the Automic Vault
isotope keychain as `NODE_AUTH_TOKEN` and rewrites matching npm config entries
to reference `${NODE_AUTH_TOKEN}`.

The post-install hook wraps `/opt/<formula>/bin/npm`, for example
`/opt/node/bin/npm` or `/opt/node@24/bin/npm`. The wrapper injects
`NODE_AUTH_TOKEN` only when an `npm publish` invocation is detected, then execs
the original npm launcher.

## Caveats

- Only one npm publishing token is supported.
- Multiple distinct `_authToken` values fail migration and must be handled
  manually.
- Project-level npm configs are not migrated; only the npm user config is
  inspected.
