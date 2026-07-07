# pnpm Detector

## Trigger Conditions

- npm user config contains a plaintext auth token.
- pnpm config sets a package minimum release age below 24 hours.

## Sensitive Files

- `$NPM_CONFIG_USERCONFIG`
- `~/.npmrc`
- `$XDG_CONFIG_HOME/pnpm/rc`
- `~/.config/pnpm/rc`
- `~/Library/Preferences/pnpm/rc`
