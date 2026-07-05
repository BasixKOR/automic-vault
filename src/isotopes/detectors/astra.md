# Astra CLI radioisotope

The Astra CLI stores application tokens in `.astrarc`. This radioisotope moves
that file into the macOS keychain and restores it into a temporary config file
only while `astra` runs.

## Covered credentials

- Default `~/.astrarc`
- XDG config location `$XDG_CONFIG_HOME/astra/.astrarc`
- Profiles containing `AstraCS:` application tokens

## Caveats

- Runtime profile changes are not persisted back to the keychain.
- Explicit `ASTRARC` or `--config-file` usage can bypass the wrapped config
  file.
- Secure Connect Bundles and cache files under `.astra` are left in place.
