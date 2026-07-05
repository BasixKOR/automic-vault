# Poetry Radioisotope

Detect-only coverage for Poetry's plaintext auth fallback.

Poetry can store repository passwords and PyPI tokens in `auth.toml` when a
usable system keyring is unavailable. This radioisotope reports those fallback
credentials without changing Poetry's keyring behavior.
