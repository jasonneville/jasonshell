<script lang="ts">
  import ContextMenu from '../../../src/components/ContextMenu.svelte';
  import StackConfirmDialog from '../../../src/components/StackConfirmDialog.svelte';
  import '../../../src/components/ProcessManagerSurface.css';
  export let dialog = false;
</script>

<!-- Production components and app.css base controls only; no Tauri IPC is used. -->
{#if dialog}
  <StackConfirmDialog title="Delete item?" message="This cannot be undone." confirmLabel="Delete" tone="danger" onConfirm={() => {}} onCancel={() => {}} />
{:else}
  <main>
    <button type="button">Plain button</button>
    <input aria-label="Plain input" />
    <select aria-label="Plain select"><option>One</option><option>Two</option></select>
    <div role="button" tabindex="0">Role button</div>
    <!-- Wrapper-owned focus: production ProcessManager CSS gives the label the ring and the input none. -->
    <label class="process-filter"><input aria-label="Filter processes" /></label>
    <ContextMenu ariaLabel="Fixture menu" tabindex={0}>
      <button type="button" role="menuitem" class="context-menu-item">Item A</button>
      <button type="button" role="menuitem" class="context-menu-item">Item B</button>
    </ContextMenu>
  </main>
{/if}
