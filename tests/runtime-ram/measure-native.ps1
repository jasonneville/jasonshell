[CmdletBinding()]
param([Parameter(Mandatory)][string]$Stage,[Parameter(Mandatory)][string]$Evidence,[switch]$ConsentNativeLaunch,[ValidateRange(3,10)][int]$Pairs=3,[switch]$AffectedSurfaces)
Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'
if(-not $ConsentNativeLaunch) { throw 'Explicit native-launch consent required' }
if($Pairs -lt 3) { throw 'At least three pairs required' }
Add-Type -Path "$PSScriptRoot/native-observer.cs"
. "$PSScriptRoot/native-tree.ps1"
Add-Type -Path "$PSScriptRoot/native-desktop-observer.cs"
. "$PSScriptRoot/desktop-invariants.ps1"
function Snapshot { @(Get-CimInstance Win32_Process -Property ProcessId,ParentProcessId,CreationDate,Name) }
function Desktop { [DesktopInspector]::Read() }
function SaveJson($value,$path) { $value | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $path }
$Stage=(Resolve-Path $Stage).Path; $Evidence=(Resolve-Path $Evidence).Path
if(Test-Path "$Evidence/environment.json") { throw 'Refuse overwriting campaign evidence' }
$campaign=Split-Path $Evidence -Leaf
$profile=Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'com.jnev1.jasonshell.rammeasurement20261113'
$roaming=Join-Path ([Environment]::GetFolderPath('ApplicationData')) 'com.jnev1.jasonshell.rammeasurement20261113'
if((Test-Path $profile) -or (Test-Path $roaming)) { throw 'Measurement identifier profile already exists; refuse adopting it' }
if(@(Snapshot | Where-Object Name -eq 'jason-shell.exe').Count) { throw 'Preexisting JasonShell; refuse concurrent launch' }
$initial=Desktop
SaveJson @{desktop=$initial;os=[Environment]::OSVersion.VersionString;powershell=$PSVersionTable.PSVersion.ToString();profile=$profile;roaming=$roaming;warmupSeconds=60;sampleSeconds=15;intervalMs=1000;desktopIntervalMs=200;requiredStableSeconds=10;desktopSettlingDeadlineSeconds=30;affectedSurfaces=[bool]$AffectedSurfaces;nativeIdentifier='com.jnev1.jasonshell.rammeasurement20261113';model='fresh isolated profile; none imported';normalPrewarm=$true;normalHooks=$true;limits='sampled descendants; sub-second children may be missed; working sets double-count shared pages; no GPU allocations; cold means fresh process/profile, not OS file-cache flush'} "$Evidence/environment.json"
$orders=@(@('baseline','current'),@('current','baseline'),@('baseline','current'))
for($pair=0;$pair -lt $Pairs;$pair++) {
 foreach($variant in $orders[$pair % 3]) {
  $name="pair$($pair+1)-$variant"
  $out=Join-Path $Evidence $name
  if(Test-Path $out) { throw 'Refuse overwriting evidence' }
  New-Item -ItemType Directory $out | Out-Null
  $beforeSeries=[Collections.Generic.List[object]]::new()
  try {
   $baselineClock=[Diagnostics.Stopwatch]::StartNew()
   do { $beforeSeries.Add((Desktop)); Start-Sleep -Milliseconds 200 } while($baselineClock.Elapsed.TotalSeconds -le 10.3)
  } catch { $beforeSeries.Add([pscustomobject]@{Error=$_.Exception.Message;Utc=[datetime]::UtcNow}); throw }
  finally { SaveJson $beforeSeries.ToArray() "$out/desktop-before.json" }
  $before=$beforeSeries.ToArray()
  if(-not (Test-StableDesktopWindow $before (Get-DesktopInvariant $before[0]))) { throw 'Unstable physical prelaunch desktop or pointer; no app launched' }
  $pre=Snapshot
  if(@($pre | Where-Object Name -eq 'jason-shell.exe').Count) { throw 'Preexisting JasonShell' }
  $excluded=@{}; foreach($row in $pre) { $excluded["$($row.ProcessId):$($row.CreationDate.ToUniversalTime().Ticks)"]=$true }
  $known=@{}; $series=[Collections.Generic.List[object]]::new(); $failure=$null; $root=$null; $ready=$null; $rootTicks=0L; $workflow=$null; $workflowStart=$null; $quietStart=$null
  $exe=Join-Path $Stage "$variant/jason-shell.exe"
  $expected=(Get-Content "$Evidence/$variant-binary.json" | ConvertFrom-Json | Where-Object Path -like '*jason-shell.exe').Hash
  if((Get-FileHash $exe).Hash -ne $expected) { throw 'Executable hash mismatch' }
  $start=[Diagnostics.ProcessStartInfo]::new($exe); $start.WorkingDirectory=Join-Path $Stage $variant; $start.UseShellExecute=$false
  $udf=Join-Path $Stage "$campaign-$name-webview-profile"
  if(Test-Path $udf) { throw 'Refuse adopting existing WebView profile' }
  $start.Environment['WEBVIEW2_USER_DATA_FOLDER']=$udf
  # No browser flags, GC, trim, or application feature-policy overrides.
  $clock=[Diagnostics.Stopwatch]::StartNew()
  $launchUtc=[datetime]::UtcNow
  try {
   $root=[Diagnostics.Process]::Start($start)
   $null=$root.SafeHandle # pin the launched process object, never reopen it for cleanup
   $rootTicks=$root.StartTime.ToUniversalTime().Ticks
    while($clock.Elapsed.TotalSeconds -lt 300) {
    $snap=Snapshot
    if($known.Count -eq 0) {
     $row=$snap | Where-Object { $_.ProcessId -eq $root.Id } | Select-Object -First 1
     if($null -eq $row) { throw 'Root exited before first snapshot' }
     if([Math]::Abs($row.CreationDate.ToUniversalTime().Ticks-$rootTicks) -gt 10) { throw 'Root creation identity mismatch' }
     $known["$($row.ProcessId):$($row.CreationDate.ToUniversalTime().Ticks)"]=$row
    }
    $owned=@(Update-OwnedTree $known $snap $excluded)
    $metrics=@(foreach($row in $owned) {
     $p=$null
     try {
      $p=[Diagnostics.Process]::GetProcessById([int]$row.ProcessId)
      $null=$p.SafeHandle
      $ticks=$p.StartTime.ToUniversalTime().Ticks
      if([Math]::Abs($ticks-$row.CreationDate.ToUniversalTime().Ticks) -gt 10) { throw 'Process creation identity changed while sampling' }
      $p.Refresh()
      @{pid=$p.Id;parentPid=$row.ParentProcessId;createdUtc=$p.StartTime.ToUniversalTime().ToString('o');identityTicks=$ticks;name=$row.Name;privateBytes=$p.PrivateMemorySize64;workingSet=$p.WorkingSet64;cpuMs=$p.TotalProcessorTime.TotalMilliseconds;threads=$p.Threads.Count;handles=$p.HandleCount;isRoot=($p.Id -eq $root.Id -and $ticks -eq $rootTicks)}
     } catch { @{pid=$row.ProcessId;error=$_.Exception.Message} } finally { if($null -ne $p) { $p.Dispose() } }
    })
    $windows=@([RamNative]::Windows($root.Id))
    $bars=@($windows | Where-Object { $_.Visible -and $_.Title -in @('JasonShell Top Bar','JasonShell Bottom Bar') })
    if($null -eq $ready -and $bars.Count -eq 2) { $ready=$clock.Elapsed.TotalSeconds; SaveJson @{elapsedSeconds=$ready;windows=$windows;desktop=(Desktop)} "$out/readiness.json" }
    $phase=if($null -eq $ready) {'startup'} elseif($clock.Elapsed.TotalSeconds-$ready -lt 15) {'cold-idle'} elseif($clock.Elapsed.TotalSeconds-$ready -ge 60) {'warm-idle'} else {'warmup'}
    if($null -ne $workflow) { $phase=if($null -eq $quietStart){'surface-workflow'}else{'post-surface-quiet'} }
    $series.Add(@{elapsedSeconds=$clock.Elapsed.TotalSeconds;phase=$phase;processes=$metrics;processCount=@($metrics | Where-Object { $_.ContainsKey('privateBytes') }).Count})
    if($root.HasExited) { throw "Unexpected root exit $($root.ExitCode)" }
    if($null -ne $ready -and $clock.Elapsed.TotalSeconds-$ready -ge 75) {
     if(-not $AffectedSurfaces) { break }
     if($null -eq $workflow) {
      $worker=[Diagnostics.ProcessStartInfo]::new('pwsh'); $worker.UseShellExecute=$false; $worker.CreateNoWindow=$true
      foreach($arg in @('-NoProfile','-File',"$PSScriptRoot/native-surface-workflow.ps1",'-RootPid',"$($root.Id)",'-RootTicks',"$rootTicks",'-Output',"$out/workflow.json")) { $worker.ArgumentList.Add($arg) }
      $workflowStart=$clock.Elapsed.TotalSeconds
      $workflow=[Diagnostics.Process]::Start($worker); $null=$workflow.SafeHandle
     } elseif($workflow.HasExited) {
      if($workflow.ExitCode -ne 0) { throw 'Affected-surface workflow failed; see workflow.json' }
      if($null -eq $quietStart) { $quietStart=$clock.Elapsed.TotalSeconds }
      if($clock.Elapsed.TotalSeconds-$quietStart -ge 15) { break }
     } elseif($clock.Elapsed.TotalSeconds-$workflowStart -gt 120) { throw 'Affected-surface helper exceeded bounded deadline' }
    }
    Start-Sleep -Milliseconds 1000
   }
   if($null -eq $ready -or $clock.Elapsed.TotalSeconds -ge 300) { throw 'Native scenario deadline exceeded' }
  } catch { $failure=$_.Exception.Message }
  finally {
   # Stop only our pinned automation helper before closing its target app, so no
   # delayed UIA action can act after cleanup. Never enumerate or kill user processes.
   if($null -ne $workflow) {
    if(-not $workflow.HasExited) { $workflow.Kill(); $workflow.WaitForExit() }
    $workflow.Dispose()
   }
   SaveJson @($series) "$out/series.json"
   SaveJson @($known.Values | Select-Object ProcessId,ParentProcessId,CreationDate,Name) "$out/identities.json"
   $shutdown=[Diagnostics.Stopwatch]::StartNew(); $normal=$false
   if($null -ne $root -and -not $root.HasExited) {
    if($root.StartTime.ToUniversalTime().Ticks -ne $rootTicks) { throw 'Root identity changed' }
    $top=@([RamNative]::Windows($root.Id) | Where-Object Title -eq 'JasonShell Top Bar')
    if($top.Count -eq 1) { $null=[RamNative]::PostMessage([IntPtr]$top[0].Handle,16,[IntPtr]::Zero,[IntPtr]::Zero); $normal=$root.WaitForExit(30000) }
    if(-not $normal) { SaveJson @{failure='Graceful shutdown failed; owned root retained for investigation';rootPid=$root.Id} "$out/shutdown-blocker.json"; throw 'Graceful shutdown failed; stop campaign, no blanket kill' }
   } elseif($null -ne $root) { $normal=$true }
   $survivors=@()
   for($i=0;$i -lt 30;$i++) {
    $survivors=@(Update-OwnedTree $known (Snapshot) $excluded)
    if($survivors.Count -eq 0) { break }
    Start-Sleep -Milliseconds 500
   }
    $afterSeries=[Collections.Generic.List[object]]::new()
    $settling=[Diagnostics.Stopwatch]::StartNew()
    $acceptance=[pscustomobject]@{Accepted=$false;Reason='owned-process-survivors'}
    if($survivors.Count -eq 0) {
     do {
      try { $afterSeries.Add((Desktop)) } catch { $afterSeries.Add([pscustomobject]@{Error=$_.Exception.Message;Utc=[datetime]::UtcNow}) }
      $acceptance=Test-DesktopRestoration $before $afterSeries.ToArray() $launchUtc $survivors.Count
      if($acceptance.Accepted -or $acceptance.Reason -in @('observer-failure-or-incomplete-snapshot','pointer-interference','explorer-identity-changed','not-prospective-prelaunch-baseline')) { break }
      Start-Sleep -Milliseconds 200
     } while($settling.Elapsed.TotalSeconds -lt 30)
    }
    SaveJson $afterSeries.ToArray() "$out/desktop-after.json"
    $restored=$acceptance.Accepted
    SaveJson @{normalExit=$normal;exitCode=$(if($null -ne $root){$root.ExitCode}else{$null});elapsedMs=$shutdown.Elapsed.TotalMilliseconds;survivors=@($survivors | Select-Object ProcessId,CreationDate,Name);launchUtc=$launchUtc;restoration=$acceptance;restored=$restored;failure=$failure;readySeconds=$ready;workflowStartSeconds=$workflowStart;quietStartSeconds=$quietStart;rootPid=$(if($null -ne $root){$root.Id}else{$null});rootIdentityTicks="$rootTicks";binaryHash=$expected} "$out/result.json"
   if($null -ne $root) { $root.Dispose() }
   if($survivors.Count -gt 0 -or -not $restored) { throw 'Cleanup/restoration mismatch; stop campaign' }
   # Move only directories absent before this campaign and owned by its unique identifier.
   if(Test-Path $profile) { Move-Item $profile (Join-Path $Stage "$campaign-$name-native-profile") }
   if(Test-Path $roaming) { Move-Item $roaming (Join-Path $Stage "$campaign-$name-roaming-profile") }
  }
  if($null -ne $failure) { throw $failure }
  Write-Output "$name completed; native shell restored"
 }
}
