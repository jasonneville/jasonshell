$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
. "$PSScriptRoot/desktop-invariants.ps1"
function State($seconds,$visible=$true) {
 [pscustomobject]@{WorkArea=[pscustomobject]@{Left=0;Top=0;Right=2560;Bottom=1440};TaskbarRect=[pscustomobject]@{Left=0;Top=1438;Right=2560;Bottom=1488};Visible=$visible;Handle=42;ExplorerPid=7;ExplorerCreatedTicks=100;Style=0x90000000L;ExStyle=0x80;AppbarState=1;Cursor=[pscustomobject]@{X=123;Y=456};Utc=([datetime]'2026-11-13T00:00:00Z').AddSeconds($seconds)}
}
function Check($condition,$message) { if(-not $condition) { throw $message } }
$a=State 0; $b=State 10; $key=Get-DesktopInvariant $a
Check (Test-StableDesktopWindow @(0..10 | ForEach-Object { State $_ }) $key) 'Matching ten-second window rejected'
Check (-not (Test-StableDesktopWindow @($a,(State 9)) $key)) 'Short window accepted'
Check (-not (Test-StableDesktopWindow @($a,(State 10 $false)) $key)) 'Hidden taskbar accepted'
foreach($field in @('Handle','ExplorerPid','ExplorerCreatedTicks','AppbarState')) {
 $changed=State 10; $changed.$field++
 Check (-not (Test-StableDesktopWindow @($a,$changed) $key)) "Changed $field accepted"
}
foreach($field in @('WorkArea','TaskbarRect')) {
 $changed=State 10; $changed.$field.Bottom--
 Check (-not (Test-StableDesktopWindow @($a,$changed) $key)) "Changed $field accepted"
}
$changed=State 10; $changed.Cursor.X++
Check (-not (Test-StableDesktopWindow @($a,$changed) $key)) 'Changed pointer condition accepted'
Check (-not (Test-StableDesktopWindow @($a,(State 5 $false),$b) $key)) 'Intermediate mismatch accepted'
Write-Output 'PASS: stable physical bounds/identity/autohide/pointer invariants; short/transient/mismatched windows rejected'
