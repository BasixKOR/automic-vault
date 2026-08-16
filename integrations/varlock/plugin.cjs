const { execFile } = require('node:child_process');
const { plugin } = require('varlock/plugin-lib');
const { ResolutionError, SchemaError } = plugin.ERRORS;

const helper = process.env.AUTOMIC_VAULT_VARLOCK_HELPER
  || '/Applications/Automic Vault.app/Contents/Resources/AutomicVaultVarlockPlugin';

plugin.name = '@automicvault/varlock-plugin';
plugin.icon = 'mdi:shield-key-outline';

plugin.registerResolverFunction({
  name: 'automicVault',
  label: 'Request a Secret from Automic Vault',
  icon: plugin.icon,
  argsSchema: { type: 'mixed', arrayMinLength: 0, arrayMaxLength: 1 },
  process() {
    const argument = this.arrArgs?.[0];
    const inferredName = this.parent?.key;
    if (!argument && !inferredName) {
      throw new SchemaError('Could not infer the Automic Vault Secret Name');
    }
    return { argument, inferredName };
  },
  async resolve({ argument, inferredName }) {
    const name = argument ? await argument.resolve() : inferredName;
    if (typeof name !== 'string' || !/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) {
      throw new SchemaError('Expected a valid Automic Vault Secret Name');
    }
    return new Promise((resolve, reject) => {
      execFile(helper, [name], { encoding: 'utf8', maxBuffer: 1024 * 1024 }, (error, stdout, stderr) => {
        if (error) {
          reject(new ResolutionError(stderr.trim() || error.message));
        } else {
          resolve(stdout);
        }
      });
    });
  },
});
