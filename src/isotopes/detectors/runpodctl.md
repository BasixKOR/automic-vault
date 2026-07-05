# runpodctl Radioisotope

The runpodctl CLI stores the RunPod API key in its local config, usually
`~/.runpod/config.toml`. Older installs can also have `~/.runpod.yaml`, which
current runpodctl versions still read and migrate.

Automic Vault migrates a single API key from those package-owned config files
into the macOS keychain, removes the API-key entry from the persisted config,
and injects `RUNPOD_API_KEY` only while `runpodctl` runs.

This keeps local RunPod API keys out of plaintext home-directory config without
changing runpodctl's command-line interface. Non-secret config entries remain
on disk, and the detector reports the config if the API key reappears.

## Caveats

- If both TOML and legacy YAML configs exist with different API keys, migration
  is left for manual handling because one `RUNPOD_API_KEY` cannot safely
  represent both files.
- We migrate the default `~/.runpod/config.toml` and legacy `~/.runpod.yaml`
  files only.
- Direct execution of the original binary will not receive credentials.
