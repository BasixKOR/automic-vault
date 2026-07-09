# wsk Detector

## Trigger Conditions

- OpenWhisk CLI properties contain a plaintext AUTH key.

## Mitigation

```sh
sudo av harden wsk
```

## Sensitive Files

- `~/.wskprops`
