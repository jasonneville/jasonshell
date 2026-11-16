param(
  [string]$Url = 'http://127.0.0.1:4196',
  [string]$Session = 'search-scope',
  [string]$OutputDirectory = 'C:/Users/jnev1/AppData/Local/Temp/opencode/search-scope-rounded-popup-runtime'
)
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force $OutputDirectory | Out-Null
function Browser([Parameter(ValueFromRemainingArguments=$true)][string[]]$Words) {
  $result = & agent-browser --session $Session @Words
  if ($LASTEXITCODE -ne 0) { throw "agent-browser failed: $($Words -join ' ')`n$result" }
  return $result
}
function Evaluate([string]$Script) {
  $encoded = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($Script))
  return (Browser @('eval', '-b', $encoded) | Out-String | ConvertFrom-Json)
}
Browser @('open', $Url) | Out-Null
Browser @('wait', '--fn', 'window.searchScopeReady === true') | Out-Null
Browser @('set', 'viewport', '320', '620') | Out-Null
Evaluate @'
(async()=>{
 await window.searchScopeFixture.refresh(); await window.searchScopeFixture.scope('All');
 window.searchScopeFixture.ledger.length=0;
 window.scopeTrustedEvents=[]; window.scopeTrustedStates=[];
 for(const type of ['keydown','click','pointerdown','input']) document.addEventListener(type,e=>window.scopeTrustedEvents.push({type:e.type,key:e.key,trusted:e.isTrusted,target:e.target.getAttribute?.('aria-label')||e.target.getAttribute?.('role')||e.target.tagName}));
 window.recordScopeState=(stage)=>{
   const t=document.querySelector('button[role=combobox]'),p=document.getElementById(t.getAttribute('aria-controls'));
   const s={stage,open:t.getAttribute('aria-expanded')==='true',inert:p.inert,inertAttribute:p.getAttribute('inert'),popoverOpen:p.matches(':popover-open'),focus:document.activeElement===t?'trigger':document.activeElement===p?'popup':document.activeElement.getAttribute('aria-label'),label:t.firstElementChild.textContent,rows:document.querySelectorAll('#search-results [role=option]').length};
   window.scopeTrustedStates.push(s); return s;
 }; return true;
})()
'@ | Out-Null
$failures = [Collections.Generic.List[string]]::new()
function Record([string]$Stage, [scriptblock]$Accept) {
  $state = Evaluate "window.recordScopeState('$Stage')"
  if (-not (& $Accept $state)) { $failures.Add("$Stage failed: $($state | ConvertTo-Json -Compress)") }
}
$trigger = 'button[role=combobox]'
$lensPoint = Evaluate "(()=>{const r=document.querySelector('.search-panel-lens').getBoundingClientRect();return {x:r.left+r.width/2,y:r.top+r.height/2};})()"
Browser @('mouse', 'move', [string][Math]::Round($lensPoint.x), [string][Math]::Round($lensPoint.y)) | Out-Null
Browser @('mouse', 'down') | Out-Null
Browser @('mouse', 'up') | Out-Null
Record 'lens pointer opens combined capsule popup' { param($s) $s.open -and $s.popoverOpen -and $s.focus -eq 'popup' }
Browser @('press', 'Escape') | Out-Null
Browser @('click', $trigger) | Out-Null
Record 'pointer opens real non-inert popup' { param($s) $s.open -and $s.popoverOpen -and -not $s.inert -and $s.focus -eq 'popup' }
Browser @('press', 'Escape') | Out-Null
Record 'Escape closes popup restores trigger' { param($s) -not $s.open -and $s.focus -eq 'trigger' }
Browser @('press', 'Enter') | Out-Null
Record 'Enter opens' { param($s) $s.open -and $s.focus -eq 'popup' }
Browser @('press', 'Home') | Out-Null
Browser @('press', 'ArrowDown') | Out-Null
Browser @('press', 'Enter') | Out-Null
Record 'keyboard Apps selection' { param($s) -not $s.open -and $s.label -eq 'Apps' -and $s.rows -eq 2 -and $s.focus -eq 'trigger' }
Browser @('press', 'Space') | Out-Null
Record 'Space opens' { param($s) $s.open -and $s.focus -eq 'popup' }
Browser @('press', 'End') | Out-Null
Browser @('press', 'Enter') | Out-Null
Record 'End selects Everything' { param($s) -not $s.open -and $s.label -eq 'Everything' -and $s.rows -eq 1 }
Browser @('press', 'ArrowDown') | Out-Null
Record 'ArrowDown opens' { param($s) $s.open -and $s.focus -eq 'popup' }
Browser @('press', 'Home') | Out-Null
Browser @('press', 'w') | Out-Null
Browser @('press', 'Enter') | Out-Null
Record 'typeahead selects Windows' { param($s) -not $s.open -and $s.label -eq 'Windows' -and $s.rows -eq 1 }
Browser @('press', 'ArrowDown') | Out-Null
Evaluate "window.searchScopeFixture.refresh({query:'trusted refresh while open'})" | Out-Null
Record 'refresh preserves open popup focus' { param($s) $s.open -and $s.focus -eq 'popup' }
Browser @('press', 'Tab') | Out-Null
Record 'Tab closes and advances to query' { param($s) -not $s.open -and $s.focus -eq 'Search Everything' }
Browser @('focus', $trigger) | Out-Null
Browser @('press', 'ArrowDown') | Out-Null
Browser @('click', 'input[role=combobox]') | Out-Null
Record 'outside click closes and preserves query focus' { param($s) -not $s.open -and $s.focus -eq 'Search Everything' }
Evaluate 'window.searchScopeFixture.mountSharedControls()' | Out-Null
$sharedStates = [Collections.Generic.List[object]]::new()
function SharedRecord([string]$Stage, [string]$ExpectedFocus) {
  $state = Evaluate "({stage:'$Stage',focus:document.activeElement.getAttribute('aria-label')})"
  $sharedStates.Add($state)
  if ($state.focus -ne $ExpectedFocus) { $failures.Add("$Stage expected focus '$ExpectedFocus', received '$($state.focus)'") }
}
Browser @('click', "button[aria-label='Default focus control']") | Out-Null
$outsideHit = Evaluate "(()=>{const e=document.querySelector('input[aria-label]'),r=e.getBoundingClientRect();return document.elementFromPoint(r.left+r.width/2,r.top+r.height/2)===e;})()"
if (-not $outsideHit) { throw 'Isolated outside focus target is covered; cannot claim outside-click evidence' }
Browser @('click', "input[aria-label='Shared focus outside']") | Out-Null
SharedRecord 'shared default restores trigger on outside dismissal' 'Default focus control'
Browser @('click', "button[aria-label='Opt-out focus control']") | Out-Null
$outsideHit = Evaluate "(()=>{const e=document.querySelector('input[aria-label]'),r=e.getBoundingClientRect();return document.elementFromPoint(r.left+r.width/2,r.top+r.height/2)===e;})()"
if (-not $outsideHit) { throw 'Isolated outside focus target is covered; cannot claim outside-click evidence' }
Browser @('click', "input[aria-label='Shared focus outside']") | Out-Null
SharedRecord 'shared optout preserves outside target focus' 'Shared focus outside'
Browser @('click', "button[aria-label='Opt-out focus control']") | Out-Null
Browser @('press', 'Escape') | Out-Null
SharedRecord 'shared optout Escape still restores trigger' 'Opt-out focus control'
Browser @('press', 'ArrowDown') | Out-Null
Browser @('press', 'Home') | Out-Null
Browser @('press', 'ArrowDown') | Out-Null
Browser @('press', 'Enter') | Out-Null
SharedRecord 'shared optout selection still restores trigger' 'Opt-out focus control'
Evaluate 'window.searchScopeFixture.removeSharedControls()' | Out-Null
$report = Evaluate '({states:window.scopeTrustedStates,events:window.scopeTrustedEvents,ledger:window.searchScopeFixture.ledger})'
if ($report.ledger | Where-Object { $_.name -in @('search-panel:query','hide_search_panel') -or ($_.name -eq 'search-panel:key' -and $_.payload -eq 'Escape') }) { $failures.Add('Scope-only trusted journey emitted backend query or search close') }
$result = @{ failures = @($failures); evidence = $report; sharedFocusStates = @($sharedStates); boundary = 'Trusted CDP pointer/keyboard events on production mounted renderer with mocked IPC; fixture refresh/setup programmatic; not native shell.' }
$result | ConvertTo-Json -Depth 30 | Set-Content (Join-Path $OutputDirectory 'trusted-journey.json')
$result | ConvertTo-Json -Depth 30
if ($failures.Count) { exit 1 }
