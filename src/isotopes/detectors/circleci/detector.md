# circleci Detector

## Trigger Conditions

- CircleCI config contains an API token.

## Mitigation

```sh
av harden circleci
```

## Sensitive Files

- `~/.circleci/cli.yml`
