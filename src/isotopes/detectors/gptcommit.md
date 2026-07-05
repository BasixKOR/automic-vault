# gptcommit Radioisotope

`gptcommit` can store OpenAI API keys in
`~/.config/gptcommit/config.toml`. It also supports
`GPTCOMMIT__OPENAI__API_KEY`, which gives the radioisotope a narrow wrapper
boundary.

This radioisotope migrates the global API key into the macOS keychain, removes
it from the global config file, and injects it only while `gptcommit` runs.

## Caveats

- Only the global `~/.config/gptcommit/config.toml` API key is migrated.
- Repository-local `gptcommit.toml` files are detected but not migrated.
- Runtime config edits happen in a temporary HOME and are not persisted.
- Direct execution of the original binary will not receive credentials.
