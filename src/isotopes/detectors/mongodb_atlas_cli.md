# MongoDB Atlas CLI Radioisotope Detector

This detector reports plaintext MongoDB Atlas CLI fallback credentials.

Current MongoDB Atlas CLI uses a native OS keyring when available. The upstream
fallback path can store credential fields in `config.toml` under the Atlas CLI
config directory, normally `~/Library/Application Support/atlascli/config.toml`
on macOS.

This radioisotope is detect-only because the right remediation is to use or
repair upstream's keyring-backed store, not wrap the CLI.
