import '../../../src/app.css';
import { mount, unmount, tick } from 'svelte';
import StackPopupSurface from '../../../src/components/StackPopupSurface.svelte';
import { child, deep, nested, context, summary, deferred, ledger, overrides, resetBridge } from './bridge';
let popup: ReturnType<typeof mount> | undefined;
const checks: string[] = [];
const element = (selector: string) => document.querySelector<HTMLElement>(selector);
const count = (command: string) => ledger.filter((entry) => entry.command === command).length;
function assert(value: unknown, label: string) { if (!value) throw new Error(label); checks.push(label); }
async function until(predicate: () => unknown, label: string) {
  const deadline = performance.now() + 4000;
  while (!predicate()) { if (performance.now() > deadline) throw new Error(label); await new Promise((resolve) => setTimeout(resolve, 10)); }
}
async function navigate(path: string) {
  const input = element('input[aria-label="Current folder path"]') as HTMLInputElement;
  input.value = path; input.dispatchEvent(new Event('input', { bubbles: true }));
  input.closest('form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
  await until(() => input.value === path && element(`[data-stack-entry-path="${CSS.escape(path + '\\draft.txt')}"]`), `Navigation failed: ${path}`);
}
async function run() {
  if (popup) await unmount(popup); resetBridge(); checks.length = 0;
  const metadata = deferred<unknown>(); overrides.set('get_stack_git_metadata', () => metadata.promise);
  popup = mount(StackPopupSurface, { target: element('#surface')! });
  await until(() => element('.stack-git-branch')?.textContent === 'main', 'Initial branch missing');
  assert(count('get_stack_git_summary') === 1, 'Initial lightweight summary call count 1');
  assert(count('get_stack_git_status') === 0, 'No full workbench status during initial browse');
  await until(() => count('get_stack_git_metadata') === 1, 'Deferred metadata never started');
  assert(ledger.findIndex((entry) => entry.command === 'get_stack_git_summary') < ledger.findIndex((entry) => entry.command === 'get_stack_git_metadata'), 'Summary IPC precedes deferred metadata IPC');
  const branch = element('.stack-git-branch'); const delayed = deferred<unknown>();
  overrides.set('get_stack_git_context', () => delayed.promise);
  element(`[data-stack-entry-path="${CSS.escape(deep)}"]`)!.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
  const childRow = () => element(`[data-stack-entry-path="${CSS.escape(deep + '\\draft.txt')}"]`);
  await until(() => Boolean(childRow()), 'Committed child listing missing');
  assert(element('.stack-git-branch') === branch, 'Delayed same-repo context retained branch DOM');
  assert(childRow()!.getAttribute('data-git-status') === null, 'Unverified child row has no inherited badge');
  delayed.resolve(context(deep));
  await until(() => childRow()?.getAttribute('data-git-status') === 'modified', 'Verified child badge missing');
  assert(count('get_stack_git_summary') === 1, 'Same-repo child navigation summary count remains 1');
  overrides.set('get_stack_git_context', ({ path }) => context(String(path)));
  for (const name of ['Back', 'Forward']) {
    element(`[aria-label="${name}"]`)!.click();
    await until(() => (element('input[aria-label="Current folder path"]') as HTMLInputElement).value === (name === 'Back' ? child : deep), `${name} failed`);
    assert(element('.stack-git-branch') === branch, `${name} retained original branch DOM node`);
  }
  assert(count('get_stack_git_summary') === 1, 'Back/Forward issued zero extra summaries');
  assert(count('get_stack_git_metadata') === 1, 'Same-repo navigation issued zero extra metadata');
  const refreshContext = deferred<unknown>(); overrides.set('get_stack_git_context', () => refreshContext.promise);
  overrides.set('get_stack_git_summary', () => Promise.reject(new Error('fixture refresh unavailable')));
  element('[aria-label="Refresh"]')!.click(); await until(() => count('get_stack_git_summary') === 2 && document.body.textContent?.includes('fixture refresh unavailable'), 'Refresh did not invalidate/fail summary'); await tick();
  assert(element('.stack-git-branch')?.textContent === 'main', 'Refresh failure retained known snapshot');
  refreshContext.resolve(context(deep)); await tick(); await tick();
  assert(count('get_stack_git_summary') === 2, 'Delayed Refresh context did not implicitly retry failed summary');
  overrides.set('get_stack_git_context', ({ path }) => context(String(path)));
  overrides.set('get_stack_git_summary', ({ path }) => summary(String(path)));
  overrides.set('get_stack_git_metadata', ({ path }) => ({ ...context(String(path)), remoteRepositoryUrl: null }));
  childRow()!.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
  await until(() => Boolean(element('.cm-content')), 'Production CodeMirror editor missing');
  const content = element('.cm-content')!; content.focus();
  const selection = window.getSelection()!; const range = document.createRange(); range.selectNodeContents(content); selection.removeAllRanges(); selection.addRange(range);
  assert(document.execCommand('insertText', false, 'saved browser refresh draft'), 'Actual CodeMirror accepts browser draft insertion');
  await until(() => element('.stack-text-editor-draft-state')?.textContent?.includes('Draft changed'), 'Editor draft did not become dirty');
  element('[aria-label="Save file"]')!.click();
  await until(() => count('save_stack_basic_text_file') === 1 && count('get_stack_git_summary') === 3, 'Successful editor save did not invalidate summary');
  assert(count('save_stack_basic_text_file') === 1, 'Successful editor save invalidated exactly once without navigation');
  await until(() => element('.stack-text-editor-draft-state')?.textContent?.includes('Saved'), 'Editor save did not settle');
  element('[aria-label="Close editor and return to folder"]')!.click();
  await until(() => Boolean(childRow()), 'Editor close did not restore current folder');
  await navigate(nested); await until(() => element('.stack-git-branch')?.textContent === 'other', 'Nested repository branch missing');
  metadata.resolve({ ...context(child), remoteRepositoryUrl: 'https://stale.invalid', ahead: 99 }); await tick();
  assert(element('.stack-git-branch')?.textContent === 'other', 'Stale metadata did not replace nested snapshot');
  assert(!element('.stack-git-remote-link'), 'Stale old-repository metadata did not attach remote link to new repository');
}
function publishEvidence() { Object.assign(window, { stackGitRefreshEvidence: { checks: checks.slice(), ledger: ledger.slice() } }); }
element('#run')!.onclick = () => { element('#result')!.textContent = 'Running'; void run().then(() => { element('#result')!.textContent = 'PASS'; publishEvidence(); }).catch((error) => { element('#result')!.textContent = `FAIL: ${String(error)}`; publishEvidence(); }); };
Object.assign(window, { stackGitRefreshEvidence: { checks, ledger } });

async function reviewRegression() {
  if (popup) await unmount(popup); resetBridge(); checks.length = 0;
  popup = mount(StackPopupSurface, { target: element('#surface')! });
  await until(() => element('.stack-git-branch')?.textContent === 'main' && Boolean(element('.stack-git-remote-link')), 'Initial known metadata missing');
  const knownRemote = element('.stack-git-remote-link')!.getAttribute('title');
  const readFolder = overrides.get('read_stack_folder')!;
  overrides.set('read_stack_folder', () => Promise.reject(new Error('review post-paste listing failed')));
  element('[aria-label="Paste into current folder"]')!.click();
  await until(() => count('paste_stack_items') === 1 && count('read_stack_folder') >= 2, 'Native paste/follow-up folder read missing');
  await until(() => count('get_stack_git_summary') === 2, 'Successful paste followed by listing rejection did not invalidate summary');
  assert(count('get_stack_git_summary') === 2, 'Successful paste invalidated despite follow-up listing failure');
  await until(() => document.body.textContent?.includes('review post-paste listing failed'), 'Post-paste listing error diagnostic missing');
  assert(document.body.textContent?.includes('review post-paste listing failed'), 'Post-paste listing error remains visible');
  overrides.set('read_stack_folder', readFolder);
  overrides.set('paste_stack_items', () => ({ pasted: [], failures: [{ path: 'C:\\fixture\\incoming.txt', message: 'review all-failed paste' }] }));
  element('[aria-label="Paste into current folder"]')!.click();
  await until(() => count('paste_stack_items') === 2 && count('read_stack_folder') >= 3, 'All-failed paste/folder refresh missing');
  await tick(); await tick();
  assert(count('get_stack_git_summary') === 2, 'All-failed paste did not invalidate summary');
  await until(() => document.body.textContent?.includes('review all-failed paste'), 'All-failed paste diagnostic missing');
  assert(document.body.textContent?.includes('review all-failed paste'), 'All-failed paste error remains visible');
  // Retire the completed file-operation UI before the independent metadata-error
  // case so its persistent operation status does not mask the Git diagnostic.
  // Keep the same ledger: the remount and its reads remain part of the evidence.
  await unmount(popup!);
  popup = mount(StackPopupSurface, { target: element('#surface')! });
  await until(() => Boolean(element('.stack-git-remote-link')), 'Remounted known metadata missing');
  overrides.set('get_stack_git_metadata', () => Promise.reject(new Error('review metadata probe timeout')));
  element('[aria-label="Refresh"]')!.click();
  await until(() => document.body.textContent?.includes('review metadata probe timeout'), 'Metadata error diagnostic missing');
  assert(element('.stack-git-remote-link')?.getAttribute('title') === knownRemote, 'Metadata probe failure retained known remote link');
  assert(document.body.textContent?.includes('review metadata probe timeout'), 'Metadata probe failure exposes diagnostic');
}
element('#review-run')!.onclick = () => {
  element('#review-result')!.textContent = 'Running';
  const publish = () => Object.assign(window, { stackGitRefreshReviewEvidence: { checks: checks.slice(), ledger: ledger.slice() } });
  void reviewRegression().then(() => { element('#review-result')!.textContent = 'PASS'; publish(); }).catch((error) => { element('#review-result')!.textContent = `FAIL: ${String(error)}`; publish(); });
};
