# npm Detector

## Trigger Conditions

- npm config sets a package minimum release age below 24 hours.

## Sensitive Files

- `$NPM_CONFIG_USERCONFIG`
- `~/.npmrc`

## Mitigation

Set `min-release-age=1` in the reported npm config file.
