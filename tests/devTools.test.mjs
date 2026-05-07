import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  DEV_TOOL_COMMANDS,
  TASK_EVENTS,
  editorLaunchRequestFromWorkspace,
  rankTaskHistoryForWorkspace,
  terminalLaunchRequestFromWorkspace,
  workspaceTaskRequest
} from '../dist-tests/lib/devTools.js';

const commandsSource = readFileSync(new URL('../src/ipc/commands.ts', import.meta.url), 'utf8');
const eventsSource = readFileSync(new URL('../src/ipc/events.ts', import.meta.url), 'utf8');
const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
const rustToolPlansSource = readFileSync(
  new URL('../src-tauri/src/dev_tools/tool_plans.rs', import.meta.url),
  'utf8'
);
const rustTaskSource = readFileSync(new URL('../src-tauri/src/dev_tools/task_runner.rs', import.meta.url), 'utf8');
const capabilitySource = [
  '../src-tauri/capabilities/default.json',
  '../src-tauri/capabilities/bottom-bar.json',
  '../src-tauri/capabilities/stack-popup.json',
  '../src-tauri/capabilities/process-manager.json'
]
  .map((path) => readFileSync(new URL(path, import.meta.url), 'utf8'))
  .join('\n');

function workspace() {
  return {
    id: 'jasonshell',
    name: 'JasonShell',
    rootPath: 'C:\\dev\\jasonshell',
    aliases: ['shell'],
    pins: [],
    toolDefaults: { terminal: 'Windows Terminal', editor: 'VS Code', shell: 'pwsh' },
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
    restoration: { status: 'reserved-not-implemented' }
  };
}

test('developer tooling IPC constants and task events are centralized and registered', () => {
  assert.deepEqual(DEV_TOOL_COMMANDS, {
      buildTerminalLaunchPlan: 'build_terminal_launch_plan',
      buildEditorLaunchPlan: 'build_editor_launch_plan',
      getWorkspaceGitStatus: 'get_workspace_git_status',
      spawnWorkspaceTask: 'spawn_workspace_task',
      cancelWorkspaceTask: 'cancel_workspace_task',
    listWorkspaceTaskHistory: 'list_workspace_task_history',
    listJasonshellTaskProcessMetadata: 'list_jasonshell_task_process_metadata'
  });
  assert.deepEqual(TASK_EVENTS, {
    started: 'task:started',
    output: 'task:output',
    completed: 'task:completed'
  });
  for (const command of Object.values(DEV_TOOL_COMMANDS)) {
    assert.match(commandsSource, new RegExp(command));
    assert.match(mainSource, new RegExp(command));
  }
  for (const event of Object.values(TASK_EVENTS)) {
    assert.match(eventsSource, new RegExp(event));
  }
});

test('top bar no longer exposes project context launcher', () => {
  const topBarSource = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
  const commandIndex = topBarSource.indexOf('class="command-control"');

  assert.ok(commandIndex > 0);
  assert.doesNotMatch(topBarSource, /project-context-control|project-context-button|showProjectContextPanel/);
  assert.doesNotMatch(commandsSource, /launchProjectContext|project_context/);
  assert.doesNotMatch(mainSource, /project_context/);
});

test('tool launch requests derive safe terminal and editor argv templates from workspace state', () => {
  const terminal = terminalLaunchRequestFromWorkspace(workspace());
  const editor = editorLaunchRequestFromWorkspace(workspace(), 'C:\\dev\\jasonshell\\src\\main.ts', 12);

  assert.deepEqual(terminal, {
    workspace: { id: 'jasonshell', name: 'JasonShell', rootPath: 'C:\\dev\\jasonshell' },
    template: { executable: 'wt.exe', args: ['-d', '{workspacePath}'], cwd: '{workspacePath}' }
  });
  assert.deepEqual(editor, {
    workspace: { id: 'jasonshell', name: 'JasonShell', rootPath: 'C:\\dev\\jasonshell' },
    filePath: 'C:\\dev\\jasonshell\\src\\main.ts',
    fileLine: 12,
    template: { executable: 'code', args: ['--goto', '{filePath}:{fileLine}'], cwd: '{workspacePath}' }
  });
});

test('task requests and history rankings preserve workspace process metadata references', () => {
  const profile = workspace();
  const request = workspaceTaskRequest(profile, profile.tasks[0]);
  const ranked = rankTaskHistoryForWorkspace(
    [
      {
        taskId: 'old',
        workspaceId: 'jasonshell',
        workspacePath: 'C:\\dev\\jasonshell',
        label: 'Validate',
        executable: 'npm',
        args: ['run', 'validate'],
        processId: 1,
        startedAtEpochMs: 1,
        finishedAtEpochMs: 2,
        exitCode: 0,
        canceled: false
      },
      {
        taskId: 'new',
        workspaceId: 'jasonshell',
        workspacePath: 'C:\\dev\\jasonshell',
        label: 'Validate',
        executable: 'npm',
        args: ['run', 'validate'],
        processId: 2,
        startedAtEpochMs: 10,
        finishedAtEpochMs: 11,
        exitCode: 0,
        canceled: false
      },
      {
        taskId: 'other',
        workspaceId: 'other',
        workspacePath: 'C:\\dev\\other',
        label: 'Other',
        executable: 'npm',
        args: ['test'],
        processId: 3,
        startedAtEpochMs: 99,
        finishedAtEpochMs: 100,
        exitCode: 0,
        canceled: false
      }
    ],
    'jasonshell'
  );

  assert.deepEqual(request, {
    workspaceId: 'jasonshell',
    taskId: 'validate'
  });
  assert.deepEqual(ranked.map((entry) => entry.taskId), ['new', 'old']);
});

test('task request helper refuses undeclared task objects instead of minting argv payloads', () => {
  const profile = workspace();

  assert.throws(
    () => workspaceTaskRequest(profile, { ...profile.tasks[0], id: 'whoami', command: 'cmd.exe', args: ['/C', 'whoami'] }),
    /Workspace task is not declared/
  );
});

test('Rust tooling foundation rejects command-line execution seams and registers capabilities', () => {
  assert.match(rustToolPlansSource, /uses_shell: false/);
  assert.match(rustToolPlansSource, /shell metacharacters/);
  assert.match(rustToolPlansSource, /parent-directory traversal/);
  assert.match(rustTaskSource, /TASK_OUTPUT_EVENT/);
  assert.match(rustTaskSource, /MAX_TASK_HISTORY_ENTRIES: usize = 50/);
  assert.match(rustTaskSource, /deny_unknown_fields/);
  assert.match(rustTaskSource, /list_jasonshell_task_process_metadata/);
  assert.match(capabilitySource, /workspace tooling action plans/);
  assert.match(capabilitySource, /developer tooling events/);
  assert.match(capabilitySource, /editor\/terminal file action plans/);
  assert.match(capabilitySource, /task metadata references/);
});
