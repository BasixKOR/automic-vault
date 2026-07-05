# aws-vault Radioisotope

Detect-only coverage for aws-vault configuration and file-backed state.

aws-vault is already a credential manager, so this radioisotope does not move
its backend data. It reports AWS config entries that invoke aws-vault and the
default file backend directory when present.
