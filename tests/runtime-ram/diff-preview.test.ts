import { readFileSync } from 'node:fs';
import { transpileModule, ModuleKind } from 'typescript';
import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { beforeEach, expect, it, vi } from 'vitest';
import * as diffState from '../../src/lib/stackGitPanelState';
import { child, ledger, overrides, resetBridge, status } from '../browser/stack-mouse/bridge';
vi.mock('@tauri-apps/api/core', async () => import('../browser/stack-mouse/bridge'));
vi.mock('@tauri-apps/api/event', async () => import('../browser/stack-mouse/bridge'));
vi.mock('@tauri-apps/api/window', async () => import('../browser/stack-mouse/bridge'));
import StackGitPanel from '../../src/components/StackGitPanel.svelte';

const source = readFileSync('src/components/StackGitPanel.svelte', 'utf8');
const body = source.match(/  function getDiffLines\(text: string\) \{[\s\S]*?\n  \}/)?.[0];
const legacyProduction = body ? new Function('maxRenderedDiffLines', `${transpileModule(body, { compilerOptions: { module: ModuleKind.None } }).outputText}; return getDiffLines;`)(4000) as (text: string) => string[] : undefined;
const helper = (diffState as any).getBoundedStackGitDiffLines as undefined | ((text: string, limit: number) => string[]);
const preview = (text: string, limit = 4000) => helper ? helper(text, limit) : legacyProduction!(text);
const oracle = (text: string, limit = 4000) => {
  const lines = text.split(/\r?\n/);
  return lines.length <= limit ? lines : [...lines.slice(0, limit), `Diff truncated after ${limit.toLocaleString()} lines (${lines.length.toLocaleString()} total).`];
};
beforeEach(resetBridge);

it('bounded preview helper exposes reusable actual-production API', () => {
  expect(helper, 'export getBoundedStackGitDiffLines(text: string, limit: number): string[] from src/lib/stackGitPanelState.ts').toBeTypeOf('function');
});

it.each(['', 'one', 'one\n', 'a\r\nb\r\n', 'a\rb', '@@ -1,2 +1,2 @@\r\n-old\r\n+new\r\n same', '+'.repeat(100000)])('preserves exact line and parsed-row semantics for %j', (text) => {
  expect(preview(text)).toEqual(oracle(text));
  expect(diffState.parseStackGitUnifiedDiffRows(preview(text))).toEqual(diffState.parseStackGitUnifiedDiffRows(oracle(text)));
});

it.each([3999, 4000, 4001, 30000])('bounds materialization with exact totals/truncation at %i lines', (count) => {
  const text = Array.from({ length: count }, (_, i) => i === 0 ? '@@ -1,30000 +1,30000 @@' : i % 3 === 0 ? '-old' : i % 3 === 1 ? '+new' : ' same').join('\r\n');
  const expected = oracle(text);
  let materializedSplitLines = 0;
  const split = String.prototype.split;
  const spy = vi.spyOn(String.prototype, 'split').mockImplementation(function (this: string, ...args: any[]) {
    const result = Reflect.apply(split, this, args) as string[];
    if (String(this) === text) materializedSplitLines += result.length;
    return result;
  });
  let actual: string[];
  try { actual = preview(text); } finally { spy.mockRestore(); }
  console.info('ram-git-diff', JSON.stringify({ inputLines: count, returnedLines: actual!.length, materializedSplitLines, inputCharacters: text.length }));
  expect(actual!).toEqual(expected);
  expect(materializedSplitLines).toBeLessThanOrEqual(4001);
});

it('mounted Git viewer uses bounded preview, retaining hunks/classification and exact truncation notice', async () => {
  const text = ['@@ -1,10000 +1,10000 @@', '-old sentinel', '+new sentinel', ...Array.from({ length: 9997 }, () => ' context')].join('\r\n');
  overrides.set('stack_git_diff', () => ({ repositoryRoot: child, content: text }));
  const view = render(StackGitPanel, { folderPath: child, initialStatus: status });
  const split = String.prototype.split;
  let materializedSplitLines = 0;
  const spy = vi.spyOn(String.prototype, 'split').mockImplementation(function (this: string, ...args: any[]) {
    const result = Reflect.apply(split, this, args) as string[];
    if (String(this) === text) materializedSplitLines += result.length;
    return result;
  });
  try {
    const group = screen.getByRole('listitem', { name: 'Unstaged changes' });
    await fireEvent.click(within(group).getByRole('button', { name: /draft.txt/ }));
    const region = await screen.findByRole('region', { name: 'Diff for draft.txt' });
    await within(region).findByText('Diff truncated after 4,000 lines (10,000 total).');
    expect(region.textContent).toContain('old sentinel');
    expect(region.textContent).toContain('new sentinel');
  } finally { spy.mockRestore(); }
  expect.soft(materializedSplitLines).toBeLessThanOrEqual(4001);
  expect(ledger.filter(({ command }) => command === 'stack_git_diff').length).toBeGreaterThanOrEqual(1);
  view.unmount();
});
