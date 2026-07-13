# snowflake-cli Detector

## Trigger Conditions

- Snowflake CLI config contains plaintext credentials.

## Mitigation

```sh
av harden snowflake-cli
```

## Sensitive Files

- `~/.snowflake/config.toml`
- `~/.snowflake/connections.toml`
- `~/Library/Application Support/snowflake/config.toml`
- `~/Library/Application Support/snowflake/connections.toml`
- `~/.config/snowflake/config.toml`
- `~/.config/snowflake/connections.toml`
