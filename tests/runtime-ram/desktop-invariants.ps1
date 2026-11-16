function Get-DesktopInvariant($state) {
 # Explicit physical fields; timestamps, foreground and sample ordering are diagnostics,
 # not substitutes for matching the owned desktop resource and pointer conditions.
 return (@(
  $state.WorkArea.Left,$state.WorkArea.Top,$state.WorkArea.Right,$state.WorkArea.Bottom,
  $state.TaskbarRect.Left,$state.TaskbarRect.Top,$state.TaskbarRect.Right,$state.TaskbarRect.Bottom,
  $state.Visible,$state.Handle,$state.ExplorerPid,$state.ExplorerCreatedTicks,$state.AppbarState,$state.Style,$state.ExStyle,
  $state.Cursor.X,$state.Cursor.Y
 ) -join '|')
}
function Test-DesktopObservation($state) {
 if($null -eq $state) { return $false }
 foreach($name in @('WorkArea','TaskbarRect','Visible','Handle','ExplorerPid','ExplorerCreatedTicks','Style','ExStyle','AppbarState','Cursor','Utc')) {
  if($null -eq $state.PSObject.Properties[$name]) { return $false }
 }
 return $true
}
function Test-DesktopRestoration {
 param([object[]]$Before,[object[]]$After,[datetime]$LaunchUtc,[int]$OwnedSurvivorCount,[double]$RequiredSeconds=10,[int]$RequiredSamples=10)
 $reject={ param($why) [pscustomobject]@{Accepted=$false;Reason=$why} }
 if($OwnedSurvivorCount -ne 0) { return (& $reject 'owned-process-survivors') }
 if($Before.Count -lt $RequiredSamples -or $After.Count -eq 0) { return (& $reject 'insufficient-samples') }
 foreach($s in @($Before)+@($After)) { if(-not (Test-DesktopObservation $s)) { return (& $reject 'observer-failure-or-incomplete-snapshot') } }
 if($Before[-1].Utc -ge $LaunchUtc -or $After[0].Utc -le $LaunchUtc) { return (& $reject 'not-prospective-prelaunch-baseline') }
 $expected=Get-DesktopInvariant $Before[0]
 if(-not (Test-StableDesktopWindow $Before $expected $RequiredSeconds)) { return (& $reject 'unstable-prelaunch-baseline') }
 $baseline=$Before[0]
 foreach($s in $After) {
  if($s.Cursor.X -ne $baseline.Cursor.X -or $s.Cursor.Y -ne $baseline.Cursor.Y) { return (& $reject 'pointer-interference') }
  if($s.Handle -ne $baseline.Handle -or $s.ExplorerPid -ne $baseline.ExplorerPid -or $s.ExplorerCreatedTicks -ne $baseline.ExplorerCreatedTicks) { return (& $reject 'explorer-identity-changed') }
 }
 if($After.Count -lt $RequiredSamples) { return (& $reject 'insufficient-samples') }
 # Only a continuous suffix matching the ORIGINAL physical baseline can pass.
 # A transient hidden observation stays in raw evidence and never waives visibility.
 $start=$After.Count-1
 while($start -ge 0 -and (Get-DesktopInvariant $After[$start]) -eq $expected) { $start-- }
 if($start -eq $After.Count-1) { return (& $reject 'desktop-invariant-mismatch') }
 $suffix=@($After[($start+1)..($After.Count-1)])
 if($suffix.Count -lt $RequiredSamples -or -not (Test-StableDesktopWindow $suffix $expected $RequiredSeconds)) { return (& $reject 'not-yet-stable-baseline') }
 return [pscustomobject]@{Accepted=$true;Reason='original-baseline-returned-and-stable';StableSampleCount=$suffix.Count;StableSeconds=($suffix[-1].Utc-$suffix[0].Utc).TotalSeconds}
}
function Test-StableDesktopWindow {
 param([object[]]$Samples,[string]$Expected,[double]$RequiredSeconds=10)
 if($Samples.Count -lt 2) { return $false }
 if(($Samples[-1].Utc-$Samples[0].Utc).TotalSeconds -lt $RequiredSeconds) { return $false }
 foreach($sample in $Samples) { if((Get-DesktopInvariant $sample) -ne $Expected) { return $false } }
 for($i=1;$i -lt $Samples.Count;$i++) {
  $gap=($Samples[$i].Utc-$Samples[$i-1].Utc).TotalSeconds
  if($gap -le 0 -or $gap -gt 1.5) { return $false }
 }
 return $true
}
