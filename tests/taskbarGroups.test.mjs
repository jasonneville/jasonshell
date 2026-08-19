import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  buildTaskWindowGroups,
  hasTaskbarGroupDragStarted,
  isTaskWindowActivityIndicatorEligible,
  moveTaskWindowGroup,
  reconcileTaskWindowGroupOrder,
  taskbarGroupDragDelta,
  taskbarGroupDropTargetFromDisplacement,
  taskbarGroupOrderFromDisplacement,
  taskbarGroupDropTargetFromPointer,
  taskbarGroupDropPlacement,
  taskbarGroupReorderOffset,
  taskWindowGroupKey
} from '../dist-tests/lib/taskbarGroups.js';

function taskWindow(overrides) {
  return {
    hwnd: overrides.hwnd,
    title: overrides.title ?? overrides.processName,
    processName: overrides.processName,
    iconDataUrl: overrides.iconDataUrl ?? 'data:image/png;base64,icon',
    isActive: overrides.isActive ?? false,
    isMinimized: overrides.isMinimized ?? false,
    activityState: overrides.activityState ?? 'idle',
    attentionState: overrides.attentionState ?? 'idle',
    toastCount: overrides.toastCount ?? 0
  };
}

test('groups open task windows by application identity', () => {
  const groups = buildTaskWindowGroups([
    taskWindow({ hwnd: '10', processName: 'firefox', title: 'Docs' }),
    taskWindow({ hwnd: '11', processName: 'Code', title: 'Editor' }),
    taskWindow({ hwnd: '12', processName: 'Firefox', title: 'Mail', isActive: true })
  ]);

  assert.deepEqual(groups.map((group) => group.key), ['firefox', 'code']);
  assert.equal(groups[0].windows.length, 2);
  assert.equal(groups[0].isActive, true);
  assert.equal(groups[0].hasAttention, false);
  assert.equal(groups[0].toastCount, 0);
});

test('keeps group toast count at the highest window count', () => {
  const groups = buildTaskWindowGroups([
    taskWindow({ hwnd: '10', processName: 'Code', title: 'Editor', toastCount: 2 }),
    taskWindow({ hwnd: '11', processName: 'code', title: 'Preview', toastCount: 7 }),
    taskWindow({ hwnd: '12', processName: 'code', title: 'Terminal', toastCount: 3 })
  ]);

  assert.equal(groups[0].toastCount, 7);
});

test('marks a task window group attentive when any child requests attention', () => {
  const groups = buildTaskWindowGroups([
    taskWindow({ hwnd: '10', processName: 'Code', title: 'Editor', attentionState: 'idle' }),
    taskWindow({ hwnd: '11', processName: 'code', title: 'Preview', attentionState: 'requested' }),
    taskWindow({ hwnd: '12', processName: 'code', title: 'Terminal', attentionState: 'idle' })
  ]);

  assert.equal(groups[0].hasAttention, true);
});

test('marks a task window group busy when any eligible contained window is busy', () => {
  const groups = buildTaskWindowGroups([
    taskWindow({ hwnd: '10', processName: 'WindowsTerminal', title: 'Shell' }),
    taskWindow({ hwnd: '11', processName: 'WindowsTerminal', title: 'OpenCode', activityState: 'busy' })
  ]);

  assert.equal(groups[0].isBusy, true);
});

test('suppresses generic busy task window group indicators', () => {
  const groups = buildTaskWindowGroups([
    taskWindow({ hwnd: '10', processName: 'notepad', title: 'Notes', activityState: 'busy' })
  ]);

  assert.equal(groups[0].isBusy, false);
});

test('suppresses generic windows with llm text in the title', () => {
  const groups = buildTaskWindowGroups([
    taskWindow({ hwnd: '10', processName: 'notepad', title: 'OpenCode planning notes', activityState: 'busy' }),
    taskWindow({ hwnd: '11', processName: 'notepad', title: 'Claude prompt notes', activityState: 'busy' })
  ]);

  assert.deepEqual(groups.map((group) => group.isBusy), [false]);
  assert.equal(
    isTaskWindowActivityIndicatorEligible(
      taskWindow({ hwnd: '12', processName: 'notepad', title: 'OpenCode planning notes' })
    ),
    false
  );
  assert.equal(
    isTaskWindowActivityIndicatorEligible(
      taskWindow({ hwnd: '13', processName: 'notepad', title: 'Claude prompt notes' })
    ),
    false
  );
});

test('allows terminal and llm busy task window group indicators', () => {
  const groups = buildTaskWindowGroups([
    taskWindow({ hwnd: '10', processName: 'pwsh', title: 'cargo test', activityState: 'busy' }),
    taskWindow({ hwnd: '11', processName: 'OpenCode', title: 'Generating', activityState: 'busy' }),
    taskWindow({ hwnd: '12', processName: 'WindowsTerminal', title: 'Claude prompt', activityState: 'busy' })
  ]);

  assert.deepEqual(groups.map((group) => group.isBusy), [true, true, true]);
});

test('allows browser busy indicators only with download metadata', () => {
  const groups = buildTaskWindowGroups([
    taskWindow({ hwnd: '10', processName: 'firefox', title: 'Downloads - example.zip', activityState: 'busy' }),
    taskWindow({ hwnd: '11', processName: 'chrome', title: 'Docs', activityState: 'busy' })
  ]);

  assert.deepEqual(groups.map((group) => group.isBusy), [true, false]);
  assert.equal(isTaskWindowActivityIndicatorEligible(taskWindow({ hwnd: '12', processName: '', title: '' })), false);
});

test('requires browser identity from process metadata for download busy indicators', () => {
  assert.equal(
    isTaskWindowActivityIndicatorEligible(
      taskWindow({ hwnd: '10', processName: 'notepad', title: 'Chrome download notes', activityState: 'busy' })
    ),
    false
  );
  assert.equal(
    isTaskWindowActivityIndicatorEligible(
      taskWindow({ hwnd: '11', processName: 'chrome', title: 'Download notes', activityState: 'busy' })
    ),
    true
  );
});

test('reconciles dragged group order as windows appear and disappear', () => {
  assert.deepEqual(
    reconcileTaskWindowGroupOrder(['code', 'firefox', 'terminal'], ['firefox', 'spotify', 'code']),
    ['code', 'firefox', 'spotify']
  );
});

test('moves task window groups without mutating the original order', () => {
  const order = ['firefox', 'code', 'terminal'];
  const next = moveTaskWindowGroup(order, 'terminal', 'firefox');

  assert.deepEqual(next, ['terminal', 'firefox', 'code']);
  assert.deepEqual(order, ['firefox', 'code', 'terminal']);
});

test('moves task window groups after the target for end placement', () => {
  assert.deepEqual(
    moveTaskWindowGroup(['firefox', 'code', 'terminal'], 'firefox', 'terminal', 'after'),
    ['code', 'terminal', 'firefox']
  );
});

test('starts pointer taskbar group drag only after movement threshold', () => {
  assert.equal(hasTaskbarGroupDragStarted(100, 104), false);
  assert.equal(hasTaskbarGroupDragStarted(100, 106), true);
  assert.equal(hasTaskbarGroupDragStarted(100, 93), true);
});

test('keeps below-threshold pointer gestures eligible for click activation', () => {
  assert.equal(hasTaskbarGroupDragStarted(240, 245), false);
});

test('calculates pointer drag delta for visible tile movement', () => {
  assert.equal(taskbarGroupDragDelta(120, 150), 30);
  assert.equal(taskbarGroupDragDelta(120, 100), -20);
});

test('calculates drag reorder compensation from captured rects instead of live transforms', () => {
  const rects = [
    { key: 'firefox', left: 0, width: 90 },
    { key: 'code', left: 90, width: 110 },
    { key: 'terminal', left: 200, width: 100 }
  ];

  assert.equal(taskbarGroupReorderOffset('firefox', ['code', 'terminal', 'firefox'], rects), -210);
  assert.equal(taskbarGroupReorderOffset('terminal', ['terminal', 'firefox', 'code'], rects), 200);
});

test('returns zero drag compensation when the captured source rect is unavailable', () => {
  assert.equal(
    taskbarGroupReorderOffset('missing', ['firefox'], [{ key: 'firefox', left: 0, width: 100 }]),
    0
  );
});

test('chooses reorder placement from pointer position within target group', () => {
  assert.equal(taskbarGroupDropPlacement(139, 100, 80), 'before');
  assert.equal(taskbarGroupDropPlacement(141, 100, 80), 'after');
});

test('chooses drag release target from centerlines including edge positions', () => {
  const rects = [
    { key: 'firefox', left: 0, width: 100 },
    { key: 'code', left: 110, width: 100 },
    { key: 'terminal', left: 220, width: 100 }
  ];

  assert.deepEqual(taskbarGroupDropTargetFromPointer(-20, rects, 'code'), {
    targetKey: 'firefox',
    placement: 'before'
  });
  assert.deepEqual(taskbarGroupDropTargetFromPointer(380, rects, 'code'), {
    targetKey: 'terminal',
    placement: 'after'
  });
  assert.deepEqual(taskbarGroupDropTargetFromPointer(190, rects, 'firefox'), {
    targetKey: 'code',
    placement: 'after'
  });
});

test('chooses rightward drag targets that commit over neighbors and at the end', () => {
  const rects = [
    { key: 'firefox', left: 0, width: 100 },
    { key: 'code', left: 100, width: 100 },
    { key: 'terminal', left: 200, width: 100 }
  ];

  const overRightNeighbor = taskbarGroupDropTargetFromPointer(260, rects, 'code');
  assert.deepEqual(overRightNeighbor, { targetKey: 'terminal', placement: 'after' });
  assert.deepEqual(
    moveTaskWindowGroup(['firefox', 'code', 'terminal'], 'code', overRightNeighbor.targetKey, overRightNeighbor.placement),
    ['firefox', 'terminal', 'code']
  );

  const beyondLast = taskbarGroupDropTargetFromPointer(380, rects, 'firefox');
  assert.deepEqual(beyondLast, { targetKey: 'terminal', placement: 'after' });
  assert.deepEqual(
    moveTaskWindowGroup(['firefox', 'code', 'terminal'], 'firefox', beyondLast.targetKey, beyondLast.placement),
    ['code', 'terminal', 'firefox']
  );
});

test('uses stable centerlines for multi-step rightward drag placement', () => {
  const rects = [
    { key: 'firefox', left: 0, width: 100 },
    { key: 'code', left: 100, width: 100 },
    { key: 'terminal', left: 200, width: 100 },
    { key: 'notes', left: 300, width: 100 }
  ];
  const order = ['firefox', 'code', 'terminal', 'notes'];

  const overCode = taskbarGroupDropTargetFromPointer(151, rects, 'firefox');
  assert.deepEqual(overCode, { targetKey: 'code', placement: 'after' });
  const afterCode = moveTaskWindowGroup(order, 'firefox', overCode.targetKey, overCode.placement);
  assert.deepEqual(afterCode, ['code', 'firefox', 'terminal', 'notes']);

  const overTerminal = taskbarGroupDropTargetFromPointer(251, rects, 'firefox');
  assert.deepEqual(overTerminal, { targetKey: 'terminal', placement: 'after' });
  assert.deepEqual(
    moveTaskWindowGroup(afterCode, 'firefox', overTerminal.targetKey, overTerminal.placement),
    ['code', 'terminal', 'firefox', 'notes']
  );
});

test('commits a rightward release at the far edge to the final position', () => {
  const rects = [
    { key: 'firefox', left: 0, width: 100 },
    { key: 'code', left: 100, width: 100 },
    { key: 'terminal', left: 200, width: 100 }
  ];
  const target = taskbarGroupDropTargetFromPointer(300, rects, 'firefox');

  assert.deepEqual(target, { targetKey: 'terminal', placement: 'after' });
  assert.deepEqual(
    moveTaskWindowGroup(['firefox', 'code', 'terminal'], 'firefox', target.targetKey, target.placement),
    ['code', 'terminal', 'firefox']
  );
});

test('chooses mirrored one-step drag targets from source displacement', () => {
  const rects = [
    { key: 'alpha', left: 0, width: 100 },
    { key: 'bravo', left: 100, width: 100 },
    { key: 'charlie', left: 200, width: 100 }
  ];
  const order = ['alpha', 'bravo', 'charlie'];

  assert.deepEqual(taskbarGroupDropTargetFromDisplacement('bravo', order, rects, -101), {
    targetKey: 'alpha',
    placement: 'before'
  });
  assert.deepEqual(taskbarGroupDropTargetFromDisplacement('bravo', order, rects, 101), {
    targetKey: 'charlie',
    placement: 'after'
  });
});

test('chooses mirrored multi-step drag targets from source displacement', () => {
  const rects = [
    { key: 'alpha', left: 0, width: 100 },
    { key: 'bravo', left: 100, width: 100 },
    { key: 'charlie', left: 200, width: 100 },
    { key: 'delta', left: 300, width: 100 },
    { key: 'echo', left: 400, width: 100 }
  ];
  const order = ['alpha', 'bravo', 'charlie', 'delta', 'echo'];

  assert.deepEqual(taskbarGroupDropTargetFromDisplacement('charlie', order, rects, -201), {
    targetKey: 'alpha',
    placement: 'before'
  });
  assert.deepEqual(taskbarGroupDropTargetFromDisplacement('charlie', order, rects, 201), {
    targetKey: 'echo',
    placement: 'after'
  });
});

test('restores original order when rightward displacement returns to source slot', () => {
  const rects = [
    { key: 'alpha', left: 0, width: 100 },
    { key: 'bravo', left: 100, width: 100 },
    { key: 'charlie', left: 200, width: 100 }
  ];
  const order = ['alpha', 'bravo', 'charlie'];

  assert.deepEqual(taskbarGroupOrderFromDisplacement('alpha', order, rects, 101), [
    'bravo',
    'alpha',
    'charlie'
  ]);
  assert.deepEqual(taskbarGroupOrderFromDisplacement('alpha', order, rects, 0), order);
});

test('keeps a positive visual offset after crossing the first right neighbor boundary', () => {
  const rects = [
    { key: 'alpha', left: 0, width: 100 },
    { key: 'bravo', left: 100, width: 100 },
    { key: 'charlie', left: 200, width: 100 }
  ];
  const previewOrder = taskbarGroupOrderFromDisplacement('alpha', ['alpha', 'bravo', 'charlie'], rects, 101);
  const visualDelta = 101 + taskbarGroupReorderOffset('alpha', previewOrder, rects);

  assert.deepEqual(previewOrder, ['bravo', 'alpha', 'charlie']);
  assert.equal(visualDelta, 1);
});

test('restores original order when leftward displacement returns to source slot', () => {
  const rects = [
    { key: 'alpha', left: 0, width: 100 },
    { key: 'bravo', left: 100, width: 100 },
    { key: 'charlie', left: 200, width: 100 }
  ];
  const order = ['alpha', 'bravo', 'charlie'];

  assert.deepEqual(taskbarGroupOrderFromDisplacement('charlie', order, rects, -101), [
    'alpha',
    'charlie',
    'bravo'
  ]);
  assert.deepEqual(taskbarGroupOrderFromDisplacement('charlie', order, rects, 0), order);
});

test('chooses mirrored edge release targets from source displacement', () => {
  const rects = [
    { key: 'alpha', left: 0, width: 100 },
    { key: 'bravo', left: 100, width: 100 },
    { key: 'charlie', left: 200, width: 100 },
    { key: 'delta', left: 300, width: 100 }
  ];
  const order = ['alpha', 'bravo', 'charlie', 'delta'];

  const leftEdge = taskbarGroupDropTargetFromDisplacement('charlie', order, rects, -500);
  const rightEdge = taskbarGroupDropTargetFromDisplacement('bravo', order, rects, 500);

  assert.deepEqual(leftEdge, { targetKey: 'alpha', placement: 'before' });
  assert.deepEqual(rightEdge, { targetKey: 'delta', placement: 'after' });
  assert.deepEqual(moveTaskWindowGroup(order, 'charlie', leftEdge.targetKey, leftEdge.placement), [
    'charlie',
    'alpha',
    'bravo',
    'delta'
  ]);
  assert.deepEqual(moveTaskWindowGroup(order, 'bravo', rightEdge.targetKey, rightEdge.placement), [
    'alpha',
    'charlie',
    'delta',
    'bravo'
  ]);
});

test('committed drag order survives open-window refresh reconciliation', () => {
  const committedOrder = moveTaskWindowGroup(['firefox', 'code', 'terminal'], 'firefox', 'terminal', 'after');

  assert.deepEqual(
    reconcileTaskWindowGroupOrder(committedOrder, ['terminal', 'firefox', 'code', 'notepad']),
    ['code', 'terminal', 'firefox', 'notepad']
  );
});

test('uses window handle as fallback identity when process is unavailable', () => {
  assert.equal(taskWindowGroupKey(taskWindow({ hwnd: '99', processName: '   ' })), 'window:99');
});
