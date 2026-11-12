// Independent of production types: freezes the public wire schema.
export const context = (openGeneration = 1, tabRevision = 1, activeTab = 'processes', focusPid: number | null = null, visible = true) => ({ visible, activeTab, openGeneration, tabRevision, focusPid });
export function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
export const processRow = (pid: number, name = `Process-${pid}`) => ({ pid, name, status: 'running', isKillable: true, cpuPercent: 3, memoryBytes: 4096, memoryPercent: 1, creationTime100ns: `${pid}000`, normalizedImagePath: `c:\\fixture\\${name}.exe`, childProcessCount: 0, descendantProcessCount: 0, listeningPorts: [] });
const sample = <T>(value: T | null, unit: string | null = null, source = 'fixture') => ({ state: value === null ? 'unavailable' : 'ready', value, reason: value === null ? 'not_supported' : null, source, unit, sampledAtMs: value === null ? null : 1000 });
export function snapshot(openGeneration = 1, tabRevision = 2, snapshotRevision = 1) {
  return {
    schemaVersion: 1, openGeneration, tabRevision, snapshotRevision, capturedAtMs: 1000,
    live: { state: 'partial', sampledAtMs: 1000, cpu: sample(73, 'percent', 'PDH whole-machine'),
      memory: { totalBytes: sample(16000000000, 'bytes'), availableBytes: sample(4000000000, 'bytes'), usedPercent: sample(75, 'percent') },
      gpus: [{ id: 'adapter-0', name: 'Fixture GPU', utilizationPercent: sample(47, 'percent', 'PDH busiest engine, not total GPU') }],
      network: [{ id: 'interface-0', name: 'Fixture VPN', kind: 'vpn', ipv4: ['192.0.2.7'], ipv6: ['2001:db8::7'], gateways: [], dnsServers: ['192.0.2.53'], linkReceiveBitsPerSecond: sample(1000000000, 'bits_per_second'), linkTransmitBitsPerSecond: sample(1000000000, 'bits_per_second'), receiveBytesPerSecond: sample(1200, 'bytes_per_second'), transmitBytesPerSecond: sample(600, 'bytes_per_second') }],
      uptimeMs: sample(3600000, 'milliseconds'), sensors: { cpuTemperatureC: { ...sample(null, 'celsius'), reason: 'provider_not_verified' }, gpuTemperatureC: { ...sample(null, 'celsius'), reason: 'provider_not_verified' } } },
    storage: { state: 'partial', sampledAtMs: 1000, roots: [{ root: 'C:\\', kind: 'fixed', filesystem: sample('NTFS'), totalBytes: sample(1000000000000, 'bytes'), totalFreeBytes: sample(300000000000, 'bytes'), callerAvailableBytes: sample(200000000000, 'bytes') }] },
    inventory: { state: 'partial', sampledAtMs: 1000, cpuName: sample('Fixture CPU'), physicalCores: sample(8), logicalProcessors: sample(16), installedRamBytes: sample(16000000000, 'bytes'), gpuNames: sample(['Fixture GPU']), vramBytes: sample(null, 'bytes'), board: sample('Fixture Board'), bios: sample('Fixture BIOS'), windowsVersion: sample('Windows-reported fixture') }
  };
}

export function quotaLimitedSnapshot() {
  const wire=snapshot(); const gib=1024 ** 3;
  // Physical size is a test oracle only, NOT a new native field/measurement.
  // GetDiskFreeSpaceExW's total is caller-quota-limited, whereas total free is
  // the distinct volume-wide value and caller available is quota-aware again.
  const root=wire.storage.roots[0];
  root.totalBytes={...root.totalBytes,value:100*gib,source:'GetDiskFreeSpaceExW'};
  root.totalFreeBytes={...root.totalFreeBytes,value:300*gib,source:'GetDiskFreeSpaceExW'};
  root.callerAvailableBytes={...root.callerAvailableBytes,value:25*gib,source:'GetDiskFreeSpaceExW'};
  return {wire,physicalCapacityBytes:512*gib};
}

// Redesign acceptance: deliberately not the mockup's fictional product data.
// Keep dynamic collections and mixed availability visible to regression tests.
export function metricStripSnapshot() {
  const wire = quotaLimitedSnapshot().wire;
  wire.storage.roots.push({
    root: 'Z:\\', kind: 'mapped', filesystem: sample('ReFS'),
    totalBytes: sample(64 * 1024 ** 3, 'bytes', 'Fixture mapped capacity'),
    totalFreeBytes: sample(16 * 1024 ** 3, 'bytes', 'Fixture mapped free'),
    callerAvailableBytes: sample(null, 'bytes', 'Fixture mapped quota unavailable'),
  });
  wire.live.network.push({
    id: 'interface-1', name: 'Fixture Ethernet with a deliberately long adapter name', kind: 'physical',
    ipv4: ['198.51.100.24', '198.51.100.25'], ipv6: ['2001:db8:1234:5678:90ab:cdef:1234:5678'],
    gateways: ['198.51.100.1'], dnsServers: ['198.51.100.53', '2001:db8::53'],
    linkReceiveBitsPerSecond: sample(2500000000, 'bits_per_second'),
    linkTransmitBitsPerSecond: sample(2500000000, 'bits_per_second'),
    receiveBytesPerSecond: { ...sample(null, 'bytes_per_second', 'Fixture Ethernet receive baseline'), state: 'warming_up', reason: 'no_baseline' },
    transmitBytesPerSecond: { ...sample(2048, 'bytes_per_second', 'Fixture Ethernet stale send'), state: 'partial', reason: 'stale' },
  });
  wire.live.gpus.push({ id: 'adapter-1', name: 'Fixture secondary GPU', utilizationPercent: sample(null, 'percent', 'Fixture secondary GPU unavailable') });
  wire.inventory.gpuNames.value = ['Fixture GPU', 'Fixture secondary GPU'];
  return wire;
}
