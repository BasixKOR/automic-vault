# censys

Censys stores local CLI credentials in `~/.config/censys/censys.cfg`. That
file can contain Search API credentials and an ASM API key.

This radioisotope migrates those credential entries into Automic Vault, removes
them from the local file, and injects `CENSYS_API_ID`, `CENSYS_API_SECRET`, and
`CENSYS_ASM_API_KEY` only while `censys` runs. Non-secret config remains on
disk, and the detector reports the config if credential entries reappear.

## Caveats

- Empty credential fields are not exported at runtime.
- Explicit `CENSYS_CONFIG_PATH` environment values can still point the CLI at a
  different config file.
- Direct execution of the original binary will not receive credentials.
