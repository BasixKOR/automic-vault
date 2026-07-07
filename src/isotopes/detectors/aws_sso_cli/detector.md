# aws-sso-cli Detector

## Trigger Conditions

- AWS SSO cache contains plaintext token or role credentials.

## Sensitive Files

- `~/.aws/sso/cache/*.json`
- `~/.aws/cli/cache/*.json`
