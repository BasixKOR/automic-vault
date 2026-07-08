# aws-cli-legacy-plugins Detector

## Trigger Conditions

- AWS CLI legacy plugins are configured.

## Rationale

Legacy plugins are a trivial hook for malware to use to steal your keys.

## Mitigation

Edit your configuration file and delete the legacy plugins section.

## Sensitive Files

- `$AWS_CONFIG_FILE`
- `~/.aws/config`
