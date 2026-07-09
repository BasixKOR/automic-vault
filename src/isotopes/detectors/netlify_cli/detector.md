# netlify-cli Detector

## Trigger Conditions

- Netlify CLI config contains plaintext credentials.

## Mitigation

```sh
sudo av harden netlify-cli
```

## Sensitive Files

- `~/Library/Preferences/netlify/config.json`
- `~/.netlify/config.json`
