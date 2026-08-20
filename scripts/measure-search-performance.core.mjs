export function validatePhase0HarnessSamples(harnessJson) {
  if (!Array.isArray(harnessJson) || harnessJson.length === 0) {
    throw new Error('phase0 harness samples missing');
  }

  let boundaryCount = 0;
  for (const [index, sample] of harnessJson.entries()) {
    if (!sample || typeof sample !== 'object') throw new Error(`sample ${index} invalid`);
    const ops = sample.observed_operations;
    if (!Array.isArray(ops)) throw new Error(`sample ${index} observed_operations invalid`);
    if (ops.includes('RecursiveFilesystemScan')) throw new Error(`sample ${index} contains RecursiveFilesystemScan`);
    const everythingBoundaryCount = ops.filter((op) => op === 'EverythingBoundary').length;
    if (everythingBoundaryCount !== 1) throw new Error(`sample ${index} EverythingBoundary count invalid`);

    const boundaryTrace = sample.boundary_trace;
    if (!Array.isArray(boundaryTrace) || boundaryTrace.length !== 2) {
      throw new Error(`sample ${index} boundary_trace invalid`);
    }
    if (!boundaryTrace.every((entry) => typeof entry === 'string' && entry.length > 0)) {
      throw new Error(`sample ${index} boundary_trace entries invalid`);
    }
    boundaryCount += everythingBoundaryCount;

    if (typeof sample.stale_count !== 'number' || sample.stale_count < 0) throw new Error(`sample ${index} stale_count invalid`);
    if (typeof sample.latest_count !== 'number' || sample.latest_count < 0) throw new Error(`sample ${index} latest_count invalid`);
    if (!Array.isArray(sample.stale_entries)) throw new Error(`sample ${index} stale_entries invalid`);
    if (!Array.isArray(sample.latest_entries)) throw new Error(`sample ${index} latest_entries invalid`);
  }

  return { runtime_sample_count: harnessJson.length, boundary_count: boundaryCount };
}
