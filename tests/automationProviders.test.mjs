import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  AUTOMATION_COMMANDS,
  AUTOMATION_FORWARDING_STATUS,
  LOCAL_AUTOMATION_OPT_IN_FLAG,
  assertSafeForwardingContract,
  destructiveAutomationConfirmation
} from '../dist-tests/lib/automation.js';
import {
  PROVIDER_BUDGET_LIMITS,
  PROVIDER_COMMANDS,
  assertNoExecutableProviderConfig,
  assertNoProviderSecrets,
  assertSafeProviderRegistry,
  defaultProviderBudget,
  normalizeProviderConfig,
  validateProviderBudget
} from '../dist-tests/lib/providerContracts.js';

const commandsSource = readFileSync(new URL('../src/ipc/commands.ts', import.meta.url), 'utf8');
const contractsSource = readFileSync(new URL('../src-tauri/src/contracts.rs', import.meta.url), 'utf8');
const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
const automationSource = readFileSync(new URL('../src-tauri/src/automation.rs', import.meta.url), 'utf8');
const providersSource = readFileSync(new URL('../src-tauri/src/providers.rs', import.meta.url), 'utf8');

test('automation IPC contracts are registered without enabling arbitrary execution', () => {
  assert.equal(LOCAL_AUTOMATION_OPT_IN_FLAG, '--allow-local-automation');
  assert.equal(AUTOMATION_COMMANDS.parseCli, 'parse_automation_cli');
  assert.equal(AUTOMATION_COMMANDS.validateRequest, 'validate_automation_request');
  assert.equal(AUTOMATION_COMMANDS.forwardingContract, 'get_single_instance_forwarding_contract');

  for (const command of Object.values(AUTOMATION_COMMANDS)) {
    assert.match(commandsSource, new RegExp(command));
    assert.match(contractsSource, new RegExp(command));
    assert.match(mainSource, new RegExp(command));
  }

  assert.match(automationSource, /planned-not-wired/);
  assert.match(automationSource, /executes_forwarded_payloads: false/);
  assert.match(automationSource, /arbitrary_plugin_execution_allowed: false/);
});

test('automation wrapper enforces safe forwarding contract shape', () => {
  assertSafeForwardingContract({
    status: AUTOMATION_FORWARDING_STATUS,
    transport: 'local-single-instance-forwarding-plan',
    acceptsArgvOnly: true,
    requiresLocalOptIn: true,
    requiresAuthenticatedDestructiveActions: true,
    executesForwardedPayloads: false,
    arbitraryPluginExecutionAllowed: false
  });
  assert.equal(
    destructiveAutomationConfirmation({ kind: 'deleteWorkspace', target: 'main' }),
    'delete-workspace:main'
  );
  assert.throws(
    () =>
      assertSafeForwardingContract({
        status: AUTOMATION_FORWARDING_STATUS,
        transport: 'unsafe',
        acceptsArgvOnly: true,
        requiresLocalOptIn: true,
        requiresAuthenticatedDestructiveActions: true,
        executesForwardedPayloads: true,
        arbitraryPluginExecutionAllowed: false
      }),
    /must not execute/
  );
});

test('provider contract rejects secret and executable/plugin config in TS wrapper', () => {
  assert.deepEqual(defaultProviderBudget(), {
    maxResults: PROVIDER_BUDGET_LIMITS.defaultMaxResults,
    timeoutMs: PROVIDER_BUDGET_LIMITS.defaultTimeoutMs
  });
  assert.doesNotThrow(() =>
    normalizeProviderConfig({
      id: 'workspace-files',
      type: 'workspace-files',
      config: { scope: 'active-workspace' }
    })
  );
  assert.throws(() => validateProviderBudget({ maxResults: 101, timeoutMs: 50 }), /maxResults/);
  assert.throws(() => assertNoProviderSecrets({ apiToken: 'abc' }), /secret-like key/);
  assert.throws(() => assertNoProviderSecrets({ header: 'Bearer abc.def' }), /secret-like value/);
  assert.throws(() => assertNoExecutableProviderConfig({ pluginPath: 'C:\\plugins\\bad.dll' }), /executable\/plugin/);
});

test('provider IPC contracts are registered and deny arbitrary plugin execution', () => {
  assert.equal(PROVIDER_COMMANDS.resolveRegistry, 'resolve_provider_registry');
  assert.match(commandsSource, /resolve_provider_registry/);
  assert.match(contractsSource, /resolve_provider_registry/);
  assert.match(mainSource, /resolve_provider_registry/);
  assert.match(providersSource, /arbitrary_plugin_execution_allowed: false/);
  assert.match(providersSource, /external-executable/);
  assertSafeProviderRegistry({
    providers: [],
    totalMaxResults: 0,
    maxTimeoutMs: 0,
    arbitraryPluginExecutionAllowed: false
  });
  assert.throws(
    () =>
      assertSafeProviderRegistry({
        providers: [],
        totalMaxResults: 0,
        maxTimeoutMs: 0,
        arbitraryPluginExecutionAllowed: true
      }),
    /must not allow/
  );
});
