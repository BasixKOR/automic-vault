# wsk Detector

## Trigger Conditions

- OpenWhisk CLI properties contain a plaintext AUTH key.

## Mitigation

```sh
av harden wsk
```

## Sensitive Files

- `~/.wskprops`
