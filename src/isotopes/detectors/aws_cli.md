# Automic Vault `aws-cli` Isotope

The isotope now uses AWS' native `credential_process` protocol instead of
placing AWS secrets in the `aws` process environment.

## Implementation

Migration moves plain text keys from `~/.aws/credentials` to the Keychain
and installs this non-secret config in `~/.aws/config`:

```ini
[default]
credential_process = /usr/local/bin/av credential-helper aws
```

The installed `/opt/awscli/bin/aws` launcher runs AWS Python in isolated mode
and mints a short-lived `AUTOMIC_VAULT_CREDENTIAL_HELPER_TOKEN` for the AWS
process. The helper only answers when that token is present and the parent
process is the root-controlled AWS launcher path running under isolated Python,
so unrelated processes cannot call the helper directly to retrieve credentials
and cannot use `PYTHONPATH`/`sitecustomize` injection to make AWS Python call it.
The launcher refuses to run when AWS CLI legacy external plugins are configured
because those plugins run as Python code inside the credential-approved AWS
process.

Commands that can print credentials, authentication tokens, private keys,
decrypted secrets, or signed capability URLs are approval gated in the launcher
before AWS CLI code runs. That includes `aws configure export-credentials` and
the shorter `aws config export-credentials` spelling, temporary-credential APIs
such as STS, SSO, IAM, Cognito identity, Lake Formation, S3 Control, and
service-specific credential issuers; service login tokens such as ECR,
CodeArtifact, RDS, and EKS; decrypted secret reads from Secrets Manager, SSM
with `--with-decryption`, KMS, and ACM; and S3 presigned URLs. The launcher
recognizes AWS global options before or between the service and operation
tokens.

Detection also treats `aws login` cache files under `~/.aws/login/cache` as
plain text credentials. Migration warns when those files are present because
this isotope cannot safely migrate the result of `aws login`.

## Caveats

We assume a single profile and user. If you have more complex credential
requirements you should use `brew:aws-vault-binary` instead. It’s more
cumbersome but also more capable.

AWS CLI legacy external plugins configured under `[plugins]` are not supported.
The detector reports them and the launcher refuses to run until they are
removed. If your workflow depends on them, use non-isotoped `brew:awscli` or a
dedicated credential manager.
