# Railway Hardener

`av harden railway` installs the signed Railway Isotope and migrates legacy or
OAuth credentials from the production, staging, and development config files
into environment-and-host-bound Automic Vault Secrets. The files retain user
and project metadata plus reserved non-secret `@av` markers.

The patched Target routes session reads, login and refresh stores, and logout or
invalid-grant deletes through fixed Railway-only XPC operations. Each operation
binds the live Target, complete arguments, Railway environment, host, and
derived Secret Name.

Unsupported user fields, mixed legacy/OAuth credentials, partial markers, and
incomplete sessions fail without changing any config file. The authorized
Target receives reusable session material in memory; the Hardener cannot
protect it after Secret Application.
