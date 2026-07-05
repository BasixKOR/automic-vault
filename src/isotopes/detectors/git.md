# git Detector

Reports when:
- Git credential store contains plaintext credentials.
- Git config enables a plaintext Git credential-store file.
- Git credential helper delegates github.com credentials to `gh auth git-credential`.
- `git credential fill` returns a GitHub password or token for github.com.
- Git config enables git-credential-oauth as an ambient credential helper.
- Git config contains a plaintext OAuth client secret.

## Detection Caveats

- Scans `~/.git-credentials`, global Git config, and XDG Git config; repository-local and included Git config files are not followed.
- The live `git credential fill` probe only queries `github.com` and may produce an unattributed finding because Git does not report which helper returned the credential.
