param([Parameter(Mandatory)][string]$Output)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
Add-Type -Path "$PSScriptRoot/native-desktop-observer.cs"
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class DpiObservationTest {
 [DllImport("user32.dll")] public static extern IntPtr GetThreadDpiAwarenessContext();
 [DllImport("user32.dll")] public static extern bool AreDpiAwarenessContextsEqual(IntPtr a,IntPtr b);
}
'@
$before=[DpiObservationTest]::GetThreadDpiAwarenessContext()
$state=[DesktopInspector]::Read()
$after=[DpiObservationTest]::GetThreadDpiAwarenessContext()
if(-not [DpiObservationTest]::AreDpiAwarenessContextsEqual($before,$after)) { throw 'Observer changed thread DPI context' }
if($state.WorkArea.Right -le $state.WorkArea.Left -or $state.WorkArea.Bottom -le $state.WorkArea.Top -or $state.ExplorerPid -le 0 -or $state.ExplorerCreatedTicks -le 0) { throw 'Incomplete native observer state' }
if($state.Visible -ne (($state.Style -band 0x10000000) -ne 0)) { throw 'Visibility and captured WS_VISIBLE disagree' }
$state | ConvertTo-Json -Depth 5 | Set-Content $Output
foreach($parameters in @(@{Samples=0;IntervalMs=200},@{Samples=1;IntervalMs=0},@{Samples=1;IntervalMs=1001})) {
 $failed=$false
 try { & "$PSScriptRoot/inspect-desktop.ps1" -Output "$Output-must-not-exist" @parameters } catch { $failed=$true }
 if(-not $failed -or (Test-Path "$Output-must-not-exist")) { throw 'Invalid sampler bounds accepted' }
}
$failed=$false
try { & "$PSScriptRoot/measure-native.ps1" -Stage $PSScriptRoot -Evidence $PSScriptRoot } catch { $failed=$_.Exception.Message -like '*consent*' }
if(-not $failed) { throw 'Native launch consent gate failed' }
Write-Output 'PASS: read-only native observer compiles; valid physical/style/identity fields, visibility parity, DPI restoration, three parameter bounds and native-launch consent gate'
