# huggingface-cli Detector

## Trigger Conditions

- Hugging Face token file contains a plaintext token.

## Mitigation

```sh
av harden huggingface-cli
```

## Sensitive Files

- `~/.cache/huggingface/token`
