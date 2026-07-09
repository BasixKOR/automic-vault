# composer Detector

## Trigger Conditions

- Composer auth.json contains plaintext credentials.

## Mitigation

```sh
sudo av harden composer
```

## Sensitive Files

- `$COMPOSER_HOME/auth.json`
- `$XDG_CONFIG_HOME/composer/auth.json`
- `~/.composer/auth.json`
- `~/Library/Application Support/Composer/auth.json`
