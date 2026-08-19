import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const source = readFileSync(new URL('../scripts/smoke-taskbar-attention.ps1', import.meta.url), 'utf8');

test('taskbar smoke script exposes deterministic exe, env, and latency contracts', () => {
  assert.match(source, /param\(\s*\[string\]\$ExePath/);
  assert.match(source, /\[ValidateSet\('auto', 'manual-matrix'\)\]/);
  assert.match(source, /Resolve-JasonShellExe/);
  assert.match(source, /jason-shell\.exe/);
  assert.match(source, /JASONSHELL_TASKBAR_NATIVE_HOOKS/);
  assert.doesNotMatch(source, /JASONSHELL_TASKBAR_NATIVE_HOOKS_V2/);
  assert.match(source, /visible/);
  assert.match(source, /minimized/);
  assert.match(source, /manual-matrix/);
  assert.match(source, /LatencyMs/);
  assert.match(source, /250ms bound/);
  assert.match(source, /ReceiverHwnd/);
  assert.match(source, /RequestAt/);
  assert.match(source, /DetectedAt/);
  assert.match(source, /ForegroundAt/);
  assert.match(source, /ClearAt/);
  assert.match(source, /Remove-Item -LiteralPath \$p -Force/);
  assert.doesNotMatch(source, /--fixture-case|--jsonl|JASONSHELL_TASKBAR_ATTENTION_FIXTURE|JASONSHELL_TASKBAR_ATTENTION_HOOKS/);
  assert.doesNotMatch(source, /Start-Process\s+cmd|Invoke-Expression|powershell -c|Set-ExecutionPolicy/);
});

test('taskbar smoke script keeps bounded process launch and cleanup semantics', () => {
  assert.match(source, /WaitForExit\(\$Timeout\)/);
  assert.match(source, /Kill\(\$true\)/);
  assert.match(source, /CreateNoWindow = \$true/);
  assert.match(source, /UseShellExecute = \$false/);
  assert.match(source, /RedirectStandardOutput = \$true/);
  assert.match(source, /RedirectStandardError = \$true/);
  assert.match(source, /ReadLineAsync\(\)/);
  assert.doesNotMatch(source, /add_OutputDataReceived|add_ErrorDataReceived/);
  assert.match(source, /Set-Content -LiteralPath \$StdoutPath/);
  assert.match(source, /ConvertFrom-Json -AsHashtable/);
  assert.match(source, /if \(\$failures\.Count -gt 0\) \{ 1 \} else \{ 0 \}/);
});
