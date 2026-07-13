# pulumi Detector

## Trigger Conditions

- Pulumi credentials file contains plaintext access tokens.

## Mitigation

```sh
av harden pulumi
```

## Sensitive Files

- `$PULUMI_CREDENTIALS_PATH`
- `$PULUMI_HOME/credentials.json`
- `~/.pulumi/credentials.json`
