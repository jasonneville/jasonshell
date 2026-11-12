import type { Sample, SampleState } from './systemInfo';

export function sampleStateLabel(state: SampleState): string {
  return { ready: 'Ready', warming_up: 'Warming up', partial: 'Partial', truncated: 'Truncated', unavailable: 'Unavailable' }[state];
}

export function sampleNote(sample: Sample<unknown>): string {
  const reason = sample.reason?.replaceAll('_', ' ');
  return [sample.state === 'ready' ? '' : sampleStateLabel(sample.state), reason].filter(Boolean).join(' · ');
}

function scaled(value: number, base: number, units: string[]): string {
  let index = 0;
  while (Math.abs(value) >= base && index < units.length - 1) { value /= base; index += 1; }
  return `${value.toLocaleString(undefined, { maximumFractionDigits: 1 })} ${units[index]}`;
}

export function sampleValue(sample: Sample<unknown>): string {
  const value = sample.value;
  if (sample.state === 'unavailable' || value === null || (typeof value === 'number' && !Number.isFinite(value))) {
    return sample.state === 'warming_up' ? 'Warming up' : 'Unavailable';
  }
  if (typeof value !== 'number') return Array.isArray(value) ? value.join(', ') || 'None reported' : String(value);
  switch (sample.unit) {
    case 'percent': return `${value.toLocaleString(undefined, { maximumFractionDigits: 1 })}%`;
    case 'bytes': return scaled(value, 1024, ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB']);
    case 'bytes_per_second': return `${scaled(value, 1024, ['B', 'KiB', 'MiB', 'GiB'])}/s`;
    case 'bits_per_second': return `${scaled(value, 1000, ['bit', 'kbit', 'Mbit', 'Gbit'])}/s`;
    case 'celsius': return `${value.toLocaleString(undefined, { maximumFractionDigits: 1 })} °C`;
    case 'milliseconds': {
      const minutes = Math.floor(value / 60_000);
      return `${Math.floor(minutes / 1440)}d ${Math.floor(minutes / 60) % 24}h ${minutes % 60}m`;
    }
    default: return value.toLocaleString();
  }
}

export function sampledTime(time: number | null): string {
  return time === null ? 'Not sampled yet' : `Updated ${new Date(time).toLocaleTimeString()}`;
}
