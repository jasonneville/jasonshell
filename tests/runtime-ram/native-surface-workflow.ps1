param([Parameter(Mandatory)][int]$RootPid,[Parameter(Mandatory)][long]$RootTicks,[Parameter(Mandatory)][string]$Output)
$ErrorActionPreference='Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -Path "$PSScriptRoot/native-observer.cs"
$events=[Collections.Generic.List[object]]::new()
$clock=[Diagnostics.Stopwatch]::StartNew()
function Assert-Root {
 $p=[Diagnostics.Process]::GetProcessById($RootPid)
 try { if($p.StartTime.ToUniversalTime().Ticks -ne $RootTicks) { throw 'Workflow root identity mismatch' } } finally { $p.Dispose() }
}
function Window($title) {
 Assert-Root
 $windows=@([RamNative]::Windows($RootPid) | Where-Object { $_.Title -eq $title -and $_.Visible })
 if($windows.Count -ne 1) { return $null }
 return [System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$windows[0].Handle)
}
function Named($window,$name) {
 if($null -eq $window) { return $null }
 return $window.FindFirst([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty,$name))
}
function Wait-For($predicate,$description) {
 $wait=[Diagnostics.Stopwatch]::StartNew()
 do { Assert-Root; $value=& $predicate; if($value) { return $value }; Start-Sleep -Milliseconds 100 } while($wait.Elapsed.TotalSeconds -lt 15)
 throw "UIA deadline: $description"
}
function Invoke-Button($title,$name) {
 $allowed=@('JasonShell Bottom Bar|Open process manager','JasonShell Process Manager|Close process manager','JasonShell Top Bar|Open persistent terminal')
 if("$title|$name" -notin $allowed) { throw 'Workflow action not allowlisted' }
 $button=Wait-For { Named (Window $title) $name } $name
 Assert-Root
 if(-not $button.Current.IsEnabled -or $button.Current.IsOffscreen) { throw "Button not available: $name" }
 ([System.Windows.Automation.InvokePattern]$button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)).Invoke()
}
function Record($name,$duration) { $events.Add(@{name=$name;elapsedSeconds=$clock.Elapsed.TotalSeconds;durationMs=$duration}) }
function Loaded-ProcessRows {
 $grid=Named (Window 'JasonShell Process Manager') 'Running processes'
 if($null -eq $grid) { return 0 }
 $buttons=$grid.FindAll([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ControlTypeProperty,[System.Windows.Automation.ControlType]::Button))
 $count=0
 foreach($button in $buttons) {
  # These names exist only on actual production data rows. Read/count only;
  # this function never obtains InvokePattern or records process identity text.
  if($button.Current.Name -match '^(Arm single-process kill confirmation for |.+ process \d+ is protected from kill actions$)') { $count++ }
 }
 return $count
}
$failure=$null
try {
 for($i=1;$i -le 3;$i++) {
  $timer=[Diagnostics.Stopwatch]::StartNew()
  Invoke-Button 'JasonShell Bottom Bar' 'Open process manager'
  $rows=Wait-For { Loaded-ProcessRows } 'positive production process snapshot rows'
  $timer.Stop()
  Record "process-open-$i" $timer.Elapsed.TotalMilliseconds
  # Separate fixed dwell from invoke-to-positive-loaded-snapshot timing.
  Start-Sleep -Seconds 3
  $events.Add(@{name='process-row-count';count=$rows;fixedDwellSeconds=3;elapsedSeconds=$clock.Elapsed.TotalSeconds})
  Invoke-Button 'JasonShell Process Manager' 'Close process manager'
  $null=Wait-For { -not (Window 'JasonShell Process Manager') } 'process manager hidden'
  Record "process-close-$i" 0
  Start-Sleep -Seconds 3
 }
 for($i=1;$i -le 3;$i++) {
  $timer=[Diagnostics.Stopwatch]::StartNew()
  Invoke-Button 'JasonShell Top Bar' 'Open persistent terminal'
  $null=Wait-For { Named (Window 'JasonShell Terminal') 'Terminal output' } 'terminal pane attached'
  $timer.Stop()
  Record "terminal-open-$i" $timer.Elapsed.TotalMilliseconds
  Start-Sleep -Seconds 3
  Invoke-Button 'JasonShell Top Bar' 'Open persistent terminal'
  $null=Wait-For { -not (Window 'JasonShell Terminal') } 'terminal panel hidden'
  Record "terminal-hide-$i" 0
  Start-Sleep -Seconds 3
 }
} catch { $failure=$_.Exception.Message }
@{events=$events.ToArray();failure=$failure;elapsedSeconds=$clock.Elapsed.TotalSeconds;limits='UIA instrumentation enables accessibility; native invoke-to-observed-control timings include synchronous UIA overhead. Terminal hide/show keeps same mounted session; not a backend restart/reattach or output-churn test. No terminal commands or Kill actions.'} | ConvertTo-Json -Depth 8 | Set-Content $Output
if($null -ne $failure) { throw $failure }
