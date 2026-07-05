# yt-dlp Radioisotope Detector

This detector reports plaintext yt-dlp auth inputs.

yt-dlp can consume credentials from `~/.netrc` and from password options in
config files such as `~/.config/yt-dlp/config` and `~/.yt-dlp.conf`.

This radioisotope is detect-only because these auth inputs are generic request
state rather than a narrow package-owned credential store.
