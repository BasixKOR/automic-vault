# huggingface-cli Radioisotope

Hugging Face Hub stores the active CLI token in `~/.cache/huggingface/token`.

This radioisotope migrates that active token into the Automic Vault keychain and
wraps `hf` so `HF_TOKEN` is present only while the CLI runs.

## Caveats

- We currently migrate the active token file only.
- Named `stored_tokens` entries are not migrated.
- Direct execution of the original binary will not receive the credentials.
