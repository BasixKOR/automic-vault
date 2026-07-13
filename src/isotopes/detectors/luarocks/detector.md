# luarocks Detector

## Trigger Conditions

- LuaRocks upload config contains a plaintext API key.

## Mitigation

```sh
av harden luarocks
```

## Sensitive Files

- `$XDG_CONFIG_HOME/luarocks/upload_config.lua`
- `~/.config/luarocks/upload_config.lua`
- `~/.luarocks/upload_config.lua`
