# Maestro Radioisotope

Maestro stores cloud login state in `~/.mobiledev/authtoken`. Maestro Studio
can also store an OpenAI token in `~/.mobiledev/openaitoken`.

The radioisotope moves those files into the macOS keychain and runs `maestro`
with a temporary Java `user.home` containing the injected files. It also
exports the cloud token as `MAESTRO_CLOUD_API_KEY`, which Maestro supports for
non-interactive cloud authentication.

## Caveats

- Only `~/.mobiledev/authtoken` and `~/.mobiledev/openaitoken` are migrated.
- Runtime login/logout and Studio token changes happen in temporary runtime
  state and are not persisted back to the keychain.
- Commands that depend on other files under Java `user.home` will see the
  temporary home while the wrapper runs.
- Direct execution of the original binary will not receive credentials.
