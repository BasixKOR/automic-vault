# curl Radioisotope Detector

This detector reports plaintext curl credentials in user-level config files.
It does not currently migrate credentials or modify curl.

Detected hazards:

- `~/.netrc` machine passwords
- `~/.curlrc` `user`, `proxy-user`, bearer token, and Authorization settings
