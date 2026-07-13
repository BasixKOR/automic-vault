# heroku Detector

## Trigger Conditions

- Heroku API token is stored in plaintext netrc.

## Mitigation

```sh
av harden heroku
```

## Sensitive Files

- `$NETRC`
- `~/.netrc`
