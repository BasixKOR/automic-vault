# LuaRocks Radioisotope

`luarocks upload --api-key=...` persists the upload API key in
`upload_config.lua` next to the user's LuaRocks config. The radioisotope moves
that key into the macOS keychain and injects it as `LUAROCKS_API_KEY` only when
`luarocks upload` runs without an explicit `--api-key`.

The migration checks the default XDG and legacy LuaRocks user config
locations, plus any `LUAROCKS_CONFIG`-style override in the current
environment, and rewrites `key = "..."` assignments to `key = nil`.

## Caveats

- Only Lua string assignments to an upload config `key` field are migrated.
- If multiple distinct upload keys are found, migration stops so the user can
  choose the correct key manually.
- Upload keys passed later with `--api-key` are still handled by LuaRocks
  itself and may be written by LuaRocks.
- Direct execution of the original binary will not receive credentials.
