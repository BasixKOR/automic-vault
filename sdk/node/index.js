'use strict'

const crypto = require('node:crypto')
const fs = require('node:fs')
const net = require('node:net')
const os = require('node:os')
const path = require('node:path')

class AutomicVaultMissingSecretError extends Error {
  constructor (secret) {
    super(`missing Automic Vault secret ${secret}`)
    this.name = 'AutomicVaultMissingSecretError'
    this.secret = secret
  }
}

class AutomicVaultDaemonError extends Error {
  constructor (message) {
    super(message)
    this.name = 'AutomicVaultDaemonError'
  }
}

async function secret (name) {
  validateSecretName(name)
  const mode = resolveMode(process.env)
  const backtrace = captureBacktrace()
  if (mode === 'development') {
    return resolveFromDaemon(name, mode, backtrace)
  }
  return resolveFromEnvironment(name, mode, backtrace)
}

function validateSecretName (name) {
  if (typeof name !== 'string' || !/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) {
    throw new TypeError('secret name must be a valid environment variable name')
  }
}

function resolveMode (env) {
  const raw = env.AUTOMIC_VAULT_ENV || env.NODE_ENV || (env.CI ? 'testing' : 'development')
  return normalizeMode(raw)
}

function normalizeMode (value) {
  switch (String(value || '').toLowerCase()) {
    case 'prod':
    case 'production':
      return 'production'
    case 'ci':
      return 'ci'
    case 'test':
    case 'testing':
      return 'testing'
    case 'dev':
    case 'development':
    default:
      return 'development'
  }
}

function resolveFromEnvironment (name, mode, backtrace) {
  const value = process.env[name]
  if (value == null) {
    throw new AutomicVaultMissingSecretError(name)
  }
  warnIfUnexpectedCallsite(name, mode, backtrace, process.cwd(), process.stderr)
  return value
}

function resolveFromDaemon (name, mode, backtrace) {
  const socketPath = dotenvSocketPath(process.env)
  const id = `${process.pid}-${Date.now()}-${Math.random().toString(16).slice(2)}`
  const payload = {
    type: 'secret_request',
    id,
    secret: name,
    cwd: process.cwd(),
    runtime: 'node',
    pid: process.pid,
    mode,
    backtrace
  }

  return new Promise((resolve, reject) => {
    const socket = net.createConnection(socketPath)
    let buffer = ''
    socket.setEncoding('utf8')
    socket.on('connect', () => {
      socket.write(`${JSON.stringify(payload)}\n`)
    })
    socket.on('data', chunk => {
      buffer += chunk
      const index = buffer.indexOf('\n')
      if (index < 0) return
      const line = buffer.slice(0, index)
      socket.end()
      let response
      try {
        response = JSON.parse(line)
      } catch (err) {
        reject(new AutomicVaultDaemonError(`invalid dotenv daemon response: ${err.message}`))
        return
      }
      if (response.type === 'secret_response' && response.id === id) {
        resolve(response.value)
      } else if (response.type === 'error') {
        reject(new AutomicVaultDaemonError(response.message || 'dotenv daemon returned an error'))
      } else {
        reject(new AutomicVaultDaemonError('dotenv daemon returned an unexpected response'))
      }
    })
    socket.on('error', err => {
      if (err.code === 'ENOENT' || err.code === 'ECONNREFUSED') {
        reject(new AutomicVaultDaemonError(`dotenv daemon unavailable at ${socketPath}; run: av dotenv serve`))
      } else {
        reject(new AutomicVaultDaemonError(`dotenv daemon error: ${err.message}`))
      }
    })
  })
}

function dotenvSocketPath (env) {
  if (env.AUTOMIC_VAULT_DOTENV_SOCKET) return env.AUTOMIC_VAULT_DOTENV_SOCKET
  const home = os.homedir()
  return path.join(home, 'Library', 'Application Support', 'Automic Vault', 'dotenv.sock')
}

function captureBacktrace () {
  const stack = new Error().stack || ''
  return stack
    .split('\n')
    .slice(2)
    .map(line => line.trim())
    .filter(line => line && !line.includes(`${path.sep}sdk${path.sep}node${path.sep}index.js`))
}

function warnIfUnexpectedCallsite (name, mode, backtrace, cwd, stream) {
  const root = findMetadataRoot(cwd)
  if (!root) return
  const metadata = readMetadata(root)
  if (!metadata) return
  const normalized = normalizeBacktrace(backtrace, root)
  const fingerprint = callsiteFingerprint(name, 'node', normalized)
  const expected = Array.isArray(metadata.expected_callsites) &&
    metadata.expected_callsites.some(entry => entry && entry.fingerprint === fingerprint)
  if (expected) return

  stream.write(`${JSON.stringify({
    type: 'automic_vault_unexpected_secret_usage',
    secret: name,
    mode,
    runtime: 'node',
    project_hash: metadata.project_hash,
    fingerprint,
    backtrace: normalized
  })}\n`)
}

function findMetadataRoot (cwd) {
  let current = path.resolve(cwd)
  while (true) {
    const candidate = path.join(current, '.config', 'automic-vault.json')
    if (fs.existsSync(candidate)) return current
    const parent = path.dirname(current)
    if (parent === current) return null
    current = parent
  }
}

function readMetadata (root) {
  try {
    return JSON.parse(fs.readFileSync(path.join(root, '.config', 'automic-vault.json'), 'utf8'))
  } catch {
    return null
  }
}

function normalizeBacktrace (backtrace, root) {
  const prefix = path.resolve(root)
  return backtrace
    .map(line => String(line).replaceAll(prefix, '.'))
    .filter(line => line.trim())
}

function callsiteFingerprint (secretName, runtime, normalizedBacktrace) {
  const hash = crypto.createHash('sha256')
  hash.update(secretName)
  hash.update(Buffer.from([0]))
  hash.update(runtime)
  for (const frame of normalizedBacktrace) {
    hash.update(Buffer.from([0]))
    hash.update(frame)
  }
  return hash.digest('hex')
}

module.exports = {
  AutomicVaultDaemonError,
  AutomicVaultMissingSecretError,
  secret,
  _internals: {
    callsiteFingerprint,
    captureBacktrace,
    dotenvSocketPath,
    findMetadataRoot,
    normalizeBacktrace,
    resolveMode,
    warnIfUnexpectedCallsite
  }
}
