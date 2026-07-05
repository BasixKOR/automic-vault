# VirusTotal CLI Radioisotope

The VirusTotal CLI stores its API key in `~/.vt.toml` after `vt init`.
The upstream CLI also accepts `VTCLI_APIKEY`, which gives the radioisotope a
clean wrapper boundary.

The radioisotope migrates the API key into the Automic Vault keychain and
injects it only while `vt` runs. The wrapper uses a temporary `HOME`, so
runtime `vt init` output and relationship caches do not write new plaintext
secrets into the user's real home directory.

The migration preserves non-secret `~/.vt.toml` settings when possible, but
runtime edits to temporary config/cache files are not persisted back to
keychain.
