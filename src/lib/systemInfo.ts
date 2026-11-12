import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';

export type SampleState = 'ready' | 'warming_up' | 'partial' | 'truncated' | 'unavailable';
export type SampleReason = 'no_baseline' | 'provider_not_verified' | 'not_supported' | 'access_denied' | 'query_failed' | 'timed_out' | 'limit_reached' | 'stale' | 'not_connected';
export type Sample<T> = { state: SampleState; value: T | null; reason: SampleReason | null; source: string; unit: string | null; sampledAtMs: number | null };
export type GpuMetric = { id: string; name: string | null; utilizationPercent: Sample<number> };
export type NetworkAdapter = {
  id: string; name: string; kind: 'physical' | 'vpn' | 'virtual' | 'unknown';
  ipv4: string[]; ipv6: string[]; gateways: string[]; dnsServers: string[];
  linkReceiveBitsPerSecond: Sample<number>; linkTransmitBitsPerSecond: Sample<number>;
  receiveBytesPerSecond: Sample<number>; transmitBytesPerSecond: Sample<number>;
};
export type StorageRoot = { root: string; kind: 'fixed' | 'removable' | 'mapped' | 'unknown'; filesystem: Sample<string>; totalBytes: Sample<number>; totalFreeBytes: Sample<number>; callerAvailableBytes: Sample<number> };
export type SystemInfoSnapshot = {
  schemaVersion: 1;
  openGeneration: number; tabRevision: number; snapshotRevision: number; capturedAtMs: number;
  live: {
    state: SampleState; sampledAtMs: number | null; cpu: Sample<number>;
    memory: { totalBytes: Sample<number>; availableBytes: Sample<number>; usedPercent: Sample<number> };
    gpus: GpuMetric[]; network: NetworkAdapter[]; uptimeMs: Sample<number>;
    sensors: { cpuTemperatureC: Sample<number>; gpuTemperatureC: Sample<number> };
  };
  storage: { state: SampleState; sampledAtMs: number | null; roots: StorageRoot[] };
  inventory: {
    state: SampleState; sampledAtMs: number | null; cpuName: Sample<string>;
    physicalCores: Sample<number>; logicalProcessors: Sample<number>; installedRamBytes: Sample<number>;
    gpuNames: Sample<string[]>; vramBytes: Sample<number>; board: Sample<string>; bios: Sample<string>; windowsVersion: Sample<string>;
  };
};
export function getSystemInfoSnapshot(args: { openGeneration: number; tabRevision: number; afterRevision?: number | null }): Promise<SystemInfoSnapshot | null> {
  return invoke<SystemInfoSnapshot | null>(IPC_COMMANDS.getSystemInfoSnapshot, { ...args, afterRevision: args.afterRevision ?? null });
}
