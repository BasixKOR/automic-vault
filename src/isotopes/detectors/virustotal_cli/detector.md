# virustotal-cli Detector

## Trigger Conditions

- VirusTotal config contains a plaintext API key.

## Mitigation

```sh
av harden virustotal-cli
```

## Sensitive Files

- `~/.vt.toml`
