import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';

export const PROVIDER_BUDGET_LIMITS = {
  defaultMaxResults: 25,
  defaultTimeoutMs: 150,
  maxResults: 100,
  maxTimeoutMs: 500
} as const;

export type ProviderType = 'workspace-files' | 'git-changes' | 'task-history' | 'commands' | 'settings' | 'processes';

export interface ProviderBudget {
  maxResults: number;
  timeoutMs: number;
}

export interface ProviderConfig {
  id: string;
  type: ProviderType;
  disabled?: boolean;
  budget?: ProviderBudget;
  config?: unknown;
}

export interface ProviderRegistryConfig {
  providers: ProviderConfig[];
}

export interface ResolvedProvider {
  id: string;
  type: ProviderType;
  disabled: boolean;
  budget: ProviderBudget;
}

export interface ProviderRegistry {
  providers: ResolvedProvider[];
  totalMaxResults: number;
  maxTimeoutMs: number;
  arbitraryPluginExecutionAllowed: boolean;
}

export const PROVIDER_COMMANDS = {
  resolveRegistry: IPC_COMMANDS.resolveProviderRegistry
} as const;

const SECRET_KEY_PATTERN = /(token|secret|password|credential|api[_-]?key|authorization|cookie)/iu;
const EXECUTABLE_PROVIDER_KEY_PATTERN = /^(executable|command|script|pluginPath|dllPath|entrypoint)$/iu;
const SECRET_VALUE_PATTERN = /(bearer\s+|ghp_|gho_|github_pat_|xoxb-|sk-|akia)/iu;

export function defaultProviderBudget(): ProviderBudget {
  return {
    maxResults: PROVIDER_BUDGET_LIMITS.defaultMaxResults,
    timeoutMs: PROVIDER_BUDGET_LIMITS.defaultTimeoutMs
  };
}

export function normalizeProviderConfig(provider: ProviderConfig): ProviderConfig {
  const budget = provider.budget ?? defaultProviderBudget();
  validateProviderBudget(budget);
  assertNoProviderSecrets(provider.config);
  assertNoExecutableProviderConfig(provider.config);
  return {
    ...provider,
    id: provider.id.trim(),
    disabled: provider.disabled ?? false,
    budget
  };
}

export function validateProviderBudget(budget: ProviderBudget): void {
  if (!Number.isInteger(budget.maxResults) || budget.maxResults < 1 || budget.maxResults > PROVIDER_BUDGET_LIMITS.maxResults) {
    throw new Error(`Provider maxResults must be between 1 and ${PROVIDER_BUDGET_LIMITS.maxResults}`);
  }
  if (!Number.isInteger(budget.timeoutMs) || budget.timeoutMs < 1 || budget.timeoutMs > PROVIDER_BUDGET_LIMITS.maxTimeoutMs) {
    throw new Error(`Provider timeoutMs must be between 1 and ${PROVIDER_BUDGET_LIMITS.maxTimeoutMs}`);
  }
}

export function assertNoProviderSecrets(value: unknown, path: string[] = ['providers']): void {
  if (!value || typeof value !== 'object') {
    if (typeof value === 'string' && SECRET_VALUE_PATTERN.test(value)) {
      throw new Error(`Provider config must not store secret-like value at ${path.join('.')}`);
    }
    return;
  }

  if (Array.isArray(value)) {
    value.forEach((child, index) => assertNoProviderSecrets(child, [...path, String(index)]));
    return;
  }

  for (const [key, child] of Object.entries(value)) {
    const nextPath = [...path, key];
    if (SECRET_KEY_PATTERN.test(key)) {
      throw new Error(`Provider config must not store secret-like key: ${nextPath.join('.')}`);
    }
    assertNoProviderSecrets(child, nextPath);
  }
}

export function assertNoExecutableProviderConfig(value: unknown): void {
  if (!value || typeof value !== 'object') {
    return;
  }
  if (Array.isArray(value)) {
    value.forEach(assertNoExecutableProviderConfig);
    return;
  }

  for (const [key, child] of Object.entries(value)) {
    if (EXECUTABLE_PROVIDER_KEY_PATTERN.test(key)) {
      throw new Error(`Provider config must not declare executable/plugin loading key: ${key}`);
    }
    assertNoExecutableProviderConfig(child);
  }
}

export function assertSafeProviderRegistry(registry: ProviderRegistry): void {
  if (registry.arbitraryPluginExecutionAllowed) {
    throw new Error('Provider registry must not allow arbitrary plugin execution');
  }
}

export async function resolveProviderRegistry(config: ProviderRegistryConfig): Promise<ProviderRegistry> {
  const normalized = { providers: config.providers.map(normalizeProviderConfig) };
  const registry = await invoke<ProviderRegistry>(PROVIDER_COMMANDS.resolveRegistry, { config: normalized });
  assertSafeProviderRegistry(registry);
  return registry;
}
