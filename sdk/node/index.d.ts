export class AutomicVaultMissingSecretError extends Error {
  constructor(secret: string)
  secret: string
}

export class AutomicVaultDaemonError extends Error {
  constructor(message: string)
}

export function secret(name: string): Promise<string>
