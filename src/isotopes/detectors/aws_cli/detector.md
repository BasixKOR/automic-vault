# aws-cli Detector

## Trigger Conditions

- AWS shared credentials file contains plaintext access keys.
- AWS CLI legacy plugins are configured.
- AWS login cache contains cached access credentials.

## Sensitive Files

- `$AWS_SHARED_CREDENTIALS_FILE`
- `~/.aws/credentials`
- `$AWS_CONFIG_FILE`
- `~/.aws/config`
- `~/.aws/login/cache/*.json`
