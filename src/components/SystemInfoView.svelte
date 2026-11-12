<script lang="ts">
  import './SystemInfoView.css';
  import type { Sample, SampleState, SystemInfoSnapshot } from '../lib/systemInfo';
  import { sampleNote, sampleStateLabel, sampleValue, sampledTime } from '../lib/systemInfoState';

  let { snapshot, loading = false, error = '' }: { snapshot: SystemInfoSnapshot | null; loading?: boolean; error?: string } = $props();

  type SamplingEntry = [label: string, sample: Sample<unknown>];

  function usedRam(total: Sample<number>, available: Sample<number>) {
    if (total.state === 'unavailable' || available.state === 'unavailable' || total.value === null || available.value === null
      || !Number.isFinite(total.value) || !Number.isFinite(available.value) || available.value < 0 || total.value < available.value) return null;
    return sampleValue({ ...total, value: total.value - available.value });
  }
</script>

{#snippet metric(label: string, sample: Sample<unknown>, caption = '')}
  <div class="system-info-metric">
    <dt>{label}</dt>
    <dd>
      <strong>{sampleValue(sample)}</strong>
      {#if sampleNote(sample)}<span class="system-info-note">{sampleNote(sample)}</span>{/if}
      {#if caption}<small>{caption}</small>{/if}
    </dd>
  </div>
{/snippet}

{#snippet sampling(section: string, state: SampleState, time: number | null, entries: SamplingEntry[])}
  <details class="system-info-sampling">
    <summary>{section} sampling details</summary>
    <p>{sampleStateLabel(state)} · {sampledTime(time)}</p>
    <dl>
      {#each entries as entry (entry)}
        <div class="system-info-provenance">
          <dt>{entry[0]}</dt>
          <dd>
            <span>{entry[1].source}</span>
            <span>{sampleStateLabel(entry[1].state)} · {entry[1].reason?.replaceAll('_', ' ') ?? 'No reported issue'} · unit: {entry[1].unit ?? 'text / count'}</span>
            <span>{sampledTime(entry[1].sampledAtMs)}{entry[1].sampledAtMs !== null ? ` · timestamp ${entry[1].sampledAtMs} ms` : ''}</span>
          </dd>
        </div>
      {/each}
    </dl>
  </details>
{/snippet}

<!-- svelte-ignore a11y_no_noninteractive_tabindex (The scroll region needs keyboard focus for Page Up/Down and arrow scrolling.) -->
<div class="system-info-view" role="region" tabindex="0" aria-label="System information details">
  <div class="system-info-update" role="status" aria-live="polite">
    {#if error}{error}{:else if loading}Loading system information…{:else if snapshot}{sampledTime(snapshot.capturedAtMs)} · cached native readings{:else}Waiting for system information…{/if}
  </div>
  {#if snapshot}
    <section class="system-info-section system-info-strip" aria-labelledby="system-overview-title">
      <h2 id="system-overview-title">Overview</h2>
      <p class="system-info-description">Whole-machine readings · independent of the process filter</p>
      <dl class="system-info-metrics system-info-overview">
        {@render metric('Whole-machine CPU', snapshot.live.cpu)}
        <div class="system-info-memory">
          {@render metric('RAM used', snapshot.live.memory.usedPercent)}
          <p class="system-info-description">
            {#if usedRam(snapshot.live.memory.totalBytes, snapshot.live.memory.availableBytes)}{usedRam(snapshot.live.memory.totalBytes, snapshot.live.memory.availableBytes)} used · {/if}
            <span>Usable RAM: {sampleValue(snapshot.live.memory.totalBytes)}</span>
            <span>Available RAM: {sampleValue(snapshot.live.memory.availableBytes)}</span>
          </p>
          {#if sampleNote(snapshot.live.memory.totalBytes) || sampleNote(snapshot.live.memory.availableBytes)}
            <p class="system-info-note">{sampleNote(snapshot.live.memory.totalBytes)} {sampleNote(snapshot.live.memory.availableBytes)}</p>
          {/if}
        </div>
        <div class="system-info-gpus">
          {#each snapshot.live.gpus as gpu (gpu.id)}
            {@render metric('GPU · busiest engine', gpu.utilizationPercent, gpu.name ?? 'GPU name unavailable')}
          {:else}<div class="system-info-empty">GPU busiest-engine usage unavailable</div>{/each}
          <p class="system-info-description">Busiest engine, not total GPU</p>
        </div>
        {@render metric('OS-reported uptime since boot', snapshot.live.uptimeMs)}
      </dl>
      {@render sampling('Overview', snapshot.live.state, snapshot.live.sampledAtMs, [
        ['Whole-machine CPU', snapshot.live.cpu], ['RAM used', snapshot.live.memory.usedPercent],
        ['Usable RAM', snapshot.live.memory.totalBytes], ['Available RAM', snapshot.live.memory.availableBytes],
        ['OS-reported uptime', snapshot.live.uptimeMs],
        ...snapshot.live.gpus.map((gpu): SamplingEntry => [gpu.name ?? 'GPU', gpu.utilizationPercent])
      ])}
    </section>
    <section class="system-info-section" aria-labelledby="system-storage-title">
      <h2 id="system-storage-title">Storage</h2>
      <div class="system-info-section-content">
      {#each snapshot.storage.roots as root (root.root)}
        <article class="system-info-item">
          <h3>{root.root} <span>{root.kind}</span></h3>
          <dl class="system-info-metrics">
            {@render metric('Filesystem', root.filesystem)}
            {@render metric('Caller-visible capacity (quota-aware)', root.totalBytes)}
            {@render metric('Total free space', root.totalFreeBytes)}
            {@render metric('Available to current caller (quota-aware)', root.callerAvailableBytes)}
          </dl>
        </article>
      {:else}<p class="system-info-empty">No storage roots reported · {sampleStateLabel(snapshot.storage.state)}</p>{/each}
      {@render sampling('Storage', snapshot.storage.state, snapshot.storage.sampledAtMs, snapshot.storage.roots.flatMap((root): SamplingEntry[] => [
        [`${root.root} filesystem`, root.filesystem], [`${root.root} caller-visible capacity`, root.totalBytes],
        [`${root.root} total free`, root.totalFreeBytes], [`${root.root} caller available`, root.callerAvailableBytes]
      ]))}
      </div>
    </section>
    <section class="system-info-section" aria-labelledby="system-sensors-title">
      <h2 id="system-sensors-title">Sensors</h2>
      <div class="system-info-section-content">
      <dl class="system-info-metrics">
        {@render metric('CPU temperature', snapshot.live.sensors.cpuTemperatureC)}
        {@render metric('GPU temperature', snapshot.live.sensors.gpuTemperatureC)}
      </dl>
      <p class="system-info-description">Temperatures require a verified running provider. Unsupported readings are not zero.</p>
      {@render sampling('Sensors', snapshot.live.state, snapshot.live.sampledAtMs, [
        ['CPU temperature', snapshot.live.sensors.cpuTemperatureC], ['GPU temperature', snapshot.live.sensors.gpuTemperatureC]
      ])}
      </div>
    </section>
    <section class="system-info-section" aria-labelledby="system-network-title">
      <h2 id="system-network-title">Network</h2>
      <div class="system-info-section-content">
      <p class="system-info-description">Per-adapter traffic and link speed are distinct. Local addresses only; adapter traffic is not summed.</p>
      {#each snapshot.live.network as adapter (adapter.id)}
        <article class="system-info-item">
          <h3>{adapter.name} <span>{adapter.kind}</span></h3>
          <dl class="system-info-metrics">
            {@render metric('Receive traffic', adapter.receiveBytesPerSecond)}
            {@render metric('Send traffic', adapter.transmitBytesPerSecond)}
            {@render metric('Receive link speed', adapter.linkReceiveBitsPerSecond)}
            {@render metric('Send link speed', adapter.linkTransmitBitsPerSecond)}
            <div class="system-info-metric"><dt>Local IPv4</dt><dd>{#each adapter.ipv4 as address (address)}<span>{address}</span>{:else}None reported{/each}</dd></div>
            <div class="system-info-metric"><dt>Local IPv6</dt><dd>{#each adapter.ipv6 as address (address)}<span>{address}</span>{:else}None reported{/each}</dd></div>
            <div class="system-info-metric"><dt>Gateways</dt><dd>{adapter.gateways.join(', ') || 'None reported'}</dd></div>
            <div class="system-info-metric"><dt>DNS servers</dt><dd>{adapter.dnsServers.join(', ') || 'None reported'}</dd></div>
          </dl>
        </article>
      {:else}<p class="system-info-empty">No network adapters reported · {sampleStateLabel(snapshot.live.state)}</p>{/each}
      {@render sampling('Network', snapshot.live.state, snapshot.live.sampledAtMs, snapshot.live.network.flatMap((adapter): SamplingEntry[] => [
        [`${adapter.name} receive traffic`, adapter.receiveBytesPerSecond], [`${adapter.name} send traffic`, adapter.transmitBytesPerSecond],
        [`${adapter.name} receive link speed`, adapter.linkReceiveBitsPerSecond], [`${adapter.name} send link speed`, adapter.linkTransmitBitsPerSecond]
      ]))}
      </div>
    </section>
    <section class="system-info-section" aria-labelledby="system-hardware-title">
      <h2 id="system-hardware-title">Hardware</h2>
      <div class="system-info-section-content">
      <dl class="system-info-metrics">
        {@render metric('CPU model', snapshot.inventory.cpuName)}
        {@render metric('Physical cores', snapshot.inventory.physicalCores)}
        {@render metric('Logical processors', snapshot.inventory.logicalProcessors)}
        {@render metric('Installed RAM', snapshot.inventory.installedRamBytes)}
        {@render metric('GPU models', snapshot.inventory.gpuNames)}
        {@render metric('Dedicated VRAM', snapshot.inventory.vramBytes)}
        {@render metric('Motherboard', snapshot.inventory.board)}
        {@render metric('BIOS', snapshot.inventory.bios)}
        {@render metric('Windows-reported version', snapshot.inventory.windowsVersion)}
      </dl>
      {@render sampling('Hardware', snapshot.inventory.state, snapshot.inventory.sampledAtMs, [
        ['CPU model', snapshot.inventory.cpuName], ['Physical cores', snapshot.inventory.physicalCores], ['Logical processors', snapshot.inventory.logicalProcessors],
        ['Installed RAM', snapshot.inventory.installedRamBytes], ['GPU models', snapshot.inventory.gpuNames], ['Dedicated VRAM', snapshot.inventory.vramBytes],
        ['Motherboard', snapshot.inventory.board], ['BIOS', snapshot.inventory.bios], ['Windows-reported version', snapshot.inventory.windowsVersion]
      ])}
      </div>
    </section>
  {/if}
</div>
