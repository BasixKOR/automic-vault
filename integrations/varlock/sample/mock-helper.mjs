#!/usr/bin/env node

const [digest, ...names] = process.argv.slice(2);
const expected = process.env.AUTOMIC_VAULT_VARLOCK_EXPECTED_NAMES?.split(',').sort();

if (!/^[a-f0-9]{64}$/.test(digest || '')
    || names.join('\0') !== expected?.join('\0')) {
  console.error('expected one complete Varlock Secret batch');
  process.exit(1);
}

process.stdout.write(JSON.stringify(Object.fromEntries(names.map((name) => [name, name]))));
