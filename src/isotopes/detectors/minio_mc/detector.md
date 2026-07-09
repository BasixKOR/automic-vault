# minio-mc Detector

## Trigger Conditions

- MinIO mc config file contains plaintext alias secrets.

## Mitigation

```sh
sudo av harden minio-mc
```

## Sensitive Files

- `~/.mc/config.json`
