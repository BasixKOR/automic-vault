# ordercli Hardener

`av harden ordercli` installs the signed ordercli Isotope and migrates the
Foodora access token, refresh token, OAuth client secret, pending MFA token,
and cookies into one Automic Vault Secret. Supported config files retain only
provider metadata and reserved non-secret `@av` markers.

The patched Target routes config reads, login/session updates, and logout
deletes through fixed ordercli-only XPC operations. Each operation binds the
live Target, complete arguments, provider scope, and exact Secret Name.

Conflicting configs, unsupported Foodora fields, partial markers, and malformed
credential bundles fail without changing any config file. Deliveroo config is
not credential-bearing and remains unchanged.
