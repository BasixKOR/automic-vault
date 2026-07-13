# vagrant Detector

## Trigger Conditions

- Vagrant Cloud token file contains a plaintext token.

## Mitigation

```sh
av harden vagrant
```

## Sensitive Files

- `$VAGRANT_HOME/data/vagrant_login_token`
- `~/.vagrant.d/data/vagrant_login_token`
