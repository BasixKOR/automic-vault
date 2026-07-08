# aws-cli-credentials-file Detector

## Trigger Conditions

- AWS shared credentials file contains plaintext access keys.

## Mitigation

```sh
av harden aws
```

## Sensitive Files

- `$AWS_SHARED_CREDENTIALS_FILE`
- `~/.aws/credentials`
