<script lang="ts">
  import ContextMenu from '../../../src/components/ContextMenu.svelte';
  import ContextMenuItem from '../../../src/components/ContextMenuItem.svelte';
  import SpeechIndicatorSurface from '../../../src/components/SpeechIndicatorSurface.svelte';
</script>

<ContextMenu ariaLabel="Generic baseline menu" style="left:20px;top:60px;width:220px">
  <ContextMenuItem icon="settings">Production menu item</ContextMenuItem>
</ContextMenu>
<ContextMenu ariaLabel="Bar baseline menu" className="context-menu-overlay" style="left:20px;top:180px;width:220px">
  <ContextMenuItem icon="settings">Production bar menu item</ContextMenuItem>
</ContextMenu>
<SpeechIndicatorSurface />
