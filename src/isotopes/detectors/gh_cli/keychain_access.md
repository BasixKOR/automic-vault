# gh-cli-keychain-access Detector

## Trigger Conditions

Any process that can run `/usr/bin/security` can trivially retrieve your GitHub
token:

```sh
security find-generic-password -s gh:<host> -w
```

Or, more simply:

```sh
gh auth token
```

## Mitigation

```sh
av harden gh
```
