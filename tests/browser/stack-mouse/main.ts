import '../../../src/app.css';
import { mount, unmount, tick } from 'svelte';
import StackPopupSurface from '../../../src/components/StackPopupSurface.svelte';
import { child, fixtureRowHeight, installLargeDirectoryFixture, largeDirectoryCount, largeFileName, ledger, resetBridge } from './bridge';

let popup: ReturnType<typeof mount> | null = null;
const events: { type: string; button: number; phase: string; prevented: boolean; branch: boolean; diff: boolean; confirmation: boolean }[] = [];
const viewports: { phase: string; scrollTop: number; height: number; renderedRows: number; selectedPath: string; rowVisible: boolean }[] = [];
const element = (selector: string) => document.querySelector<HTMLElement>(selector);
const folderRow = (entryPath: string) => [...document.querySelectorAll<HTMLElement>('[data-stack-entry-path]')].find((row) => row.getAttribute('data-stack-entry-path') === entryPath);
const exists = (selector: string) => Boolean(element(selector));
const path = () => (element('input[aria-label="Current folder path"]') as HTMLInputElement)?.value;
const folderReads = () => ledger.filter(({ command }) => command === 'read_stack_folder').length;
function assert(condition: unknown, message: string): asserts condition { if (!condition) throw new Error(message); }
async function until(predicate: () => unknown, message: string) {
  const deadline = performance.now() + 3000;
  while (!predicate()) { if (performance.now() > deadline) throw new Error(message); await new Promise((resolve) => setTimeout(resolve, 10)); }
}
function state(event: MouseEvent, phase: string) {
  return { type: event.type, button: event.button, phase, prevented: event.defaultPrevented,
    branch: exists('[aria-label="Branch picker"]'), diff: exists('[aria-label="Diff for draft.txt"]'), confirmation: exists('[role="alertdialog"]') };
}
for (const type of ['pointerdown', 'mousedown', 'auxclick']) {
  window.addEventListener(type, (event) => { if ((event as MouseEvent).button >= 3) events.push(state(event as MouseEvent, 'capture-observer-before-owner')); }, true);
  window.addEventListener(type, (event) => { if ((event as MouseEvent).button >= 3) events.push(state(event as MouseEvent, 'bubble-observer')); });
}

async function reset(largeDirectory = false) {
  if (popup) await unmount(popup);
  resetBridge(); events.length = 0; viewports.length = 0;
  if (largeDirectory) installLargeDirectoryFixture();
  popup = mount(StackPopupSurface, { target: element('#surface')! });
  await until(() => folderRow(child), 'Fixture folder not mounted');
}
async function openRepo(largeDirectory = false) {
  await reset(largeDirectory);
  folderRow(child)!.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
  await until(() => path() === child && (largeDirectory ? folderRow(`${child}\\${largeFileName(0)}`) : exists('[data-stack-entry-path$="draft.txt"]')), 'Repo not opened');
}
async function sidePress(button = 3, target = element('.stack-popup')!) {
  const event = new PointerEvent('pointerdown', { pointerType: 'mouse', pointerId: 1, button, buttons: button === 3 ? 8 : 16, bubbles: true, cancelable: true });
  target.dispatchEvent(event);
  events.push(state(event, 'dispatch-return'));
  assert(event.defaultPrevented, `pointerdown button ${button} not owned`);
  await tick();
  // Use mounted underlying surface after a layer is removed to detect double-unwind.
  for (const type of ['mousedown', 'pointerup', 'mouseup', 'auxclick']) {
    const init = { button, buttons: type === 'mousedown' ? (button === 3 ? 8 : 16) : 0, bubbles: true, cancelable: true };
    const compatibility = type === 'pointerup' ? new PointerEvent(type, { ...init, pointerType: 'mouse', pointerId: 1 }) : new MouseEvent(type, init);
    element('.stack-popup')!.dispatchEvent(compatibility);
    events.push(state(compatibility, 'dispatch-return'));
    await tick();
  }
}
function unchanged(reads: number) {
  assert(path() === child, 'Layer navigation changed folder path');
  assert(folderReads() === reads, 'Layer navigation read another folder');
  assert(!ledger.some(({ command }) => /hide|delete|save|revert|checkout|create/.test(command)), 'Unexpected native mutation');
}
async function gitJourney() {
  await openRepo();
  const reads = folderReads();
  element('.stack-git-branch')!.click();
  await until(() => exists('[aria-label="Git panel"]'), 'Git panel missing');
  element('.stack-git-change-group-file[role="button"]')!.click();
  await until(() => exists('[aria-label="Diff for draft.txt"]'), 'Changes diff missing');
  element('[aria-label="Branch main"]')!.click();
  await until(() => exists('[aria-label="Delete local branch topic"]'), 'Branches missing');
  element('[aria-label="Delete local branch topic"]')!.click();
  await until(() => exists('[role="alertdialog"]'), 'Confirmation missing');
  await sidePress(4, element('[role="alertdialog"]')!);
  assert(exists('[role="alertdialog"]'), 'Forward dismissed confirmation');
  await sidePress(3, element('[role="alertdialog"]')!);
  assert(!exists('[role="alertdialog"]') && exists('[aria-label="Branch picker"]'), 'Back did not cancel only confirmation');
  unchanged(reads);
  await sidePress(3, element('[aria-label="Git panel"]')!);
  assert(!exists('[aria-label="Branch picker"]') && exists('[aria-label="Diff for draft.txt"]'), 'Branch picker press closed underlying diff');
  await sidePress();
  assert(!exists('[aria-label="Diff for draft.txt"]') && exists('[aria-label="Git panel"]'), 'Diff press closed panel');
  await sidePress();
  assert(!exists('[aria-label="Git panel"]') && exists('[aria-label="Folder details"]'), 'Panel press did not return to same folder');
  unchanged(reads);
}
async function editorJourney() {
  await openRepo();
  const reads = folderReads();
  element('[data-stack-entry-path$="draft.txt"]')!.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
  await until(() => exists('.cm-content'), 'Actual CodeMirror editor missing');
  const content = element('.cm-content')!;
  content.focus();
  const selection = window.getSelection()!;
  const range = document.createRange(); range.selectNodeContents(content); selection.removeAllRanges(); selection.addRange(range);
  assert(document.execCommand('insertText', false, 'unsaved browser draft'), 'Browser text insertion unsupported');
  await until(() => element('.stack-text-editor-draft-state')?.textContent?.includes('Draft changed'), 'Draft did not become dirty');
  await sidePress(3, content);
  assert(exists('[role="alertdialog"]'), 'Dirty Back failed to prompt');
  await sidePress(3, element('[role="alertdialog"]')!);
  assert(!exists('[role="alertdialog"]') && element('.cm-content')?.textContent === 'unsaved browser draft', 'Cancel lost draft');
  await sidePress(3, element('.cm-content')!);
  [...document.querySelectorAll<HTMLButtonElement>('[role="alertdialog"] button')].find((button) => button.textContent === 'Discard')!.click();
  await until(() => exists('[aria-label="Folder details"]'), 'Discard failed to close editor');
  unchanged(reads);
}
async function layoutSettled() {
  await tick();
  const frame = () => new Promise<void>((resolve, reject) => {
    const deadline = window.setTimeout(() => {
      cancelAnimationFrame(id);
      reject(new Error(`Visible browser animation frame did not arrive within 3s (visibility=${document.visibilityState})`));
    }, 3000);
    const id = requestAnimationFrame(() => { window.clearTimeout(deadline); resolve(); });
  });
  await frame();
  await frame();
  await tick();
}
function recordViewport(phase: string, selectedPath: string) {
  const body = element('.details-body')!;
  const row = folderRow(selectedPath);
  const bounds = body.getBoundingClientRect();
  const rowBounds = row?.getBoundingClientRect();
  const rowVisible = Boolean(rowBounds && rowBounds.top >= bounds.top - 1 && rowBounds.bottom <= bounds.bottom + 1);
  const sample = { phase, scrollTop: body.scrollTop, height: bounds.height, renderedRows: document.querySelectorAll('[data-stack-entry-path]').length, selectedPath, rowVisible };
  viewports.push(sample);
  return sample;
}
async function scrollLargeFolder(index: number) {
  const body = element('.details-body')!;
  assert(body.clientHeight > 0 && body.clientHeight < element('#surface')!.clientHeight, 'Folder viewport must be finite inside the popup host');
  assert(body.scrollHeight > body.clientHeight, 'Large fixture has no real scrollable layout');
  body.scrollTop = index * fixtureRowHeight;
  body.dispatchEvent(new Event('scroll', { bubbles: true }));
  const selectedPath = `${child}\\${largeFileName(index)}`;
  await until(() => folderRow(selectedPath), 'Bottom virtualized file row missing');
  assert(Math.abs(folderRow(selectedPath)!.getBoundingClientRect().height - fixtureRowHeight) <= 1, 'Actual folder row height disagrees with production virtualization contract');
  folderRow(selectedPath)!.click();
  await layoutSettled();
  const sample = recordViewport('before-open', selectedPath);
  assert(sample.rowVisible, 'Selected file is not actually visible inside scrolled folder');
  assert(sample.renderedRows < largeDirectoryCount, 'Large fixture did not exercise virtualization');
  assert(sample.scrollTop > 0, 'Large fixture did not scroll');
  return sample;
}
async function checkRestored(sample: ReturnType<typeof recordViewport>, reads: number) {
  await until(() => exists('[aria-label="Folder details"]'), 'Folder grid not remounted');
  await layoutSettled();
  const restored = recordViewport('after-return-focus', sample.selectedPath);
  assert(Math.abs(restored.scrollTop - sample.scrollTop) <= 1, `Folder offset lost: expected ${sample.scrollTop}, got ${restored.scrollTop}`);
  assert(restored.rowVisible, 'Original file is not visible after virtualized remount');
  assert(folderRow(sample.selectedPath)?.getAttribute('aria-selected') === 'true', 'Folder selection lost');
  assert(document.activeElement === element('[aria-label="Folder details"]'), 'Folder focus was not restored');
  unchanged(reads);
}
function modeButton(label: string) { return [...document.querySelectorAll<HTMLButtonElement>('[aria-label="Markdown view"] button')].find((button) => button.textContent === label)!; }
async function viewportJourney() {
  await openRepo(true); await layoutSettled();
  const reads = folderReads();
  const bottom = await scrollLargeFolder(350);
  folderRow(bottom.selectedPath)!.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
  await until(() => Boolean(modeButton('Preview')), 'Markdown Preview not mounted');
  element('.stack-text-editor-body')!.scrollTop = 137;
  await sidePress(3, element('[aria-label="Close editor and return to folder"]')!);
  await checkRestored(bottom, reads);

  folderRow(bottom.selectedPath)!.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
  await until(() => Boolean(modeButton('Edit')), 'Markdown mode controls missing');
  modeButton('Edit').click();
  await until(() => exists('.cm-content'), 'Actual CodeMirror editor missing');
  const content = element('.cm-content')!; content.focus();
  const range = document.createRange(); range.selectNodeContents(content);
  const selection = window.getSelection()!; selection.removeAllRanges(); selection.addRange(range);
  assert(document.execCommand('insertText', false, 'unsaved viewport draft'), 'Browser text insertion unsupported');
  await until(() => element('.stack-text-editor-draft-state')?.textContent?.includes('Draft changed'), 'Viewport draft did not become dirty');
  await sidePress(3, content);
  await until(() => exists('[role="alertdialog"]'), 'Dirty viewport Back failed to prompt');
  [...document.querySelectorAll<HTMLButtonElement>('[role="alertdialog"] button')].find((button) => button.textContent === 'Cancel')!.click();
  await layoutSettled();
  assert(!exists('[aria-label="Folder details"]') && element('.cm-content')?.textContent === 'unsaved viewport draft', 'Cancel lost editor/draft');
  await sidePress(3, element('.cm-content')!);
  [...document.querySelectorAll<HTMLButtonElement>('[role="alertdialog"] button')].find((button) => button.textContent === 'Discard')!.click();
  await checkRestored(bottom, reads);

  const latest = await scrollLargeFolder(100);
  folderRow(latest.selectedPath)!.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
  await until(() => Boolean(modeButton('Preview')), 'Repeated Markdown open missing');
  await sidePress(3, element('[aria-label="Close editor and return to folder"]')!);
  await checkRestored(latest, reads);
}
async function run(journey: 'git' | 'editor' | 'viewport') {
  // Make click delivery observable before the first reset, bridge await, or animation frame.
  element('#result')!.textContent = `${journey}: Running`;
  try { await (journey === 'git' ? gitJourney() : journey === 'editor' ? editorJourney() : viewportJourney()); element('#result')!.textContent = `${journey}: PASS\n${JSON.stringify({ events, viewports, ledger }, null, 2)}`; return { passed: true, events, viewports, ledger }; }
  catch (error) { element('#result')!.textContent = `${journey}: FAIL ${String(error)}\n${JSON.stringify({ events, viewports, ledger }, null, 2)}`; return { passed: false, error: String(error), events, viewports, ledger }; }
}
Object.assign(window, { stackMouse: { reset, run, events, viewports, ledger } });
element('#reset')!.onclick = () => { void reset(); };
element('#git')!.onclick = () => { void run('git'); };
element('#editor')!.onclick = () => { void run('editor'); };
element('#viewport')!.onclick = () => { void run('viewport'); };
void reset();
