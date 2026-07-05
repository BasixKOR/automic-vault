# mkcert Detector

## Trigger Conditions

- mkcert CAROOT contains a plaintext root CA private key.

## Sensitive Files

- `$CAROOT/rootCA-key.pem`
- `~/Library/Application Support/mkcert/rootCA-key.pem`
