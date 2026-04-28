import assert from 'node:assert/strict';
import { test } from 'node:test';

import { stackBrowserContextActionPlans } from '../dist-tests/lib/stackPopupViewModel.js';

function entry(path, name = 'server.ts') {
  return {
    id: path,
    name,
    path,
    entryType: 'File',
    typeLabel: 'TS File',
    size: 1,
    modifiedMs: null,
    isHidden: false,
    isReadonly: false,
    isSystem: false,
    isSymlink: false,
    isReparsePoint: false
  };
}

test('plans editor, terminal, and copy actions without executing them', () => {
  const plans = stackBrowserContextActionPlans({
    currentFolderPath: 'C:\\dev\\jasonshell\\src',
    entry: entry('C:\\dev\\jasonshell\\src\\server.ts')
  });

  assert.deepEqual(
    plans.slice(0, 5).map((plan) => [plan.id, plan.destructive, plan.requiresConfirmation]),
    [
      ['open-editor', false, false],
      ['open-terminal', false, false],
      ['copy-path', false, false],
      ['copy-directory-path', false, false],
      ['copy-name', false, false]
    ]
  );
  assert.equal(plans.find((plan) => plan.id === 'open-terminal').workingDirectory, 'C:\\dev\\jasonshell\\src');
  assert.equal(plans.find((plan) => plan.id === 'copy-path').clipboardText, 'C:\\dev\\jasonshell\\src\\server.ts');
});

test('plans template creation paths under the current folder target', () => {
  const plans = stackBrowserContextActionPlans({
    currentFolderPath: 'C:\\dev\\jasonshell\\src',
    templates: [{ id: 'component', label: 'New component', fileName: 'Component.svelte', kind: 'file' }]
  });

  const templatePlan = plans.find((plan) => plan.id === 'template:component');
  assert.equal(templatePlan.kind, 'create-from-template');
  assert.equal(templatePlan.plannedPath, 'C:\\dev\\jasonshell\\src\\Component.svelte');
  assert.equal(templatePlan.destructive, false);
});

test('plans git-aware operations and marks restore as confirmation-gated', () => {
  const target = 'C:\\dev\\jasonshell\\src\\server.ts';
  const plans = stackBrowserContextActionPlans({
    currentFolderPath: 'C:\\dev\\jasonshell\\src',
    entry: entry(target),
    git: {
      repositoryRoot: 'C:\\dev\\jasonshell',
      changedPaths: [target]
    }
  });

  assert.deepEqual(
    plans.filter((plan) => plan.kind === 'git-operation').map((plan) => [
      plan.gitOperation,
      plan.destructive,
      plan.requiresConfirmation
    ]),
    [
      ['diff', false, false],
      ['stage', false, false],
      ['restore', true, true]
    ]
  );
});

test('omits git operations outside the repository root', () => {
  const plans = stackBrowserContextActionPlans({
    currentFolderPath: 'D:\\outside',
    entry: entry('D:\\outside\\server.ts'),
    git: {
      repositoryRoot: 'C:\\dev\\jasonshell',
      changedPaths: ['D:\\outside\\server.ts']
    }
  });

  assert.equal(plans.some((plan) => plan.kind === 'git-operation'), false);
});
