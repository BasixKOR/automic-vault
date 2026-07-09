# heroku Detector

## Trigger Conditions

- Heroku API token is stored in plaintext netrc.

## Mitigation

```sh
sudo av harden heroku
```

## Sensitive Files

- `$NETRC`
- `~/.netrc`
