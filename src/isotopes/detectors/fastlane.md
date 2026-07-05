# fastlane Radioisotope

Detect-only coverage for fastlane Spaceship session cookies.

fastlane stores Apple account passwords in the system keychain where possible,
but Spaceship session cookies can still live in plaintext files. This
radioisotope reports those files without changing fastlane's auth flow.
