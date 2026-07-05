# isotope:algolia

Algolia CLI profiles are stored in plain text at
`~/.config/algolia/config.toml`. Profiles can include API keys and Crawler API
keys.

This radioisotope migrates a single profile's credentials into Automic
Vault-backed keychain storage as Algolia's native environment variables. The
persisted profile file is rewritten with API keys blanked while non-secret
settings remain available.

## Caveats

- Config files with multiple profiles must be migrated manually because
  Algolia credential environment variables override profile selection.
- Profiles with API keys must include `application_id`, and Crawler API keys
  must include `crawler_user_id`.
- Explicit `ALGOLIA_APPLICATION_ID`, `ALGOLIA_API_KEY`,
  `ALGOLIA_ADMIN_API_KEY`, `ALGOLIA_CRAWLER_USER_ID`, or
  `ALGOLIA_CRAWLER_API_KEY` environment values still take precedence in the
  Algolia CLI.
- Direct execution of the original binary will not receive migrated
  credentials.
