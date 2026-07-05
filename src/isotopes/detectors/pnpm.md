# pnpm Detector

## Trigger Conditions

- npm user config contains a plaintext auth token.
- pnpm config exists without a package minimum release age of at least 7 days.

## Sensitive Files

- `$NPM_CONFIG_USERCONFIG`
- `~/.npmrc`
- `$XDG_CONFIG_HOME/pnpm/rc`
- `~/.config/pnpm/rc`
- `~/Library/Preferences/pnpm/rc`
