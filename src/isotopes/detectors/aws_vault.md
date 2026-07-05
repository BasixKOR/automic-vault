# aws-vault Detector

## Trigger Conditions

- AWS config invokes aws-vault as an ambient credential_process.
- aws-vault file backend directory contains credential vault files.

## Sensitive Files

- `~/.aws/config`
- `~/.awsvault/keys/*`
