import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  WORKSPACE_COMMANDS,
  applyWorkspaceSearchBias,
  startupExecutionSummary,
  workspacePinsFromActivationPlan,
  workspaceSearchResults
} from '../dist-tests/lib/workspaces.js';

const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
const commandsSource = readFileSync(new URL('../src/ipc/commands.ts', import.meta.url), 'utf8');

function activationPlan() {
  return {
    workspace: {
      id: 'jasonshell',
      name: 'JasonShell',
      rootPath: 'C:\\dev\\jasonshell',
      aliases: ['shell', 'jshell'],
      pins: [],
      toolDefaults: { terminal: 'Windows Terminal', editor: 'VS Code', shell: 'pwsh' },
      tasks: [],
      startup: { mode: 'suggestOnly', taskIds: ['validate'], commands: [], env: [] },
      restoration: { status: 'reserved-not-implemented' }
    },
    layout: {
      activeWorkspaceId: 'jasonshell',
      rootPath: 'C:\\dev\\jasonshell',
      aliases: ['shell', 'jshell'],
      windowAppRestorationStatus: 'reserved-not-implemented'
    },
    search: {
      biasRoots: ['C:\\dev\\jasonshell'],
      aliases: ['shell', 'jshell'],
      resultBoost: 32
    },
    pins: {
      topBar: [
        { id: 'src', label: 'Source', path: 'C:\\dev\\jasonshell\\src', workspaceId: 'jasonshell' }
      ]
    },
    tasks: {
      exposed: [
        {
          id: 'validate',
          name: 'Validate',
          command: 'npm',
          args: ['run', 'validate'],
          cwd: 'C:\\dev\\jasonshell',
          pinned: true,
          willExecuteOnActivation: false
        }
      ]
    },
    startup: {
      mode: 'suggestOnly',
      willExecute: false,
      reason: 'workspace activation only returns a startup plan',
      taskIds: ['validate'],
      commands: [],
      env: []
    },
    restoration: { status: 'reserved-not-implemented' }
  };
}

test('workspace IPC wrapper uses centralized command names registered by Rust', () => {
  assert.deepEqual(WORKSPACE_COMMANDS, {
    list: 'list_workspaces',
    create: 'create_workspace',
    update: 'update_workspace',
    delete: 'delete_workspace',
    activate: 'activate_workspace'
  });
  for (const command of Object.values(WORKSPACE_COMMANDS)) {
    assert.match(commandsSource, new RegExp(command));
    assert.match(mainSource, new RegExp(`workspaces::${command}`));
  }
});

test('activation plan exposes top-bar pins and searchable workspace task results', () => {
  const plan = activationPlan();

  assert.deepEqual(workspacePinsFromActivationPlan(plan), [
    {
      id: 'workspace:jasonshell:src',
      name: 'Source',
      path: 'C:\\dev\\jasonshell\\src'
    }
  ]);
  assert.deepEqual(
    workspaceSearchResults(plan).map((result) => result.id),
    ['workspace:jasonshell', 'workspace:jasonshell:pin:src', 'workspace:jasonshell:task:validate']
  );
});

test('workspace search bias boosts only results under active workspace roots', () => {
  const biased = applyWorkspaceSearchBias(
    [
      {
        id: 'file:inside',
        kind: 'file',
        title: 'TopBar.svelte',
        subtitle: 'File',
        terms: 'topbar',
        priority: 50,
        path: 'C:/dev/jasonshell/src/components/TopBar.svelte'
      },
      {
        id: 'file:outside',
        kind: 'file',
        title: 'Other',
        subtitle: 'File',
        terms: 'other',
        priority: 50,
        path: 'C:/dev/other/README.md'
      }
    ],
    activationPlan()
  );

  assert.equal(biased[0].priority, 82);
  assert.match(biased[0].terms, /active workspace/);
  assert.equal(biased[1].priority, 50);
});

test('startup summary makes non-execution explicit', () => {
  assert.match(startupExecutionSummary(activationPlan()), /Startup execution blocked/);
});
