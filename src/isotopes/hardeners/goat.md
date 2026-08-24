# goat Hardener

`av harden goat` installs the signed goat Isotope and migrates the complete
password session from `auth-session.json` into Automic Vault. The file retains
only the DID, PDS origin, and reserved non-secret `@av` markers.

The patched Target routes session reads, login and refresh stores, and logout
deletes through fixed goat-only XPC operations. Each operation binds the live
Target, complete arguments, DID, PDS, and derived Secret Name.

Unknown fields and incomplete sessions fail without changing the file. The
authorized Target receives reusable session material in memory; the Hardener
cannot protect it after Secret Application.
