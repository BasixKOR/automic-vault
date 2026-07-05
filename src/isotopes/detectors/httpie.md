# HTTPie Radioisotope Detector

This detector reports plaintext HTTPie named sessions.

HTTPie stores named sessions as JSON files under the config directory, normally
`~/.config/httpie/sessions/<host>/<name>.json`. Session data can include
authentication headers, prompted passwords, and cookies.

This radioisotope is detect-only because HTTPie session files are mutable
runtime state. A safe fix needs native session-store integration or a source
isotope that preserves session updates.
