# sshpass Radioisotope

Detect-only coverage for sshpass password exposure.

sshpass can place SSH passwords in command history, process arguments, or
environment variables. This radioisotope reports obvious shell history use and
does not try to migrate password-based SSH workflows.
