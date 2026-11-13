$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
. "$PSScriptRoot/desktop-invariants.ps1"
function State($seconds,$visible=$true) {
 [pscustomobject]@{WorkArea=[pscustomobject]@{Left=0;Top=0;Right=2560;Bottom=1440};TaskbarRect=[pscustomobject]@{Left=0;Top=1438;Right=2560;Bottom=1488};Visible=$visible;Handle=42;ExplorerPid=7;ExplorerCreatedTicks=100;Style=0x90000000L;ExStyle=0x80;AppbarState=1;Cursor=[pscustomobject]@{X=123;Y=456};Utc=([datetime]'2026-11-13T00:00:00Z').AddSeconds($seconds)}
}
function Check($condition,$message) { if(-not $condition) { throw $message }; $script:passed++ }
$passed=0
$before=@(0..10 | ForEach-Object { State $_ })
$launch=([datetime]'2026-11-13T00:00:00Z').AddSeconds(11)
$after=@(20..30 | ForEach-Object { State $_ })
Check ((Test-DesktopRestoration $before $after $launch 0).Accepted) 'Stable pre/post rejected'
$transient=@((State 18 $false),(State 19 $false))+$after
Check ((Test-DesktopRestoration $before $transient $launch 0).Accepted) 'Transient visibility followed by stable baseline rejected'
$hidden=@(20..30 | ForEach-Object { State $_ $false })
Check (-not (Test-DesktopRestoration $before $hidden $launch 0).Accepted) 'Persistent hidden accepted with autohide enabled'
foreach($field in @('Handle','ExplorerPid','ExplorerCreatedTicks','AppbarState','Style','ExStyle')) {
 $changed=@(20..30 | ForEach-Object { $x=State $_; $x.$field++; $x })
 Check (-not (Test-DesktopRestoration $before $changed $launch 0).Accepted) "Changed $field accepted"
}
foreach($field in @('WorkArea','TaskbarRect')) {
 $changed=@(20..30 | ForEach-Object { $x=State $_; $x.$field.Bottom--; $x })
 Check (-not (Test-DesktopRestoration $before $changed $launch 0).Accepted) "Changed $field accepted"
}
$cursor=State 18; $cursor.Cursor.X++
Check (-not (Test-DesktopRestoration $before (@($cursor)+$after) $launch 0).Accepted) 'Pointer interference accepted after recovery'
Check ((Test-DesktopRestoration $before @($cursor) $launch 0).Reason -eq 'pointer-interference') 'Single pointer interference did not stop immediately'
Check (-not (Test-DesktopRestoration $before (@([pscustomobject]@{Error='read failure'})+$after) $launch 0).Accepted) 'Read failure accepted after recovery'
Check ((Test-DesktopRestoration $before @([pscustomobject]@{Error='read failure'}) $launch 0).Reason -eq 'observer-failure-or-incomplete-snapshot') 'Single read failure did not stop immediately'
Check (-not (Test-DesktopRestoration $before $after[0..9] $launch 0).Accepted) 'Insufficient stable duration accepted'
Check (-not (Test-DesktopRestoration $before $after $launch 1).Accepted) 'Surviving owned process accepted'
Check (-not (Test-DesktopRestoration $after $after $launch 0).Accepted) 'Post-stop observation replaced missing prelaunch baseline'
Check (-not (Test-DesktopRestoration @() $after $launch 0).Accepted) 'Missing prelaunch baseline accepted'
$recreated=State 18; $recreated.Handle++
Check (-not (Test-DesktopRestoration $before (@($recreated)+$after) $launch 0).Accepted) 'Explorer HWND recreation accepted after original returned'
Check ((Test-DesktopRestoration $before @($recreated) $launch 0).Reason -eq 'explorer-identity-changed') 'Single Explorer identity change did not stop immediately'
Write-Output "PASS: $passed prospective desktop-restoration guard cases"
