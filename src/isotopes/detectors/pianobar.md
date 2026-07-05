# pianobar Radioisotope Detector

This detector reports plaintext Pandora passwords in pianobar config files.

pianobar reads a user config file from the XDG config directory, normally
`~/.config/pianobar/config`. That file can contain a `password` entry.

This radioisotope is detect-only because pianobar does not expose a clean
temporary credential file boundary that preserves normal config behavior.
