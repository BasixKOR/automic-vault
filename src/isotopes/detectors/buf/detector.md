# buf Detector

## Trigger Conditions

- Buf registry token is stored in plaintext netrc.

## Mitigation

```sh
sudo av harden buf
```

## Sensitive Files

- `~/.netrc`
