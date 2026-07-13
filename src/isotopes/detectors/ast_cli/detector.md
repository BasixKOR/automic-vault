# ast-cli Detector

## Trigger Conditions

- Checkmarx AST config contains plaintext credentials.

## Mitigation

```sh
av harden ast-cli
```

## Sensitive Files

- `$CX_CONFIG_FILE_PATH`
- `~/.checkmarx/checkmarxcli.yaml`
