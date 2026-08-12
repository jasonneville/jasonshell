import { readFileSync } from 'node:fs';
import test from 'node:test';
import assert from 'node:assert/strict';

const script = readFileSync(new URL('../scripts/bootstrap-windows.ps1', import.meta.url), 'utf8');
const readme = readFileSync(new URL('../README.md', import.meta.url), 'utf8');

test('windows bootstrap script installs required prerequisites and launches tauri dev', () => {
  assert.match(script, /\$\{env:ProgramFiles\(x86\)\}/);
  assert.match(script, /New-TemporaryFile/);
  assert.match(script, /try\s*\{/);
  assert.match(script, /finally\s*\{/);
  assert.match(script, /F3017226-FE2A-4295-8BDF-00C3A9C2BB97/);
  assert.match(script, /HKLM:\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients/);
  assert.match(script, /HKCU:\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients/);
  assert.match(script, /Get-Command rustup/i);
  assert.match(script, /Get-Command rustc/i);
  assert.match(script, /Get-Command cargo/i);
  assert.match(script, /Get-Command node/i);
  assert.match(script, /Get-Command npm/i);
  assert.match(script, /Get-VsDevCmdPath/i);
  assert.match(script, /Test-WebView2RuntimeInstalled/i);
  assert.match(script, /Test-AdminRights/i);
  assert.match(script, /winget missing/i);
  assert.match(script, /Rustlang\.Rustup/);
  assert.match(script, /OpenJS\.NodeJS\.LTS/);
  assert.match(script, /Microsoft\.VisualStudio\.2022\.BuildTools/);
  assert.match(script, /Microsoft\.EdgeWebView2Runtime/);
  assert.match(script, /stable-msvc/);
  assert.match(script, /VsDevCmd\.bat/);
  assert.match(script, /vswhere/i);
  assert.match(script, /'"--wait --quiet --norestart --add Microsoft\.VisualStudio\.Workload\.VCTools --includeRecommended --add Microsoft\.VisualStudio\.Component\.Windows10SDK\.19041"'/);
  assert.match(script, /Admin rights required/i);
  assert.match(script, /reboot/i);
  assert.match(script, /if \(-not \$SkipInstall\) \{/);
  assert.match(script, /npm ci/);
  assert.match(script, /npm run tauri dev/);
});

test('readme documents bootstrap script and lockfile-safe install path', () => {
  assert.match(readme, /bootstrap-windows\.ps1/i);
  assert.match(readme, /bootstrap rather than `npm install`/i);
  assert.match(readme, /npm run tauri dev/);
});
