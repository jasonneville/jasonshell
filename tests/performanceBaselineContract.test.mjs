import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import test from 'node:test';

const harnessUrl = new URL('../scripts/measure-performance.ps1', import.meta.url);
const gitignore = readFileSync(new URL(`..${'/'} .gitignore`.replace('/ ', '/'), import.meta.url), 'utf8');

const REQUIRED_SCENARIOS = [
  'cold-idle',
  '20+-windows',
  'notifications',
  'noisy-quick-commands-output',
  'terminal-hidden-prewarm',
  'fullscreen',
  'multi-monitor'
];

function harnessSource() {
  assert.equal(existsSync(harnessUrl), true, 'scripts/measure-performance.ps1 must exist');
  return readFileSync(harnessUrl, 'utf8');
}

function executableSource() {
  return stripPowerShellComments(harnessSource());
}

function stripPowerShellComments(source) {
  let output = '';
  let i = 0;
  let mode = 'code';
  while (i < source.length) {
    const ch = source[i];
    const next = source[i + 1];

    if (mode === 'code') {
      if (ch === '<' && next === '#') { mode = 'blockComment'; i += 2; continue; }
      if (ch === '#') { mode = 'lineComment'; i += 1; continue; }
      if (ch === "'") { output += ch; mode = 'single'; i += 1; continue; }
      if (ch === '"') { output += ch; mode = 'double'; i += 1; continue; }
      output += ch;
      i += 1;
      continue;
    }

    if (mode === 'lineComment') {
      if (ch === '\r' || ch === '\n') { output += ch; mode = 'code'; }
      i += 1;
      continue;
    }

    if (mode === 'blockComment') {
      if (ch === '#' && next === '>') { mode = 'code'; i += 2; continue; }
      if (ch === '\r' || ch === '\n') output += ch;
      i += 1;
      continue;
    }

    if (mode === 'single') {
      output += ch;
      if (ch === "'" && next === "'") { output += next; i += 2; continue; }
      if (ch === "'") mode = 'code';
      i += 1;
      continue;
    }

    output += ch;
    if (ch === '`' && i + 1 < source.length) { output += next; i += 2; continue; }
    if (ch === '"') mode = 'code';
    i += 1;
  }
  return output;
}

function assertMentions(source, values, label = 'source') {
  for (const value of values) {
    assert.match(source, new RegExp(escapeRegExp(value), 'i'), `${label} must mention ${value}`);
  }
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function scenarioLiteralSet(source) {
  const match = source.match(/\$RequiredScenarios\s*=\s*@\((?<body>[\s\S]*?)\)/i)
    ?? source.match(/\$Scenarios\s*=\s*@\((?<body>[\s\S]*?)\)/i);
  assert.ok(match?.groups?.body, 'scenario list must be a literal PowerShell array');
  return [...match.groups.body.matchAll(/'([^']+)'|"([^"]+)"/g)].map((m) => m[1] ?? m[2]);
}

test('performance artifact root is timestamped and ignored', () => {
  assert.match(gitignore, /^test-results\/\s*$/m);

  const source = harnessSource();
  assert.match(source, /test-results[\\/]performance-regression/i);
  assert.match(source, /Get-Date\b[\s\S]*yyyy|yyyy[\s\S]*Get-Date\b/i);
  assert.match(source, /run-\$?\(?scenario|run-\$?Scenario|run-\{scenario\}/i);
  assert.match(source, /summary\.md/i);
  assert.match(source, /residual-risk\.md/i);
});

test('performance artifact root timestamp cannot collide within the same second', () => {
  const source = harnessSource();

  assert.match(source, /Get-DateStamp|timestamp/i);
  assert.match(source,
    /(yyyyMMdd[-_]?HHmmss[^'"\)]*f{3,7})|(f{3,7}[^'"\)]*yyyyMMdd[-_]?HHmmss)|New-Guid|CreateDirectory\(|Test-Path[\s\S]{0,260}(while|do\s*\{)|while[\s\S]{0,260}Test-Path/i,
    'artifact root must use millisecond/unique timestamp or retry/fail when output dir already exists');
  assert.doesNotMatch(source,
    /function\s+Get-DateStamp\s*\{\s*\(Get-Date\)\.ToString\(['"]yyyyMMdd-HHmmss['"]\)\s*\}/i,
    'second-only timestamp collides for repeated harness starts in one second');
  assert.doesNotMatch(source,
    /New-Item\s+-ItemType\s+Directory\s+-Force\s+-Path\s+\$outputDir/i,
    'timestamp collision must not silently reuse an existing run directory');
});

test('launch mode contract separates release acceptance from dev diagnostics', () => {
  const source = harnessSource();

  assertMentions(source, ['release', 'dev'], 'launch-mode contract');
  assert.match(source, /release[\s\S]{0,240}acceptance|acceptance[\s\S]{0,240}release/i);
  assert.match(source, /dev[\s\S]{0,240}diagnostic|diagnostic[\s\S]{0,240}dev/i);
  assert.match(source, /ValidateSet\([^)]*release[^)]*dev|mode[^\n]*(release|dev)/i);
  assert.match(source, /dev[\s\S]{0,160}never counts as release acceptance/i,
    'explicit dev non-acceptance statement is allowed and required');
  assert.doesNotMatch(source, /dev(?:(?!never counts|not count|cannot count|can't count)[\s\S]){0,160}counts as release acceptance/i,
    'dev evidence must not affirmatively count as release acceptance');
});

test('scenario matrix is exactly the seven Plan 01 scenarios', () => {
  const source = harnessSource();

  assert.deepEqual(scenarioLiteralSet(source), REQUIRED_SCENARIOS);
  assert.match(source, /Assert-?RequiredScenarios|Validate-?ScenarioMatrix/i, 'must validate scenario names, not only count');
  assert.match(source, /Compare-Object|SequenceEqual|ForEach-Object[\s\S]{0,240}\$RequiredScenarios/i, 'must compare requested scenarios to required names');
  assert.doesNotMatch(source, /if\s*\(\$Scenarios\.Count\s*,\s*7\)\s*\{\s*\}/i, 'fake count anchor is not validation');
  assert.doesNotMatch(source, /\$Scenarios\.Count\s+-ne\s+7[\s\S]{0,180}exactly\s+7\s+named\s+scenarios/i, 'count-only validation accepts arbitrary seven names');
});

test('scenario validation rejects arbitrary seven-item overrides by exact equality', () => {
  const source = harnessSource();

  assert.match(source, /\$RequiredScenarios/i);
  assert.match(source, /throw[\s\S]{0,220}(unknown|unsupported|must match|required|Plan 01)/i);
  assert.doesNotMatch(source, /\[string\[\]\]\$Scenarios\s*=\s*@\([\s\S]*?\)[\s\S]{0,260}\$Scenarios\.Count\s+-ne\s+7[\s\S]{0,220}\{\s*throw/i,
    'override validation must reject wrong names, not only wrong count');
});

test('scenario validation rejects reordering, not membership-only set comparison', () => {
  const source = executableSource();

  assert.match(source, /Assert-?RequiredScenarios|Validate-?ScenarioMatrix/i);
  assert.match(source,
    /SequenceEqual|for\s*\([^\)]*\$i[\s\S]{0,320}\$Requested\s*\[\s*\$i\s*\][\s\S]{0,160}\$required\s*\[\s*\$i\s*\]|ForEach-Object[\s\S]{0,360}\$RequiredScenarios[\s\S]{0,240}\$Scenarios\s*\[|\$Requested\s+-ceq\s+\$required|\$Requested\s+-cne\s+\$required/i,
    'scenario matrix validation must compare ordered positions, so reordering fails');
  assert.doesNotMatch(source,
    /Compare-Object\s+-ReferenceObject\s+\$required\s+-DifferenceObject\s+\$Requested(?![\s\S]{0,220}(-SyncWindow|SequenceEqual|\$Requested\s*\[\s*\$i\s*\]))/i,
    'Compare-Object membership-only validation accepts reordered scenario arrays');
});

test('each scenario runs exactly three times', () => {
  const source = harnessSource();

  assert.match(source, /run[s]?\s*(?:per\s*scenario)?\s*=\s*3|1\.\.3|for\s*\([^\)]*3/i);
  assert.match(source, /foreach[\s\S]{0,320}scenario[\s\S]{0,320}(1\.\.3|run[s]?\s*(?:per\s*scenario)?\s*=\s*3)/i);
  assert.doesNotMatch(source, /run[s]?\s*(?:per\s*scenario)?\s*=\s*[124-9]\b/i);
});

test('per-run JSON schema includes mode, timestamps, scenario, status, metrics, and I/O availability', () => {
  const source = harnessSource();

  assertMentions(source, [
    'schema',
    'version',
    'mode',
    'startedAt',
    'finishedAt',
    'scenario',
    'metadata',
    'status',
    'cpu',
    'privateBytes',
    'workingSet',
    'threadCount',
    'handleCount',
    'controlIo',
    'available'
  ], 'per-run JSON schema');
  assert.match(source, /ConvertTo-Json/i);
  assert.match(source, /run-[^\n]*\.json/i);
});

test('run status transitions only after measurable scenario, with blocked and unavailable paths explicit', () => {
  const source = harnessSource();

  assert.match(source, /status\s*=\s*'running'|status\s*=\s*"running"/i, 'status should enter running before measurement');
  assert.match(source, /status\s*=\s*'pass'|status\s*=\s*"pass"/i, 'pass status must exist for measured success');
  assert.match(source, /status\s*=\s*'blocked'|status\s*=\s*"blocked"/i, 'blocked status must exist');
  assert.match(source, /status\s*=\s*'error'|status\s*=\s*"error"/i, 'error status must exist');
  assert.match(source, /prereq|prerequisite|unavailable/i, 'scenario prereq unavailable path must be modeled');
  assert.match(source, /if\s*\([^)]*\$metrics|if\s*\([^)]*processMetrics|\$record\.processMetrics[\s\S]{0,240}\$record\.scenario\.status\s*=\s*['"]pass/i,
    'success must be gated on captured measurable metrics');
  assert.doesNotMatch(source, /\$record\.scenario\.status\s*=\s*'blocked\/not measured'/i, 'single opaque blocked/not measured status hides status accounting');
});

test('all Plan 01 scenarios have real handler or prereq logic, not source anchors', () => {
  const source = harnessSource();

  for (const scenario of REQUIRED_SCENARIOS) {
    const escaped = escapeRegExp(scenario);
    const branchPattern = new RegExp(`(?:['"]${escaped}['"]\\s*\\{[\\s\\S]{0,260}(?:return|throw|status|reason|ok\\s*=))|(?:case\\s+['"]${escaped}['"][\\s\\S]{0,260}(?:return|throw|status|reason|ok\\s*=))`, 'i');
    assert.match(source, branchPattern,
      `${scenario} needs handler/prereq branch`);
  }
  assert.doesNotMatch(source, /pattern anchor|contract anchor/i, 'fake test anchors are forbidden');
});

test('release binary discovery is deterministic produced jason-shell.exe executable only', () => {
  const source = executableSource();

  assert.match(source,
    /Name\s+-ceq\s+['"]jason-shell\.exe['"]|\.Name\s+-ceq\s+['"]jason-shell\.exe['"]|jason-shell\.exe/,
    'release discovery must target exact produced binary name jason-shell.exe');
  assert.doesNotMatch(source, /JasonShell\.exe/i,
    'release discovery must not target legacy/cased JasonShell.exe');
  assert.doesNotMatch(source, /BaseName\s+-[ic]?eq\s+['"]JasonShell['"]/i,
    'release discovery must not accept BaseName match; only jason-shell.exe file name is valid');
  assert.doesNotMatch(source, /bundle|sidecar/i,
    'release discovery must not scan bundle/sidecar paths for acceptance launcher');
  assert.match(source, /src-tauri[\\/]target[\\/]release|target[\\/]release/i,
    'release target path contract must explicitly point at target/release executable output');
  assert.match(source, /Sort-Object/i, 'candidate choice must be deterministic');
  assert.doesNotMatch(source, /Select-Object\s+-First\s+1(?![\s\S]{0,160}Sort-Object)/i, 'must not take arbitrary filesystem first result');
  assert.doesNotMatch(source, /\.Extension\s+-in\s+@\(['"]\.exe['"]\s*,\s*['"]\.bat['"]\s*,\s*['"]\.cmd['"]\)/i, 'release launcher must not accept batch/cmd helpers');
  assert.match(source, /reject|throw[\s\S]{0,220}(msi|msix|setup|installer|bundle|package|helper)/i,
    'must reject installer/package/helper artifacts');
});

test('release binary discovery dynamically finds produced src-tauri target release jason-shell.exe', () => {
  const source = executableSource();

  assert.match(source,
    /\$\w*(?:roots|releaseRoots)\w*\s*=\s*@\([\s\S]{0,900}(src-tauri[\\/]target[\\/]release|target[\\/]release)[\s\S]{0,900}(src-tauri[\\/]target[\\/]release|target[\\/]release)[\s\S]{0,900}\)/i,
    'release harness must declare allowed release output roots in executable code');
  assert.match(source,
    /Get-ChildItem\b[^\r\n|;]*\s-Filter\s+['"]jason-shell\.exe['"]/,
    'dynamic discovery must call Get-ChildItem with exact -Filter jason-shell.exe');
  assert.match(source,
    /Get-ReleaseBinaryCandidates[\s\S]{0,180}Sort-Object\s+FullName[\s\S]{0,80}\[0\]/,
    'dynamic discovery must deterministically select the exact sorted discovery result');
  assert.doesNotMatch(source,
    /(?:-Filter\s+['"]JasonShell\.exe['"]|-Include\s+['"]JasonShell\.exe['"]|\.Name\s+-[ic]?eq\s+['"]JasonShell\.exe['"])/i,
    'dynamic discovery must not accept JasonShell.exe casing/name');
});

test('explicit release binary path must resolve under allowed release roots before return', () => {
  const source = executableSource();

  assert.match(source, /ReleaseBinaryPath/i, 'release harness must expose -ReleaseBinaryPath for explicit release executable selection');
  assert.match(source,
    /ReleaseBinaryPath[\s\S]{0,1200}(Name|Leaf|Split-Path)[\s\S]{0,360}(?:-ceq\s+['"]jason-shell\.exe['"]|jason-shell\.exe)/,
    'explicit override must be accepted only when final file name is exact jason-shell.exe');
  assert.doesNotMatch(source,
    /ReleaseBinaryPath[\s\S]{0,1200}(?:-ieq\s+['"]jason-shell\.exe['"]|-eq\s+['"]jason-shell\.exe['"]|JasonShell\.exe)/i,
    'explicit override guard must not allow case-insensitive jason-shell.exe or legacy JasonShell.exe');
  assert.match(source, /src-tauri[\\/]target[\\/]release/i, 'allowed release roots must include src-tauri/target/release');
  assert.match(source, /(?:^|[^a-z])target[\\/]release/i, 'allowed release roots must include target/release');
  assert.match(source,
    /(src-tauri[\\/]target[\\/]release|target[\\/]release)[\s\S]{0,1200}Test-Path[\s\S]{0,360}Resolve-Path/i,
    'allowed release roots must be declared, existence-checked, then resolved to canonical paths');
  assert.match(source,
    /Resolve-Path[\s\S]{0,1200}(FullName|ProviderPath|Path)[\s\S]{0,1200}(StartsWith|Relative|IsSubPath|GetRelativePath|Compare|TrimEnd)/i,
    'explicit -ReleaseBinaryPath must be canonicalized and compared with allowed release roots before acceptance');
  assert.match(source,
    /(throw|reject)[\s\S]{0,360}(outside|under|allowed|release root|target[\\/]release)/i,
    'external JasonShell.exe paths must be rejected when outside allowed release roots');
  assert.doesNotMatch(source,
    /if\s*\([^)]*\$ReleaseBinaryPath[^)]*\)\s*\{[\s\S]{0,420}(return\s+\$ReleaseBinaryPath|return\s+\$resolved|return\s+\$candidate)(?![\s\S]{0,420}(StartsWith|Relative|IsSubPath|GetRelativePath|Compare|allowed|release root))/i,
    '-ReleaseBinaryPath must not return arbitrary external JasonShell.exe without allowed-root validation');
});

test('release root canonicalization tolerates absent target release roots', () => {
  const source = executableSource();

  assert.match(source,
    /\$\w*(?:roots|releaseRoots)\w*\s*=\s*@\([\s\S]{0,900}(src-tauri[\\/]target[\\/]release|target[\\/]release)[\s\S]{0,900}(src-tauri[\\/]target[\\/]release|target[\\/]release)[\s\S]{0,900}\)/i,
    'release harness must keep both allowed roots: target/release and src-tauri/target/release');
  assert.match(source,
    /Test-Path[\s\S]{0,360}Resolve-Path|(?:Where-Object|foreach|ForEach-Object)[\s\S]{0,360}Test-Path[\s\S]{0,360}Resolve-Path/i,
    'allowed release roots must be filtered by Test-Path before Resolve-Path so an absent root is tolerated');
  assert.doesNotMatch(source,
    /Resolve-Path\s+(?:-LiteralPath\s+|-Path\s+)?\$\w*(?:roots|releaseRoots)\w*/i,
    'must not Resolve-Path the allowed-root array unconditionally; absent target/release must not fail when src-tauri/target/release exists');
});

test('timeout parameter is executable and used around scenario launch', () => {
  const source = executableSource();

  assert.match(source, /ScenarioTimeoutSeconds/i);
  assert.match(source, /deadline|AddSeconds\(\s*\$ScenarioTimeoutSeconds\s*\)/i,
    'timeout must create an executable deadline from ScenarioTimeoutSeconds');
  assert.match(source,
    /\(Get-Date\)\s+-ge\s+\$\w*deadline\b[\s\S]{0,220}(status\s*=\s*['"]error|status\s*=\s*['"]blocked|throw[\s\S]{0,80}timeout)|timeout[\s\S]{0,220}(status\s*=\s*['"]error|status\s*=\s*['"]blocked|throw)/i,
    'deadline expiry must produce explicit timeout/error status');
  assert.match(source, /finally[\s\S]{0,640}(Stop-ProcessTree|taskkill\s+\/T|Kill|Stop-Process)/i,
    'timeout and normal exits must terminate owned process tree during cleanup');
});

test('timeout enforcement has no unreachable anchors or Start-Job pid leak', () => {
  const source = executableSource();

  assert.doesNotMatch(source, /if\s*\(\s*\$false\s*\)/i,
    'unreachable if ($false) anchors are forbidden; contracts must match executable code');
  assert.doesNotMatch(source, /Start-Job/i,
    'timeout enforcement should avoid Start-Job; synchronous process/deadline control is preferred');
  assert.doesNotMatch(source, /param\s*\([^\)]*\$pid\b/i,
    'Start-Job scriptblocks must not shadow/leak PowerShell automatic $PID');
  assert.match(source,
    /deadline|ScenarioTimeoutSeconds[\s\S]{0,320}(Stop-ProcessTree|taskkill\s+\/T|Kill|Stop-Process)/i,
    'real timeout path must enforce deadline and terminate owned process tree');
});

test('CPU metric is interval delta, not single cumulative snapshot', () => {
  const source = executableSource();

  assert.match(source, /Start-Sleep[\s\S]{0,220}(cpu|TotalProcessorTime|\.CPU)[\s\S]{0,220}(cpu|TotalProcessorTime|\.CPU)|Measure-ProcessCpuDelta|Get-ProcessMetricsInterval/i,
    'CPU must be sampled across an interval');
  assert.match(source, /TotalProcessorTime|\.CPU/i);
  assert.match(source, /delta|elapsed|duration|sample/i);
  assert.doesNotMatch(source, /cpu\s*=\s*\$Process\.CPU\b/i, 'single cumulative CPU snapshot is not a scenario CPU measurement');
});

test('runner samples CPU from live process before waiting for exit, then cleans up process tree', () => {
  const source = executableSource();

  assert.match(source, /Start-Process/i, 'runner must launch scenario process');
  assert.match(source, /Wait-ProcessReadyWithinDeadline|readiness|ready|settle/i,
    'runner must wait for bounded readiness/settle before sampling');
  assert.match(source, /if\s*\([^)]*\.HasExited[^)]*\)[\s\S]{0,180}(process exited before live sampling|status\s*=\s*['"]error|throw)/i,
    'runner must verify process is still live before CPU sampling');
  assert.match(source, /Get-ProcessCpuDelta|TotalProcessorTime|Measure-ProcessCpuDelta/i,
    'runner must take interval CPU sample from live process');
  assert.match(source, /finally[\s\S]{0,640}(Stop-ProcessTree|taskkill\s+\/T|Get-CimInstance\s+Win32_Process)/i,
    'runner must cleanup process tree after live-process sampling');
  assert.doesNotMatch(source, /Start-Process[\s\S]{0,640}Wait-Process[\s\S]{0,640}(Get-ProcessCpuDelta|TotalProcessorTime|Measure-ProcessCpuDelta)/i,
    'Wait-Process before CPU interval sampling waits for exit, so sampled process is no longer live');
});

test('readiness settle loop exits while app is alive before deadline instead of waiting alive until deadline', () => {
  const source = executableSource();

  assert.match(source, /ready|readiness|settle/i, 'runner must model readiness/settle before sampling');
  assert.match(source, /deadline|ScenarioTimeoutSeconds/i, 'readiness/settle must be bounded by deadline');
  assert.match(source, /break|return\s+\$true|ready\s*=\s*\$true|settled\s*=\s*\$true/i,
    'readiness/settle loop must have success exit before deadline while process remains alive');
  assert.doesNotMatch(source,
    /while\s*\(\s*-not\s+\$\w+\.HasExited\s+-and\s*\(Get-Date\)\s+-lt\s*\$\w+\s*\)\s*\{\s*Start-Sleep[\s\S]{0,240}\}\s*if\s*\(\s*\$\w+\.HasExited\s*\)[\s\S]{0,160}if\s*\(\s*\(Get-Date\)\s+-ge\s*\$\w+\s*\)/i,
    'loop that only waits for exit/deadline treats alive app as timeout instead of readiness success');
});

test('CPU schema exposes explicit units and rate fields', () => {
  const source = executableSource();

  assert.match(source, /cpuDeltaMs/i, 'CPU schema must expose CPU delta unit in milliseconds');
  assert.match(source, /sampleElapsedMs/i, 'CPU schema must expose sample elapsed unit in milliseconds');
  assert.match(source, /cpuPercent/i, 'CPU schema must expose derived CPU rate percent');
  assert.doesNotMatch(source, /processMetrics\s*=\s*@\{[\s\S]{0,240}cpu\s*=\s*\$null/i,
    'ambiguous cpu field lacks units/rate contract');
});

test('dev launch and cleanup cannot orphan child process trees', () => {
  const source = executableSource();

  assert.match(source, /Stop-ProcessTree|taskkill\s+\/T|Get-CimInstance\s+Win32_Process|Get-ChildProcess/i,
    'cleanup must include process tree, not only parent pid');
  assert.match(source,
    /Start-Process\s+-FilePath\s+['"]npm['"][\s\S]{0,320}-PassThru[\s\S]{0,900}(Get-ProcessTreeProcessIds|Get-CimInstance\s+Win32_Process|Stop-ProcessTree|taskkill\s+\/T)/i,
    'npm dev launch must be paired with child-tree discovery or cleanup');
  assert.doesNotMatch(source,
    /finally\s*\{\s*if\s*\(\s*\$proc\s+-and\s+-not\s+\$proc\.HasExited\s*\)\s*\{\s*Stop-Process\s+-Id\s+\$proc\.Id\s*/i,
    'cleanup must not only stop live npm parent pid');
});

test('dev cleanup kills spawned child tree even when npm parent exits first', () => {
  const source = executableSource();

  assert.match(source, /Start-Process\s+-FilePath\s+['"]npm['"][\s\S]{0,260}-ArgumentList\s+@\([^)]*['"]run['"][^)]*['"]dev['"][^)]*\)[\s\S]{0,260}-PassThru|Start-Process\s+-FilePath\s+['"]npm['"][\s\S]{0,260}-PassThru[\s\S]{0,260}-ArgumentList\s+@\([^)]*['"]run['"][^)]*['"]dev['"][^)]*\)/i,
    'dev diagnostics launch through npm parent command');
  assert.match(source, /Get-CimInstance\s+Win32_Process[\s\S]{0,260}ParentProcessId=\$\(\$proc\.Id\)|Stop-ProcessTree/i,
    'dev launch must discover/remember child tree rooted at npm');
  assert.doesNotMatch(source,
    /finally\s*\{\s*if\s*\(\s*\$proc\s+-and\s+-not\s+\$proc\.HasExited\s*\)\s*\{\s*Stop-ProcessTree\s+-RootProcessId\s+\$proc\.Id\s*\}\s*\}/i,
    'cleanup gated on npm parent liveness misses children after npm exits');
  assert.match(source,
    /finally[\s\S]{0,520}(\$devChildProcessIds|childProcessIds|Get-CimInstance\s+Win32_Process|Stop-ProcessTree|taskkill\s+\/T)/i,
    'finally cleanup must cover captured/discovered child tree, not only live npm parent');
});

test('median summary separates release acceptance and dev diagnostics', () => {
  const source = harnessSource();

  assert.match(source, /median/i);
  assertMentions(source, ['cpu', 'privateBytes', 'workingSet', 'threadCount', 'handleCount', 'controlIo'], 'median summary');
  assert.match(source, /release[\s\S]{0,240}acceptance/i);
  assert.match(source, /dev[\s\S]{0,240}diagnostic/i);
  assert.match(source, /summary\.md/i);
});

test('summary exposes pass blocked error counts and denies release acceptance unless all release scenarios measured', () => {
  const source = harnessSource();

  assert.match(source, /pass(?:ed)?Count|counts?\.pass|statusCounts/i);
  assert.match(source, /blockedCount|counts?\.blocked|statusCounts/i);
  assert.match(source, /errorCount|counts?\.error|statusCounts/i);
  assert.match(source, /release[\s\S]{0,360}(all[\s-]?measured|blockedCount\s*-eq\s*0|errorCount\s*-eq\s*0|cannot claim|not accepted)/i,
    'release acceptance must require all measured, no blocked/errors');
  assert.doesNotMatch(source, /Release acceptance:\s*\$\(\[bool\]\(\$Mode\s+-eq\s+'release'\)\)/i,
    'summary cannot claim release acceptance from mode alone');
});

test('budgets are measured-baseline-relative and contain no fabricated numeric thresholds', () => {
  const source = harnessSource();

  assert.match(source, /measured[- ]baseline[- ]relative|baseline[\s\S]{0,120}non[- ]regression|non[- ]regression[\s\S]{0,120}baseline/i);
  assert.doesNotMatch(source, /cpu[^\n]*(?:<=|lt|-le)\s*\d+\s*%/i);
  assert.doesNotMatch(source, /privateBytes[^\n]*(?:<=|lt|-le)\s*\d+/i);
  assert.doesNotMatch(source, /workingSet[^\n]*(?:<=|lt|-le)\s*\d+/i);
  assert.doesNotMatch(source, /threadCount[^\n]*(?:<=|lt|-le)\s*\d+/i);
  assert.doesNotMatch(source, /handleCount[^\n]*(?:<=|lt|-le)\s*\d+/i);
});

test('residual risk and manual prereq output supports blocked/not measured scenarios', () => {
  const source = harnessSource();

  assert.match(source, /residual-risk\.md/i);
  assert.match(source, /blocked/i);
  assert.match(source, /not measured/i);
  assertMentions(source, ['notifications', 'fullscreen', 'multi-monitor'], 'manual prereq output');
  assert.match(source, /prereq|manual|unavailable/i);
});

test('manual and complex scenarios are confirmable or concretely detectable, not permanently false', () => {
  const source = executableSource();
  const manualScenarios = REQUIRED_SCENARIOS.filter((scenario) => scenario !== 'cold-idle');

  assert.match(source, /NonInteractive/i, 'runner must expose NonInteractive mode for manual/complex scenarios');
  assert.match(source, /Read-Host|PromptForChoice|ShouldContinue|operator|confirm/i,
    'interactive runner must support operator confirmation when automation cannot prove prereq');
  assert.match(source, /trigger|probe|detect|Test-|Get-|Invoke-/i,
    'runner must support concrete detection/trigger hooks where available');
  assert.match(source, /NonInteractive[\s\S]{0,320}(blocked|return\s+@\{[\s\S]{0,120}ok\s*=\s*\$false)/i,
    'NonInteractive must accurately emit blocked rather than fake pass');

  for (const scenario of manualScenarios) {
    const escaped = escapeRegExp(scenario);
    const permanentFalse = new RegExp(`['"]${escaped}['"]\\s*\\{\\s*return\\s+@\\{(?:(?!\\}).)*ok\\s*=\\s*\\$false(?:(?!\\}).)*\\}\\s*\\}`, 'i');
    assert.doesNotMatch(source, permanentFalse,
      `${scenario} cannot be permanently blocked with ok=$false only; needs interactive confirmation or concrete detection/trigger`);
  }
});
