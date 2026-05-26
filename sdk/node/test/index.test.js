'use strict'

const assert = require('node:assert/strict')
const fs = require('node:fs')
const net = require('node:net')
const os = require('node:os')
const path = require('node:path')
const test = require('node:test')

const {
  _internals
} = require('../index.js')

test('mode precedence uses AUTOMIC_VAULT_ENV, then NODE_ENV, then CI, then development', () => {
  assert.equal(_internals.resolveMode({ AUTOMIC_VAULT_ENV: 'production', NODE_ENV: 'development', CI: '1' }), 'production')
  assert.equal(_internals.resolveMode({ NODE_ENV: 'test', CI: '1' }), 'testing')
  assert.equal(_internals.resolveMode({ CI: '1' }), 'testing')
  assert.equal(_internals.resolveMode({}), 'development')
})

test('missing production env value throws named error', async () => {
  const oldEnv = { ...process.env }
  delete process.env.MISSING_SECRET
  process.env.AUTOMIC_VAULT_ENV = 'production'
  try {
    const { secret } = freshSdk()
    await assert.rejects(
      () => secret('MISSING_SECRET'),
      err => err.name === 'AutomicVaultMissingSecretError' && err.secret === 'MISSING_SECRET'
    )
  } finally {
    process.env = oldEnv
  }
})

test('production reads getenv and warns for unexpected baseline callsite', async () => {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'av-node-'))
  fs.mkdirSync(path.join(temp, '.config'))
  fs.writeFileSync(path.join(temp, '.config', 'automic-vault.json'), JSON.stringify({
    project_hash: 'hash',
    expected_callsites: []
  }))
  const oldCwd = process.cwd()
  const oldEnv = { ...process.env }
  const writes = []
  const oldWrite = process.stderr.write
  process.chdir(temp)
  process.env.AUTOMIC_VAULT_ENV = 'production'
  process.env.OPENAI_API_KEY = 'from-env'
  process.stderr.write = chunk => {
    writes.push(String(chunk))
    return true
  }
  try {
    const { secret } = freshSdk()
    assert.equal(await secret('OPENAI_API_KEY'), 'from-env')
    assert.equal(JSON.parse(writes.join('').trim()).type, 'automic_vault_unexpected_secret_usage')
  } finally {
    process.stderr.write = oldWrite
    process.chdir(oldCwd)
    process.env = oldEnv
  }
})

test('expected production callsite does not warn', () => {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'av-node-'))
  const backtrace = ['at app (./src/app.js:1:1)']
  const fingerprint = _internals.callsiteFingerprint('TOKEN', 'node', backtrace)
  fs.mkdirSync(path.join(temp, '.config'))
  fs.writeFileSync(path.join(temp, '.config', 'automic-vault.json'), JSON.stringify({
    project_hash: 'hash',
    expected_callsites: [{ fingerprint }]
  }))
  const writes = []
  _internals.warnIfUnexpectedCallsite('TOKEN', 'testing', backtrace, temp, {
    write: chunk => writes.push(String(chunk))
  })
  assert.deepEqual(writes, [])
})

test('development sends daemon request and returns response value', async () => {
  const socket = path.join(fs.mkdtempSync(path.join(os.tmpdir(), 'av-node-sock-')), 'dotenv.sock')
  const server = net.createServer(connection => {
    let data = ''
    connection.setEncoding('utf8')
    connection.on('data', chunk => {
      data += chunk
      if (!data.includes('\n')) return
      const request = JSON.parse(data.trim())
      assert.equal(request.type, 'secret_request')
      assert.equal(request.secret, 'TOKEN')
      assert.equal(request.runtime, 'node')
      connection.end(`${JSON.stringify({ type: 'secret_response', id: request.id, value: 'daemon-value' })}\n`)
    })
  })
  await new Promise(resolve => server.listen(socket, resolve))
  const oldEnv = { ...process.env }
  process.env.AUTOMIC_VAULT_ENV = 'development'
  process.env.AUTOMIC_VAULT_DOTENV_SOCKET = socket
  try {
    const { secret } = freshSdk()
    assert.equal(await secret('TOKEN'), 'daemon-value')
  } finally {
    process.env = oldEnv
    await new Promise(resolve => server.close(resolve))
  }
})

test('development reports unavailable daemon with command hint', async () => {
  const oldEnv = { ...process.env }
  process.env.AUTOMIC_VAULT_ENV = 'development'
  process.env.AUTOMIC_VAULT_DOTENV_SOCKET = path.join(os.tmpdir(), `missing-${Date.now()}.sock`)
  try {
    const { secret } = freshSdk()
    await assert.rejects(
      () => secret('TOKEN'),
      err => err.name === 'AutomicVaultDaemonError' && err.message.includes('av dotenv serve')
    )
  } finally {
    process.env = oldEnv
  }
})

function freshSdk () {
  delete require.cache[require.resolve('../index.js')]
  return require('../index.js')
}
