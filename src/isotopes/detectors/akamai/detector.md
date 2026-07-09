# akamai Detector

## Trigger Conditions

- Akamai CLI .edgerc contains plaintext EdgeGrid credentials.

## Mitigation

```sh
sudo av harden akamai
```

## Sensitive Files

- `${AKAMAI_EDGERC:-$HOME/.edgerc}`
