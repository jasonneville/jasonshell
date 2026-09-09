param([Parameter(Mandatory)][string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$output = [IO.Path]::GetFullPath($OutputDirectory)
if (-not $output.StartsWith((Join-Path $root 'test-results') + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Output must be under test-results' }
if (Test-Path -LiteralPath $output) { throw 'New evidence directory required' }
$exe = Join-Path $root 'src-tauri/target/debug/examples/stack_text_probe.exe'
if (-not (Test-Path -LiteralPath $exe)) { throw 'Build packaged probe first' }
New-Item -ItemType Directory -Path $output -Force | Out-Null
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class P01Capture {
  [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left,Top,Right,Bottom; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out Rect rect);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hwnd);
  [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hwnd, out Rect rect);
  [StructLayout(LayoutKind.Sequential)] public struct Point { public int X,Y; }
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hwnd, ref Point point);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
}
'@
$results = @()
[void][P01Capture]::SetThreadDpiAwarenessContext([IntPtr](-4))
function Get-CreationKey([datetime]$Time) {
  # CIM creation times have microsecond precision; native FILETIME has 100ns precision.
  $ticks = $Time.ToUniversalTime().Ticks
  return ($ticks - ($ticks % 10))
}
foreach ($mode in @('editor','control')) {
  $eventFile = Join-Path $output "$mode-events.json"
  $previousArgs = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
  try {
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--proxy-server=http://127.0.0.1:9 --proxy-bypass-list=<-loopback>'
    $process = Start-Process -FilePath $exe -ArgumentList @(('"' + $eventFile + '"'), $mode) -PassThru
  } finally { $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $previousArgs }
  $rootStarted = $process.StartTime.ToUniversalTime()
  $samples = @(); $capture = 'blocked'; $proxyObserved = $false; $started = [Diagnostics.Stopwatch]::StartNew()
  try {
    while ($started.Elapsed.TotalSeconds -lt 12) {
      $process.Refresh()
      if ($process.HasExited) { throw 'Probe exited before capture' }
      $all = @(Get-CimInstance Win32_Process)
      $rootRecord = $all | Where-Object { $_.ProcessId -eq $process.Id -and (Get-CreationKey $_.CreationDate) -eq (Get-CreationKey $rootStarted) }
      if (-not $rootRecord) { throw 'Probe process identity changed or unavailable' }
      $identities = @{}; $identities[[int]$process.Id] = $rootStarted
      do {
        $added = $false
        foreach ($p in $all) {
          $parent = [int]$p.ParentProcessId; $child = [int]$p.ProcessId
          if ($identities.ContainsKey($parent) -and -not $identities.ContainsKey($child) -and $p.CreationDate.ToUniversalTime() -ge $identities[$parent]) {
            $identities[$child] = $p.CreationDate.ToUniversalTime(); $added = $true
          }
        }
      } while ($added)
      foreach ($p in $all) {
        if ($identities.ContainsKey([int]$p.ProcessId)) {
          if ($p.Name -eq 'msedgewebview2.exe' -and $p.CommandLine -match '--proxy-server=http://127.0.0.1:9' -and $p.CommandLine -match '--proxy-bypass-list=.?<-loopback>') { $proxyObserved = $true }
          $live = Get-Process -Id $p.ProcessId -ErrorAction SilentlyContinue
          if ($live -and (Get-CreationKey $live.StartTime) -eq (Get-CreationKey $identities[[int]$p.ProcessId])) { $samples += @{ timeMs=$started.Elapsed.TotalMilliseconds; pid=[int]$p.ProcessId; parentPid=[int]$p.ParentProcessId; createdUtc=$identities[[int]$p.ProcessId].ToString('o'); name=$p.Name; renderer=($p.Name -eq 'msedgewebview2.exe' -and $p.CommandLine -match '--type=renderer'); privateBytes=$live.PrivateMemorySize64; workingSetBytes=$live.WorkingSet64; readTransferBytes=[string]$p.ReadTransferCount; writeTransferBytes=[string]$p.WriteTransferCount } }
        }
      }
      if ($capture -eq 'blocked' -and $process.MainWindowHandle -ne 0 -and $started.Elapsed.TotalSeconds -gt 2) {
        [void][P01Capture]::SetWindowPos($process.MainWindowHandle, [IntPtr](-1), 0, 0, 0, 0, 0x13)
        [void][P01Capture]::SetForegroundWindow($process.MainWindowHandle)
        Start-Sleep -Milliseconds 200
        if ([P01Capture]::GetForegroundWindow() -eq $process.MainWindowHandle -and -not [P01Capture]::IsIconic($process.MainWindowHandle)) {
          $rect = [P01Capture+Rect]::new()
          $origin = [P01Capture+Point]::new()
          if ([P01Capture]::GetClientRect($process.MainWindowHandle, [ref]$rect) -and [P01Capture]::ClientToScreen($process.MainWindowHandle, [ref]$origin)) {
            $bitmap = [Drawing.Bitmap]::new($rect.Right-$rect.Left,$rect.Bottom-$rect.Top)
            $graphics = [Drawing.Graphics]::FromImage($bitmap)
            try { $graphics.CopyFromScreen($origin.X,$origin.Y,0,0,$bitmap.Size); $bitmap.Save((Join-Path $output "$mode-native.png"),[Drawing.Imaging.ImageFormat]::Png); $capture='captured-awaiting-review' }
            finally { $graphics.Dispose(); $bitmap.Dispose() }
          }
        }
      }
      if ($started.Elapsed.TotalSeconds -gt 10 -and (Get-Item -LiteralPath $eventFile).Length -gt 0) { break }
      Start-Sleep -Milliseconds 100
    }
  } finally {
    if (-not $process.HasExited) { [void]$process.CloseMainWindow(); if (-not $process.WaitForExit(10000)) { throw "Owned probe $($process.Id) failed graceful close; no unrelated process terminated" } }
    $results += @{ mode=$mode; pid=$process.Id; exitCode=$process.ExitCode; capture=$capture; samples=$samples; deadProxyArgumentsObserved=$proxyObserved; nativeTiming='blocked: callback timestamps are not presentation timestamps'; executableSha256=(Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash }
  }
}
$results | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $output 'native-process.json') -Encoding utf8
$artifacts = @('editor-events.json','control-events.json','native-process.json','editor-native.png','control-native.png') | ForEach-Object { $path = Join-Path $output $_; if (Test-Path -LiteralPath $path) { @{ name=$_; sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash } } }
$artifacts | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $output 'artifact-hashes.json') -Encoding utf8
$results | ForEach-Object { [pscustomobject]@{ mode=$_.mode; pid=$_.pid; capture=$_.capture; samples=$_.samples.Count; exitCode=$_.exitCode } }
