# buf Detector

## Trigger Conditions

- Buf registry token is stored in plaintext netrc.

## Mitigation

```sh
av harden buf
```

## Sensitive Files

- `~/.netrc`
