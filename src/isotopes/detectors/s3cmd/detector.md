# s3cmd Detector

## Trigger Conditions

- s3cmd config contains plaintext credentials.

## Mitigation

```sh
av harden s3cmd
```

## Sensitive Files

- `~/.s3cfg`
