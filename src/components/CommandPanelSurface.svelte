<script lang="ts">
  import { onMount } from 'svelte';
  import './CommandPanelSurface.css';
  import MeltActionButton from './melt/MeltActionButton.svelte';
  import {
    QUICK_COMMAND_MODES,
    formatQuickCommandArgsTextarea,
    formatQuickCommandCommandsTextarea,
    loadQuickCommandsSettings,
    parseQuickCommandArgsTextarea,
    parseQuickCommandCommandsTextarea,
    runQuickCommand,
    saveQuickCommandsSettings,
    type QuickCommandEntry,
    type QuickCommandMode
  } from '../lib/quickCommands';
  import { hideCommandPanel } from '../lib/commandPanel';

  type CommandEditorModel = {
    id: string | null;
    label: string;
    mode: QuickCommandMode;
    targetPath: string;
    cwd: string;
    argsText: string;
    commandsText: string;
  };

  const modeLabels: Record<QuickCommandMode, string> = {
    direct: 'Program',
    commandBlock: 'Command block'
  };

  let entries: QuickCommandEntry[] = [];
  let loading = true;
  let saving = false;
  let runningId: string | null = null;
  let formErrors: string[] = [];
  let panelError = '';
  let editor: CommandEditorModel = blankEditor();

  function blankEditor(): CommandEditorModel {
    return {
      id: null,
      label: '',
      mode: 'direct',
      targetPath: '',
      cwd: '',
      argsText: '',
      commandsText: ''
    };
  }

  function startNewEntry() {
    formErrors = [];
    panelError = '';
    editor = blankEditor();
  }

  function startEditEntry(entry: QuickCommandEntry) {
    formErrors = [];
    panelError = '';
    editor = {
      id: entry.id,
      label: entry.label,
      mode: entry.mode,
      targetPath: entry.targetPath,
      cwd: entry.cwd ?? '',
      argsText: formatQuickCommandArgsTextarea(entry.args),
      commandsText: formatQuickCommandCommandsTextarea(entry.commands)
    };
  }

  function sortedEntries(values: readonly QuickCommandEntry[]): QuickCommandEntry[] {
    return [...values].sort((left, right) => left.label.localeCompare(right.label));
  }

  function toSlug(value: string): string {
    return value
      .toLowerCase()
      .trim()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '');
  }

  function validateEditor(): string[] {
    const errors: string[] = [];
    if (!editor.label.trim()) {
      errors.push('Label is required.');
    }
    if (editor.mode === 'direct' && !editor.targetPath.trim()) {
      errors.push('Program is required.');
    }
    if (editor.mode === 'commandBlock' && parseQuickCommandCommandsTextarea(editor.commandsText).length === 0) {
      errors.push('Add at least one command.');
    }
    const hasInvalidEmptyArg = editor.argsText
      .split(/\r?\n/u)
      .some((line) => line.length > 0 && !line.trim());
    if (hasInvalidEmptyArg) {
      errors.push('Arguments must not include whitespace-only lines.');
    }
    return errors;
  }

  async function refreshEntries() {
    loading = true;
    panelError = '';
    try {
      const quickCommands = await loadQuickCommandsSettings();
      entries = sortedEntries(quickCommands.entries);
      if (editor.id) {
        const current = entries.find((entry) => entry.id === editor.id);
        if (!current) {
          editor = blankEditor();
        }
      }
    } catch (error) {
      panelError = 'Quick command settings are unavailable.';
      console.error('Failed to load quick commands', error);
    } finally {
      loading = false;
    }
  }

  async function saveEntry() {
    formErrors = validateEditor();
    if (formErrors.length) {
      return;
    }

    const id = editor.id ?? toSlug(editor.label);
    if (!id) {
      formErrors = ['Command id could not be derived from Label.'];
      return;
    }

    const nextEntry: QuickCommandEntry = {
      id,
      label: editor.label.trim(),
      mode: editor.mode,
      targetPath: editor.mode === 'direct' ? editor.targetPath.trim() : '',
      cwd: editor.cwd.trim() ? editor.cwd.trim() : null,
      args: editor.mode === 'direct' ? parseQuickCommandArgsTextarea(editor.argsText) : [],
      commands:
        editor.mode === 'commandBlock' ? parseQuickCommandCommandsTextarea(editor.commandsText) : []
    };

    const current = entries.filter((entry) => entry.id !== id);
    const nextEntries = [...current, nextEntry];

    saving = true;
    panelError = '';
    try {
      const saved = await saveQuickCommandsSettings({ entries: nextEntries });
      entries = sortedEntries(saved.entries);
      startEditEntry(nextEntry);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      formErrors = [message];
    } finally {
      saving = false;
    }
  }

  async function deleteEntry(id: string) {
    saving = true;
    formErrors = [];
    panelError = '';
    try {
      const saved = await saveQuickCommandsSettings({
        entries: entries.filter((entry) => entry.id !== id)
      });
      entries = sortedEntries(saved.entries);
      if (editor.id === id) {
        editor = blankEditor();
      }
    } catch (error) {
      panelError = error instanceof Error ? error.message : String(error);
    } finally {
      saving = false;
    }
  }

  async function runEntry(id: string) {
    runningId = id;
    panelError = '';
    formErrors = [];
    try {
      await runQuickCommand({ id });
      await hideCommandPanel();
    } catch (error) {
      panelError = error instanceof Error ? error.message : String(error);
    } finally {
      runningId = null;
    }
  }

  function closePanel() {
    void hideCommandPanel().catch((error) => {
      console.error('Failed to hide command panel', error);
    });
  }

  onMount(() => {
    void refreshEntries();
  });
</script>

<div class="command-panel" id="command-panel" role="dialog" aria-labelledby="command-panel-title">
  <header class="command-panel-header">
    <div>
      <p>JasonShell</p>
      <h1 id="command-panel-title">Quick Commands</h1>
    </div>
    <MeltActionButton ariaLabel="Close quick commands" onClick={closePanel}>x</MeltActionButton>
  </header>

  {#if panelError}
    <p class="command-panel-error" role="alert">{panelError}</p>
  {/if}

  <section class="command-panel-layout">
    <aside class="command-list" aria-label="Saved commands">
      <div class="command-list-header">
        <h2>Saved</h2>
        <MeltActionButton ariaLabel="Create command" onClick={startNewEntry}>New</MeltActionButton>
      </div>
      {#if loading}
        <p class="command-list-state">Loading commands…</p>
      {:else if !entries.length}
        <p class="command-list-state">No quick commands saved.</p>
      {:else}
        <ul>
          {#each entries as entry (entry.id)}
            <li>
              <div class="command-row">
                <strong>{entry.label}</strong>
                <span>{modeLabels[entry.mode]}</span>
              </div>
              <div class="command-row-actions">
                <MeltActionButton
                  ariaLabel={`Run ${entry.label}`}
                  disabled={Boolean(runningId || saving)}
                  onClick={() => void runEntry(entry.id)}
                >
                  {runningId === entry.id ? 'Running…' : 'Run'}
                </MeltActionButton>
                <MeltActionButton
                  ariaLabel={`Edit ${entry.label}`}
                  disabled={Boolean(runningId || saving)}
                  onClick={() => startEditEntry(entry)}
                >
                  Edit
                </MeltActionButton>
                <MeltActionButton
                  ariaLabel={`Delete ${entry.label}`}
                  disabled={Boolean(runningId || saving)}
                  onClick={() => void deleteEntry(entry.id)}
                >
                  Delete
                </MeltActionButton>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </aside>

    <section class="command-editor" aria-label="Command editor">
      <h2>{editor.id ? 'Edit command' : 'New command'}</h2>

      {#if formErrors.length}
        <ul class="command-form-errors" role="alert">
          {#each formErrors as error (error)}
            <li>{error}</li>
          {/each}
        </ul>
      {/if}

      <label>
        <span>Label</span>
        <input
          value={editor.label}
          maxlength="96"
          spellcheck="false"
          on:input={(event) => {
            editor = { ...editor, label: (event.currentTarget as HTMLInputElement).value };
          }}
        />
      </label>

      <label>
        <span>Mode</span>
        <select
          value={editor.mode}
          on:change={(event) => {
            editor = { ...editor, mode: (event.currentTarget as HTMLSelectElement).value as QuickCommandMode };
          }}
        >
          {#each QUICK_COMMAND_MODES as mode}
            <option value={mode}>{modeLabels[mode]}</option>
          {/each}
        </select>
      </label>

      {#if editor.mode === 'direct'}
        <label>
          <span>Program</span>
          <input
            value={editor.targetPath}
            spellcheck="false"
            placeholder="git.exe"
            on:input={(event) => {
              editor = { ...editor, targetPath: (event.currentTarget as HTMLInputElement).value };
            }}
          />
        </label>
      {/if}

      <label>
        <span>Working directory</span>
        <input
          value={editor.cwd}
          spellcheck="false"
          placeholder="Optional absolute path"
          on:input={(event) => {
            editor = { ...editor, cwd: (event.currentTarget as HTMLInputElement).value };
          }}
        />
      </label>

      {#if editor.mode === 'direct'}
        <label>
          <span>Arguments (one per line)</span>
          <textarea
            rows="5"
            spellcheck="false"
            value={editor.argsText}
            on:input={(event) => {
              editor = { ...editor, argsText: (event.currentTarget as HTMLTextAreaElement).value };
            }}
          ></textarea>
        </label>
      {:else}
        <label>
          <span>Commands (one per line)</span>
          <textarea
            rows="8"
            spellcheck="false"
            value={editor.commandsText}
            placeholder={'cd C:\\dev\\my-app\npython app.py'}
            on:input={(event) => {
              editor = { ...editor, commandsText: (event.currentTarget as HTMLTextAreaElement).value };
            }}
          ></textarea>
        </label>
      {/if}

      <div class="command-editor-actions">
        <MeltActionButton
          ariaLabel="Save command"
          disabled={saving || Boolean(runningId)}
          onClick={() => void saveEntry()}
        >
          {saving ? 'Saving…' : 'Save'}
        </MeltActionButton>
        <MeltActionButton
          ariaLabel="Cancel command editing"
          disabled={saving || Boolean(runningId)}
          onClick={startNewEntry}
        >
          Clear
        </MeltActionButton>
      </div>
    </section>
  </section>
</div>
