<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { basicSetup } from 'codemirror';
  import { EditorState, StateEffect, StateField } from '@codemirror/state';
  import { EditorView, keymap } from '@codemirror/view';
  import { onMount } from 'svelte';
  import {
    P03_PROJECTION_UNITS,
    cancelCompositionOnBlur,
    createImeHarnessState,
    finishComposition,
    noteCommittedInput,
    reconcileAdjacentLease,
    redactEventText,
    startComposition,
    type ImeHarnessState,
  } from '../features/stack-browser/p03NativeHarness';

  const GIANT_COMBINING_MARKS = 20_000;
  const PROJECTION_LIMITS = Object.freeze({ units: P03_PROJECTION_UNITS });
  const IME_LEASE = 'IME lease one ends here: 境';
  const ADJACENT_LEASE = '界 adjacent lease text אבג';
  const GIANT_FIXTURE = `a${'\u0301'.repeat(GIANT_COMBINING_MARKS)} אבג XYZ גבא`;
  const appendLease = StateEffect.define<ImeHarnessState>();
  const leaseProjection = StateField.define<ImeHarnessState>({
    create: () => createImeHarnessState(IME_LEASE),
    update: (value, transaction) => transaction.effects.reduce(
      (next, effect) => effect.is(appendLease) ? effect.value : next,
      value,
    ),
  });
  let host: HTMLDivElement;
  let view: EditorView;
  let fixture: 'ime' | 'giant' = 'ime';
  let adjacentArrived = false;
  let composing = false;
  let inputCommits = 0;
  let harnessState = createImeHarnessState(IME_LEASE);
  let status = 'Ready. Activate a real IME or assistive technology.';

  async function record(kind: string, metadata: Record<string, unknown> = {}) {
    try { await invoke('p03_record', { event: { kind, timeMs: performance.now(), metadata } }); }
    catch (error) { status = `Evidence write failed: ${String(error)}`; }
  }

  function setFixture(next: 'ime' | 'giant') {
    fixture = next;
    adjacentArrived = false;
    const text = next === 'ime' ? IME_LEASE : GIANT_FIXTURE;
    if (text.length > PROJECTION_LIMITS.units) throw new Error('ResourceLimit');
    harnessState = createImeHarnessState(text);
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text }, effects: appendLease.of(harnessState) });
    view.dispatch({ selection: { anchor: next === 'ime' ? text.length : 0 }, scrollIntoView: true });
    view.focus();
    status = next === 'ime' ? 'IME fixture mounted; caret at final grapheme.' : `Giant fixture mounted: ${text.length} UTF-16 units, no newline.`;
    void record('fixtureMounted', { fixture: next, units: text.length, projectionLimit: PROJECTION_LIMITS.units });
  }

  function arriveAdjacentLease() {
    if (fixture !== 'ime' || adjacentArrived) return;
    const beforeFocused = view.hasFocus;
    const beforeComposing = view.composing;
    harnessState = { ...harnessState, projection: view.state.doc.toString(), composing };
    const reconciled = reconcileAdjacentLease(harnessState, 'right', ADJACENT_LEASE);
    const retainedUnits = view.state.doc.length;
    view.dispatch({
      changes: { from: retainedUnits, insert: reconciled.projection.slice(retainedUnits) },
      effects: appendLease.of(reconciled),
    });
    harnessState = view.state.field(leaseProjection);
    adjacentArrived = true;
    status = `Adjacent lease arrived while composition=${composing}. Finish candidate normally.`;
    void record('adjacentLeaseArrived', {
      composing, beforeFocused, afterFocused: view.hasFocus,
      beforeViewComposing: beforeComposing, afterViewComposing: view.composing,
      leaseCount: harnessState.leases.length, units: ADJACENT_LEASE.length,
    });
  }

  async function copySelection() {
    const selection = view.state.selection.main;
    const text = view.state.sliceDoc(selection.from, selection.to);
    await navigator.clipboard.writeText(text);
    status = `Copied ${text.length} UTF-16 units from visible projection.`;
    await record('selectionCopied', { fixture, from: selection.from, to: selection.to, units: text.length });
  }

  async function saveEvidence() {
    const selection = view.state.selection.main;
    await invoke('p03_save_summary', { summary: {
      fixture, adjacentArrived, composing, inputCommits,
      documentUnits: view.state.doc.length,
      selection: { from: selection.from, to: selection.to },
      hasNewline: view.state.doc.toString().includes('\n'),
    } });
    status = 'Evidence flushed. See run directory p03-summary.json and p03-events.ndjson.';
  }

  onMount(() => {
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: IME_LEASE,
        extensions: [
          basicSetup,
          EditorView.lineWrapping,
          leaseProjection,
          keymap.of([{ key: 'Ctrl-Shift-l', run: () => { arriveAdjacentLease(); return true; } }]),
          EditorView.contentAttributes.of({ 'aria-label': 'P03 bounded projected lease editor', 'aria-describedby': 'fixture-description' }),
          EditorView.updateListener.of(update => {
            if (!update.docChanged) return;
            const userInput = update.transactions.some(transaction => transaction.isUserEvent('input'));
            void record('documentChanged', { userInput, composing, units: update.state.doc.length });
          }),
        ],
      }),
    });
    const content = view.contentDOM;
    const start = () => { harnessState = startComposition(harnessState); composing = true; void record('compositionStart'); };
    const update = (event: CompositionEvent) => void record('compositionUpdate', redactEventText(event.data));
    const end = (event: CompositionEvent) => {
      harnessState = finishComposition(harnessState, event.data);
      composing = false;
      void record(event.data ? 'compositionEndPendingInput' : 'compositionCancel', redactEventText(event.data));
    };
    const input = (domEvent: Event) => {
      const event = domEvent as InputEvent;
      const previous = harnessState.inputCommits;
      harnessState = noteCommittedInput(harnessState, event);
      inputCommits = harnessState.inputCommits;
      if (inputCommits !== previous) void record('inputCommit', { ordinal: inputCommits, inputType: event.inputType, ...redactEventText(event.data) });
    };
    const blur = () => {
      const wasPending = harnessState.composing || harnessState.pendingComposition;
      harnessState = cancelCompositionOnBlur(harnessState);
      composing = false;
      if (wasPending) void record('compositionBlurCancel');
    };
    content.addEventListener('compositionstart', start);
    content.addEventListener('compositionupdate', update);
    content.addEventListener('compositionend', end);
    content.addEventListener('input', input);
    content.addEventListener('blur', blur);
    view.dispatch({ selection: { anchor: IME_LEASE.length } });
    view.focus();
    void record('harnessReady', { fixture: 'ime', units: IME_LEASE.length });
    return () => { content.removeEventListener('compositionstart', start); content.removeEventListener('compositionupdate', update); content.removeEventListener('compositionend', end); content.removeEventListener('input', input); content.removeEventListener('blur', blur); view.destroy(); };
  });
</script>

<svelte:head><title>P03 Native Projection Experiment</title></svelte:head>
<main>
  <h1>P03 native bounded projection experiment</h1>
  <p id="fixture-description">Only current bounded lease text is editable and exposed to accessibility APIs. No whole-file mirror or synthetic newline exists.</p>
  <div class="controls" role="group" aria-label="Deterministic fixtures">
    <button type="button" on:click={() => setFixture('ime')}>Mount IME boundary fixture</button>
    <span>Press Ctrl+Shift+L inside editor to arrive adjacent lease without moving focus.</span>
    <button type="button" on:click={() => setFixture('giant')}>Mount 20,001 combining + RTL fixture</button>
    <button type="button" on:click={copySelection}>Copy editor selection</button>
    <button type="button" on:click={saveEvidence}>Save evidence summary</button>
  </div>
  <dl><dt>Fixture</dt><dd>{fixture}</dd><dt>Composition active</dt><dd>{composing}</dd><dt>Input commits</dt><dd>{inputCommits}</dd><dt>Adjacent arrived</dt><dd>{adjacentArrived}</dd></dl>
  <div class="editor" bind:this={host}></div>
  <p role="status">{status}</p>
</main>

<style>
  :global(body) { margin: 0; font-family: "Segoe UI", sans-serif; background: #10141c; color: #f4f6fb; }
  main { padding: 16px; }
  .controls { display: flex; flex-wrap: wrap; gap: 8px; margin: 12px 0; }
  button { min-height: 32px; }
  dl { display: grid; grid-template-columns: max-content 1fr; gap: 4px 12px; }
  dt { font-weight: 600; }
  dd { margin: 0; }
  .editor { border: 2px solid #87b7ff; min-height: 280px; background: white; color: black; }
  :global(.cm-editor) { min-height: 280px; font: 16px/1.5 Consolas, monospace; }
  :global(.cm-content) { min-height: 260px; }
</style>
