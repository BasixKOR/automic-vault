# aws-cli Detector

Reports when:
- AWS shared credentials file contains plaintext access keys.
- AWS CLI legacy plugins are configured.
- AWS login cache contains cached access credentials.

## Detection Caveats

- Scans `AWS_SHARED_CREDENTIALS_FILE` or `~/.aws/credentials`, `AWS_CONFIG_FILE` or `~/.aws/config`, and `~/.aws/login/cache`.
