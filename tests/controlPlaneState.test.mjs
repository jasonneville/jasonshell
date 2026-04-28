import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  buildControlPlaneViewModel,
  controlPlaneActionLabel,
  controlPlaneKeyActionFromEvent,
  controlPlaneSectionTabLabel,
  filterControlPlaneSections,
  nextControlPlaneSectionId
} from '../dist-tests/features/control-plane/controlPlaneState.js';

const componentSource = readFileSync(new URL('../src/components/ControlPlaneSurface.svelte', import.meta.url), 'utf8');
const cssSource = readFileSync(new URL('../src/components/ControlPlaneSurface.css', import.meta.url), 'utf8');

function workspace(id, overrides = {}) {
  return {
    id,
    name: id === 'jasonshell' ? 'JasonShell' : 'External',
    rootPath: id === 'jasonshell' ? 'C:\\dev\\jasonshell' : 'D:\\other',
    aliases: [],
    pins: [{ id: 'src', label: 'Source', path: 'C:\\dev\\jasonshell\\src', kind: 'folder' }],
    toolDefaults: {},
    tasks: [
      {
        id: 'validate',
        name: 'Validate',
        command: 'npm',
        args: ['run', 'validate'],
        cwd: null,
        env: [],
        exposeInSearch: true,
        pinned: true
      }
    ],
    startup: { mode: 'manualOnly', taskIds: [], commands: [], env: [] },
    restoration: { status: 'reserved-not-implemented' },
    ...overrides
  };
}

const settings = {
  schema: 'jasonshell.settings',
  version: 1,
  ui: {
    activeWorkspaceId: 'jasonshell',
    enableDiagnosticsExport: false
  },
  workspaces: [workspace('jasonshell'), workspace('external')],
  taskHistory: []
};

test('derives settings and dashboard sections from existing frontend contracts without rendering secrets', () => {
  const model = buildControlPlaneViewModel({
    settings,
    gitStatuses: {
      jasonshell: {
        isRepository: true,
        branch: 'main',
        upstream: 'origin/main',
        headOid: 'abc',
        isClean: false,
        hasChanges: true,
        ahead: 1,
        behind: 0,
        hasConflicts: false,
        isRebasing: false,
        isMerging: false,
        summary: '2 modified'
      }
    },
    taskHistory: [
      {
        taskId: 'validate',
        workspaceId: 'jasonshell',
        workspacePath: 'C:\\dev\\jasonshell',
        label: 'Validate',
        executable: 'npm',
        args: ['run', 'validate', 'apiToken=abc', '--token', 'def', 'Bearer raw', 'sk-rawsecret'],
        processId: 20,
        startedAtEpochMs: Date.now(),
        finishedAtEpochMs: null,
        exitCode: 0,
        canceled: false
      }
    ],
    taskProcessMetadata: [
      {
        taskId: 'validate',
        processId: 20,
        workspaceId: 'jasonshell',
        workspacePath: 'C:\\dev\\jasonshell',
        label: 'Validate',
        startedAtEpochMs: Date.now()
      }
    ],
    processes: [
      {
        pid: 20,
        name: 'node',
        cpuPercent: 10,
        memoryBytes: 1024 * 1024 * 120,
        status: 'running',
        isKillable: true,
        workspaceHint: { kind: 'path-associated', label: 'jasonshell', path: 'C:\\dev\\jasonshell', source: 'process-path' }
      }
    ],
    providerResponse: {
      query: 'settings github_pat_rawsecret',
      results: [],
      groups: [
        { providerId: 'settings', label: 'Settings', results: [{ id: 'a' }, { id: 'b' }] },
        { providerId: 'processes', label: 'Processes', results: [{ id: 'c' }] }
      ]
    },
    providerBudget: { perProviderLimit: 2, totalLimit: 4 }
  });

  assert.deepEqual(model.sections.map((section) => section.id), [
    'settings',
    'workspaces',
    'git',
    'tasks',
    'processes',
    'providers'
  ]);
  assert.equal(model.totals.workspaceCount, 2);
  assert.equal(model.totals.dirtyRepositoryCount, 1);
  assert.equal(model.totals.runningTaskCount, 1);
  assert.match(model.sections.find((section) => section.id === 'settings').status, /jasonshell\.settings v1/);
  assert.doesNotMatch(JSON.stringify(model), /apiToken=abc/);
  assert.doesNotMatch(JSON.stringify(model), /--token def/);
  assert.doesNotMatch(JSON.stringify(model), /Bearer raw/);
  assert.doesNotMatch(JSON.stringify(model), /sk-rawsecret/);
  assert.doesNotMatch(JSON.stringify(model), /github_pat_rawsecret/);
  assert.match(JSON.stringify(model), /apiToken=\[REDACTED\]/);
  assert.match(JSON.stringify(model), /--token \[REDACTED\]/);
  assert.match(JSON.stringify(model), /Bearer \[REDACTED\]/);
});

test('bounds rendering data while preserving overflow counts and provider budgets', () => {
  const model = buildControlPlaneViewModel({
    workspaces: Array.from({ length: 8 }, (_, index) => workspace(`workspace-${index}`, { name: `Workspace ${index}` })),
    processes: Array.from({ length: 9 }, (_, index) => ({
      pid: index + 1,
      name: `process-${index}`,
      cpuPercent: index,
      memoryBytes: 1024 * 1024 * index,
      status: 'running',
      isKillable: index > 0
    })),
    providerResponse: {
      query: '',
      results: Array.from({ length: 5 }, (_, index) => ({ id: `${index}` })),
      groups: Array.from({ length: 7 }, (_, index) => ({
        providerId: index === 0 ? 'settings' : 'processes',
        label: `Provider ${index}`,
        results: Array.from({ length: index + 1 }, (_, resultIndex) => ({ id: `${index}-${resultIndex}` }))
      }))
    },
    providerBudget: { perProviderLimit: 3, totalLimit: 5 },
    itemLimit: 3
  });

  const workspaces = model.sections.find((section) => section.id === 'workspaces');
  const processes = model.sections.find((section) => section.id === 'processes');
  const providers = model.sections.find((section) => section.id === 'providers');

  assert.equal(workspaces.items.length, 4);
  assert.match(workspaces.items.at(-1).title, /5 more workspace/);
  assert.equal(processes.items.length, 4);
  assert.match(processes.items[0].title, /process-8/);
  assert.match(providers.status, /Bounded to 5 total and 3 per provider/);
  assert.equal(providers.items.length, 4);
});

test('filters sections and exposes keyboard navigation actions for accessible control-plane tabs', () => {
  const model = buildControlPlaneViewModel({ settings, activeSectionId: 'git' });
  const filtered = filterControlPlaneSections(model.sections, 'git repository');

  assert.deepEqual(filtered.map((section) => section.id), ['git']);
  assert.equal(nextControlPlaneSectionId(model.sections, 'settings', 'next'), 'workspaces');
  assert.equal(nextControlPlaneSectionId(model.sections, 'settings', 'previous'), 'providers');
  assert.equal(controlPlaneKeyActionFromEvent({ key: 'ArrowRight' }), 'focus-next-section');
  assert.equal(controlPlaneKeyActionFromEvent({ key: 'ArrowUp' }), 'focus-previous-section');
  assert.equal(controlPlaneKeyActionFromEvent({ key: 'Home' }), 'focus-first-section');
  assert.equal(controlPlaneKeyActionFromEvent({ key: 'End' }), 'focus-last-section');
  assert.equal(controlPlaneKeyActionFromEvent({ key: 'r', ctrlKey: true }), 'refresh-active-section');
  assert.equal(controlPlaneKeyActionFromEvent({ key: 'Escape' }), 'close-panel');

  const tabLabel = controlPlaneSectionTabLabel(model.sections[0], true);
  assert.match(tabLabel, /Settings/);
  assert.match(tabLabel, /selected/);
  assert.match(controlPlaneActionLabel(model.sections[0], model.sections[0].actions[0]), /Refresh: Settings/);
});

test('control-plane component includes semantic settings and dashboard regions without adding IPC authority', () => {
  assert.match(componentSource, /aria-labelledby="control-plane-title"/);
  assert.match(componentSource, /role="tablist"/);
  assert.match(componentSource, /aria-selected=/);
  assert.match(componentSource, /Control-plane summaries never render persisted secret values|Secrets and unbounded source lists/);
  assert.doesNotMatch(componentSource, /invoke\(/);
  assert.doesNotMatch(componentSource, /loadShellSettings\(/);
  assert.doesNotMatch(componentSource, /saveShellSettings\(/);
  assert.match(cssSource, /@media \(max-width: 760px\)/);
});
