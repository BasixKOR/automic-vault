# maven Radioisotope

Maven user settings commonly store repository server passwords in
`~/.m2/settings.xml`.

This radioisotope migrates `settings.xml` into the Automic Vault keychain and
wraps `mvn` so Maven reads a temporary settings file only while it is running.

## Caveats

- We currently migrate the default user `~/.m2/settings.xml` file only.
- Explicit `--settings` or `-s` arguments can override the temporary file.
- Maven's separate password-encryption tooling is a pure-isotope candidate and
  is not changed by this radioisotope.
