const { execFile } = require('node:child_process');
const { createHash } = require('node:crypto');
const { plugin } = require('varlock/plugin-lib');
const { ResolutionError, SchemaError } = plugin.ERRORS;

const MAX_SECRET_NAMES = 64;
const batches = new WeakMap();

const helper = process.env.AUTOMIC_VAULT_VARLOCK_HELPER
  || '/Applications/Automic Vault.app/Contents/Resources/AutomicVaultVarlockPlugin';

plugin.name = '@automicvault/varlock-plugin';
plugin.icon = 'mdi:shield-key-outline';

function activeSecretNames(graph) {
  const names = new Set();
  const visit = (resolver) => {
    if (!resolver) return;
    if (resolver.fnName === 'automicVault') names.add(resolver.meta.name);
    for (const child of resolver.childResolvers || []) visit(child);
  };
  for (const item of Object.values(graph.configSchema)) visit(item.valueResolver);
  return [...names].sort();
}

function schemaDigest(graph) {
  const sources = graph.sortedDataSources
    .filter((source) => source.fullPath && !source.disabled && !source.isAutoloadedValueSource)
    .map((source) => ({ path: source.fullPath, contents: source.rawContents }))
    .sort((a, b) => a.path.localeCompare(b.path));
  if (!sources.length || sources.some((source) => typeof source.contents !== 'string')) {
    throw new ResolutionError('Could not bind the active Varlock schema');
  }
  const hash = createHash('sha256');
  for (const source of sources) {
    hash.update(source.path);
    hash.update('\0');
    hash.update(source.contents);
    hash.update('\0');
  }
  return hash.digest('hex');
}

function requestBatch(graph) {
  const names = activeSecretNames(graph);
  if (!names.length || names.length > MAX_SECRET_NAMES) {
    throw new ResolutionError(`Expected between 1 and ${MAX_SECRET_NAMES} Automic Vault Secret Names`);
  }
  return new Promise((resolve, reject) => {
    execFile(
      helper,
      [schemaDigest(graph), ...names],
      { encoding: 'utf8', maxBuffer: 1024 * 1024 },
      (error, stdout, stderr) => {
        if (error) {
          reject(new ResolutionError(stderr.trim() || error.message));
          return;
        }
        try {
          const secrets = JSON.parse(stdout);
          if (!secrets || Array.isArray(secrets) || typeof secrets !== 'object'
              || Object.keys(secrets).sort().join('\0') !== names.join('\0')
              || names.some((name) => typeof secrets[name] !== 'string')) {
            throw new Error('response did not contain the exact requested Secret Names');
          }
          resolve(secrets);
        } catch (parseError) {
          reject(new ResolutionError(`Invalid Automic Vault response: ${parseError.message}`));
        }
      }
    );
  });
}

plugin.registerResolverFunction({
  name: 'automicVault',
  label: 'Request a Secret from Automic Vault',
  icon: plugin.icon,
  argsSchema: { type: 'mixed', arrayMinLength: 0, arrayMaxLength: 1 },
  process() {
    const argument = this.arrArgs?.[0];
    const name = argument ? argument.staticValue : this.parent?.key;
    if (argument && !argument.isStatic) {
      throw new SchemaError('Automic Vault Secret Names must be static');
    }
    if (!name) {
      throw new SchemaError('Could not infer the Automic Vault Secret Name');
    }
    if (typeof name !== 'string' || !/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) {
      throw new SchemaError('Expected a valid Automic Vault Secret Name');
    }
    return { name };
  },
  async resolve({ name }) {
    const graph = this.envGraph;
    if (!graph.isProcessEnvInjectionDisabled) {
      throw new SchemaError('The Automic Vault plugin requires # @disableProcessEnvInjection');
    }
    if (!batches.has(graph)) batches.set(graph, requestBatch(graph));
    return (await batches.get(graph))[name];
  },
});
