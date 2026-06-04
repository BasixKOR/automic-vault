- Commit at sensible intervals.

## `db.json` Schema Compatibility

- Treat the public `/db.json` contract as additive and backward-compatible.
- Do not bump `scripts/build-db.py` `SCHEMA_VERSION` or
  `src/lib/rs/lib.rs` `DB_SCHEMA_VERSION` for additive fields.
- Additive database changes must preserve existing field names, field types,
  and field meanings. New fields should be optional or have defaults in
  consumers.
- Existing clients accept database schemas up to their compiled maximum and
  reject future schemas, so a schema bump makes the remote database unusable
  for older clients.
- If a breaking database shape is truly required, publish it under a new public
  path such as `/db.v2.json` or `/v2/db.json` and keep `/db.json`
  backward-compatible.
- The schema value is a format-family compatibility marker, not a routine
  revision counter.

## Package Website Origin

- Package catalog routes under `/pkg/` and localized equivalents are served by
  the Rust `av-web` service from the private SQLite artifact at
  `cache/pkg.sqlite` locally and `/var/lib/automic-vault-web/pkg.sqlite` on
  Atlas.
- Do not restore the old static `www/pkg/**` or `www/pagefind/**` upload path.
  S3 remains the default static website origin, but package pages, package
  sitemaps, package CSS/JS, markdown alternates, and package search are an
  Atlas origin concern.
- Use `scripts/generate-pkg-sqlite.py` to build the package-origin artifact and
  `scripts/deploy-pkg-origin.sh` to deploy `av-web` plus `pkg.sqlite`.
- Keep public `/db.json` backward-compatible. The package SQLite database is
  private deployment data and must not drive a public schema bump.
