# maestro Detector

Reports when:
- Maestro Cloud token is stored in a plaintext token file.
- Maestro Studio OpenAI token is stored in a plaintext token file.

## Detection Caveats

- Scans `~/.mobiledev/authtoken` and `~/.mobiledev/openaitoken`.
