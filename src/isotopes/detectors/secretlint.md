# Secretlint Radioisotope

Detect-only coverage for unmasked Secretlint output.

Secretlint masks secrets by default. This radioisotope reports obvious
`--no-maskSecrets` history and common local report files that may contain
unmasked findings without changing Secretlint's scan behavior.
