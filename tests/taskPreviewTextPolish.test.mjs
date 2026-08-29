import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const previewSource = readFileSync(new URL('../src/components/TaskPreviewSurface.svelte', import.meta.url), 'utf8');
const previewCss = readFileSync(new URL('../src/components/TaskPreviewSurface.css', import.meta.url), 'utf8');

function cssRule(source, selector) {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = source.match(new RegExp(`${escapedSelector}\\s*\\{([\\s\\S]*?)\\n\\}`));
  assert.ok(match, `${selector} rule exists`);
  return match[1];
}

test('preview header separates primary title from secondary process text', () => {
  assert.match(previewSource, /previewPrimaryTitle = preview \? \(preview\.title \|\| preview\.processName\) : ''/);
  assert.match(previewSource, /previewSecondaryText = preview && preview\.processName !== previewPrimaryTitle \? preview\.processName : ''/);
  assert.match(previewSource, /class="preview-title"[\s\S]*\{previewPrimaryTitle\}/);
  assert.match(previewSource, /\{#if previewSecondaryText\}[\s\S]*class="preview-process"[\s\S]*\{previewSecondaryText\}/);
  assert.doesNotMatch(previewSource, /<span><div class="preview-title"/);
});

test('preview close button stays out of text flow with reserved header space', () => {
  const headerRule = cssRule(previewCss, '.preview-header');
  const closeButtonRule = cssRule(previewCss, '.preview-close-button');
  assert.match(previewSource, /class="preview-header"/);
  const reservedSpace = Number(headerRule.match(/padding-right:\s*([\d.]+)rem/)?.[1]);
  assert.ok(reservedSpace >= 2.1, 'preview header reserves at least the close button footprint');
  assert.match(previewCss, /\.preview-copy\s*\{[\s\S]*min-width:\s*0/);
  assert.match(closeButtonRule, /position:\s*absolute/);
  assert.match(closeButtonRule, /z-index:\s*4/);
  assert.match(closeButtonRule, /height:\s*1\.35rem/);
  assert.match(closeButtonRule, /(?:min-width:\s*2\.1rem|padding:\s*0\s+0\.42rem)/);
  assert.doesNotMatch(closeButtonRule, /width:\s*1\.35rem/);
});

test('preview text truncates long title and process labels without stealing frame space', () => {
  assert.match(previewCss, /\.preview-title\s*\{[\s\S]*overflow:\s*hidden/);
  assert.match(previewCss, /\.preview-title\s*\{[\s\S]*text-overflow:\s*ellipsis/);
  assert.match(previewCss, /\.preview-title\s*\{[\s\S]*white-space:\s*nowrap/);
  assert.match(previewCss, /\.preview-process\s*\{[\s\S]*overflow:\s*hidden/);
  assert.match(previewCss, /\.preview-process\s*\{[\s\S]*text-overflow:\s*ellipsis/);
  assert.match(previewCss, /\.preview-header\s*\{[\s\S]*flex:\s*0 0 auto/);
  assert.match(previewCss, /\.preview-frame,\s*\.preview-empty\s*\{[\s\S]*flex:\s*1 1 auto/);
});

test('native and captured preview frames keep unobstructed dominant layout', () => {
  assert.match(
    previewSource,
    /\{#if isNativeLivePreview\}[\s\S]*class="preview-frame preview-frame-native"[\s\S]*\{:else if preview\.imageDataUrl\}/
  );
  assert.match(previewCss, /\.preview-surface-native\s*\{[\s\S]*background:\s*transparent/);
  assert.match(previewCss, /\.preview-surface-native \.preview-frame-native\s*\{[\s\S]*border-color:\s*transparent/);
  assert.match(previewCss, /\.preview-image\s*\{[\s\S]*object-fit:\s*contain/);
  assert.match(previewCss, /\.preview-frame,\s*\.preview-empty\s*\{[\s\S]*min-height:\s*0/);
});
