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
