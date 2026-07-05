# Databricks Radioisotope

Detect-only coverage for plaintext Databricks CLI profile credentials.

Databricks CLI can store profile tokens and client secrets in config files
even when OAuth token storage uses the OS keyring. This radioisotope reports
those plaintext profile secrets without changing the CLI's auth behavior.
