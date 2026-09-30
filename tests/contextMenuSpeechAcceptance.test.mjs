import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import test from 'node:test';

const source = (path) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');
const menuPath = 'src/components/ContextMenu.svelte';

test('shared app-owned menu provides rounded glass motion, icons, and disabled/destructive items', () => {
  assert.ok(existsSync(new URL(`../${menuPath}`, import.meta.url)), 'shared ContextMenu component required');
  const menu = source(menuPath);
  assert.match(menu, /MaterialSymbolIcon/);
  assert.match(menu, /(?:icon|placeholder)/i);
  assert.match(menu, /disabled/);
  assert.match(menu, /destructive/);
  assert.match(menu, /separator/i);
  assert.match(menu, /150\s*ms|duration\s*:\s*150/);
  assert.match(menu, /(?:scale|transform)/);
  assert.match(menu, /(?:opacity|fade)/);
  assert.match(menu, /(?:backdrop-filter|backdropFilter)/);
  assert.match(menu, /border-radius|rounded/);
});

test('app-controlled Stack, terminal, command, and Git menus share menu presentation', () => {
  for (const path of [
    'src/components/StackPopupSurface.svelte',
    'src/components/TerminalPanelSurface.svelte',
    'src/components/StackTerminalPane.svelte',
    'src/components/CommandPanelSurface.svelte',
    'src/components/StackGitPanel.svelte'
  ]) {
    assert.ok(/ContextMenu/.test(source(path)), `${path} should consume the shared menu`);
  }
});

test('taskbar and pin right-click menus are app-owned rather than native popup dispatch', () => {
  for (const path of [
    'src/components/BottomBar.svelte',
    'src/components/TopBar.svelte',
    'src/components/QuickLaunchPanelSurface.svelte',
    'src/components/TaskGallerySurface.svelte'
  ]) {
    const component = source(path);
    assert.ok(/ContextMenu/.test(component), `${path} should render an app-owned menu`);
    assert.ok(!/\b(?:showLauncherContextMenu|showTaskWindowContextMenu|showQuickLaunchPanelContextMenu|showTaskGalleryContextMenu)\s*\(/.test(component), `${path} should not dispatch native popup menus`);
  }
});

test('speech capture retains original typing target and verifies focus before injecting paste', () => {
  const runtime = source('src-tauri/src/speech_runtime.rs');
  assert.ok(/(?:capture|snapshot|record)[\s\S]{0,100}(?:target|foreground|focus)/i.test(runtime), 'capture intended target at start');
  assert.ok(/(?:target|foreground|focus)[\s\S]{0,120}(?:verify|validat|match)/i.test(runtime), 'verify original target before paste');
  assert.ok(/(?:paste|send_input|SendInput)/i.test(runtime), 'inject paste to verified target');
  assert.match(runtime, /write_unicode_text\(text\)/);
});

test('speech history copy remains clipboard-only, never paste or injected keystrokes', () => {
  const runtime = source('src-tauri/src/speech_runtime.rs');
  const history = runtime.match(/pub\(crate\) fn copy_speech_history_transcript\([\s\S]*?\n\}/)?.[0];
  assert.ok(history, 'history copy command present');
  assert.match(history, /write_unicode_text\(transcript\)/);
  assert.doesNotMatch(history, /paste|send_input|SendInput/i);
});
