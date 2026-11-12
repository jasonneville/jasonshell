# Run against the official, already-running, visible wmux portable runtime.
# No browser is launched by this script; no provider/shell/application startup.
param(
  [Parameter(Mandatory=$true)][int]$WmuxProcessId,
  [string]$WmuxRoot=(Join-Path $env:LOCALAPPDATA 'Temp/opencode/wmux-portable-2.60.0'),
  [string]$EvidenceDirectory=(Join-Path $env:LOCALAPPDATA 'Temp/opencode/system-info-visible-e2e'),
  [string]$Url='http://127.0.0.1:4181'
)
$ErrorActionPreference='Stop'
$cli=Join-Path $WmuxRoot 'runtime/resources/cli/wmux.js'
$env:APPDATA=Join-Path $WmuxRoot 'isolated-home/AppData/Roaming'
$env:WMUX_PIPE_TOKEN=$null # CLI privately reads the isolated instance token file.
New-Item -ItemType Directory -Path $EvidenceDirectory -Force|Out-Null
Add-Type -TypeDefinition 'using System;using System.Runtime.InteropServices;public static class WmuxOwnedVisibility{[DllImport("user32.dll")]public static extern bool ShowWindow(IntPtr h,int c);[DllImport("user32.dll")]public static extern bool SetForegroundWindow(IntPtr h);[DllImport("user32.dll")]public static extern bool IsIconic(IntPtr h);[DllImport("user32.dll")]public static extern bool IsWindowVisible(IntPtr h);}'
function Restore-OwnedWmux {
  $owned=Get-Process -Id $WmuxProcessId
  if($owned.Path -ne (Join-Path $WmuxRoot 'runtime/wmux.exe')){throw 'Not the test-owned portable wmux process'}
  [WmuxOwnedVisibility]::ShowWindow($owned.MainWindowHandle,9)|Out-Null
  [WmuxOwnedVisibility]::SetForegroundWindow($owned.MainWindowHandle)|Out-Null
  if([WmuxOwnedVisibility]::IsIconic($owned.MainWindowHandle) -or ![WmuxOwnedVisibility]::IsWindowVisible($owned.MainWindowHandle)){throw 'wmux must be visible and not minimized for this journey'}
}
function Wmux([string[]]$Arguments) {
  $text=& node $cli @Arguments
  if($LASTEXITCODE -ne 0){throw "wmux command failed: $($Arguments[0..1] -join ' ')"}
  return ($text -join "`n"|ConvertFrom-Json)
}
function Capture([string]$Name) {
  Restore-OwnedWmux
  Wmux @('browser','eval','(async()=>{await new Promise((resolve,reject)=>{const timeout=setTimeout(()=>reject(new Error("visible browser produced no animation frame")),1500);requestAnimationFrame(()=>requestAnimationFrame(()=>{clearTimeout(timeout);resolve();}));});return {visible:document.visibilityState};})()')|Out-Null
  $image=Wmux @('browser','screenshot')
  if(!$image.data){throw 'wmux returned no screenshot'}
  $bytes=[Convert]::FromBase64String($image.data)
  if($bytes.Length -lt 1000){throw 'empty screenshot is not evidence'}
  $path=Join-Path $EvidenceDirectory $Name
  [IO.File]::WriteAllBytes($path,$bytes)
  return $path
}
Restore-OwnedWmux
Wmux @('browser','open',$Url)|Out-Null
$snapshot=Wmux @('browser','snapshot')
$ref=[regex]::Match($snapshot.tree,'(@e\d+): button "Run acceptance journey"').Groups[1].Value
if(!$ref){throw 'basic journey control missing'}
Wmux @('browser','click',$ref)|Out-Null
$basic=Wmux @('browser','eval','(async()=>{const end=performance.now()+9000;while(performance.now()<end){const result=document.querySelector("#result").textContent;if(/^(PASS|FAIL):/.test(result))return {result};await new Promise(r=>setTimeout(r,30));}throw new Error("basic journey deadline");})()')
if($basic.result.result -notmatch '^PASS:'){throw $basic.result.result}
$basic.result|ConvertTo-Json -Depth 20|Set-Content (Join-Path $EvidenceDirectory 'basic-result.json')
Wmux @('browser','open',$Url)|Out-Null
$snapshot=Wmux @('browser','snapshot')
$ref=[regex]::Match($snapshot.tree,'(@e\d+): button "Run extended lifecycle journey"').Groups[1].Value
if(!$ref){throw 'extended journey control missing'}
# wmux type clicks the button and sends a real CDP key event. That establishes
# keyboard modality; synthetic navigation events inside the fixture then test
# the production handlers without falsely claiming physical-key delivery.
Wmux @('browser','type',$ref,'x')|Out-Null
$proof=Wmux @('browser','eval','(async()=>{const end=performance.now()+9000;while(performance.now()<end){const evidence=window.__systemInfoEvidence;if(["PASS","FAIL"].includes(evidence.status))return evidence;await new Promise(r=>setTimeout(r,30));}throw new Error("extended journey deadline");})()')
$proof.result|ConvertTo-Json -Depth 40|Set-Content (Join-Path $EvidenceDirectory 'extended-result-and-ipc.json')
if($proof.result.status -ne 'PASS'){throw (ConvertTo-Json $proof.result -Depth 5)}
$screenshots=@(Capture 'system-info-narrow-overview.png')
foreach($section in @('storage','sensors','network','hardware')) {
  $js='(()=>{const region=document.querySelector("[aria-label=''System information details'']");const heading=document.querySelector("#system-'+$section+'-title");region.scrollTop+=heading.getBoundingClientRect().top-region.getBoundingClientRect().top-8;return {section:"'+$section+'",scrollTop:region.scrollTop};})()'
  Wmux @('browser','eval',$js)|Out-Null
  $screenshots+=Capture ('system-info-narrow-'+$section+'.png')
}
$snapshot=Wmux @('browser','snapshot')
$snapshot|ConvertTo-Json -Depth 10|Set-Content (Join-Path $EvidenceDirectory 'system-accessibility-snapshot.json')
$ref=[regex]::Match($snapshot.tree,'(@e\d+): tab "Processes"').Groups[1].Value
Wmux @('browser','click',$ref)|Out-Null
$screenshots+=Capture 'processes-targeted.png'
[pscustomobject]@{status='PASS';basic=$basic.result.result;checks=$proof.result.checks.Count;narrow=$proof.result.measurements.narrow;screenshots=$screenshots;ipcEvidence=(Join-Path $EvidenceDirectory 'extended-result-and-ipc.json')}|ConvertTo-Json -Depth 10
