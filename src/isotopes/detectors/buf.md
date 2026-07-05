# Buf Radioisotope

`buf registry login` stores Buf Schema Registry credentials in `~/.netrc` by
default. Buf also supports `BUF_TOKEN`, so the radioisotope migrates the
default `buf.build` token into the macOS keychain and injects it only while
`buf` runs.

The migration removes the matching single-line `machine buf.build ... password
...` entry from `~/.netrc` and preserves unrelated machines.

## Caveats

- We currently migrate `buf.build` and legacy `go.buf.build` `.netrc` entries
  only.
- Multi-line or custom `.netrc` layouts are detected conservatively and are not
  rewritten.
- Direct execution of the original binary will not receive credentials.
