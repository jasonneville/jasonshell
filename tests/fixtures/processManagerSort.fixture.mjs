// Deterministic data and legacy ordering oracle; never imported by application code.
export function processSortFixture(count = 192) {
  return Array.from({ length: count }, (_, index) => ({
    pid: count - index,
    name: index % 11 === 0 ? 'svchost.exe' : `Tool ${index % 17}`,
    executablePath: index % 13 === 0 ? 'C:/Windows/System32/tool.exe' : 'C:/tools/tool.exe',
    cpuPercent: index % 7 === 0 ? null : index % 9,
    memoryBytes: index % 5 === 0 ? null : (index % 23) * 1024,
    gpuPercent: index % 4 === 0 ? null : index % 3,
    threadCount: index % 6 === 0 ? undefined : index % 8,
    startTimeMs: index % 3 === 0 ? null : 1700000000000 + index % 5,
    taskbarActive: index % 19 === 0,
    status: 'running',
    isKillable: true
  }));
}

export function countedPids(values) {
  const counts = { iterations: 0, yielded: 0 };
  return {
    counts,
    iterable: {
      *[Symbol.iterator]() {
        counts.iterations += 1;
        for (const value of values) {
          counts.yielded += 1;
          yield value;
        }
      }
    }
  };
}

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' });
const groupIndex = { applications: 0, background: 1, windows: 2 };

export function legacyClassifier(isWindowsProcess) {
  return (process, options = {}) => {
    const active = new Set(options.taskbarActivePids ?? []);
    if (active.has(process.pid) || process.taskbarActive === true) return 'applications';
    return isWindowsProcess(process) ? 'windows' : 'background';
  };
}

// Preserve pre-optimization two-pass algorithm, including null/undefined and PID ties.
export function legacyOrdering(classifyProcessGroup) {
  function compareGroups(left, right, options) {
    return groupIndex[classifyProcessGroup(left, options)] - groupIndex[classifyProcessGroup(right, options)];
  }
  function sortProcesses(processes, sort, options = {}) {
    return [...processes].sort((left, right) => {
      const groups = compareGroups(left, right, options);
      if (groups !== 0) return groups;
      const comparison = sort.column === 'name'
        ? collator.compare(left.name, right.name)
        : (typeof left[sort.column] === 'number' ? left[sort.column] : Number.NEGATIVE_INFINITY)
          - (typeof right[sort.column] === 'number' ? right[sort.column] : Number.NEGATIVE_INFINITY);
      if (comparison !== 0) return sort.direction === 'asc' ? comparison : -comparison;
      return left.pid - right.pid;
    });
  }
  function orderProcessRefresh(previous, next, sort, options = {}) {
    const sorted = sortProcesses(next, sort, options);
    if (!options.preserveExistingOrder || previous.length === 0) return sorted;
    const previousIndex = new Map(previous.map((process, index) => [process.pid, index]));
    const sortedIndex = new Map(sorted.map((process, index) => [process.pid, index]));
    return [...sorted].sort((left, right) => {
      const groups = compareGroups(left, right, options);
      if (groups !== 0) return groups;
      const a = previousIndex.get(left.pid);
      const b = previousIndex.get(right.pid);
      if (typeof a === 'number' && typeof b === 'number') return a - b;
      if (typeof a === 'number') return -1;
      if (typeof b === 'number') return 1;
      return (sortedIndex.get(left.pid) ?? 0) - (sortedIndex.get(right.pid) ?? 0);
    });
  }
  return { sortProcesses, orderProcessRefresh };
}
