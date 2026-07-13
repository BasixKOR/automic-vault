# jfrog-cli Detector

## Trigger Conditions

- JFrog CLI config contains plaintext credentials.

## Mitigation

```sh
av harden jfrog-cli
```

## Sensitive Files

- `~/.jfrog/jfrog-cli.conf.v6`
