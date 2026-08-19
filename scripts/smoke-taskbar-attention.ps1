<#
.SYNOPSIS
Runs deterministic taskbar attention smoke checks against the built JasonShell exe.

.DESCRIPTION
Launches a normal JasonShell receiver with native taskbar hooks enabled, then runs
the flash fixture subprocess, captures stdout JSONL evidence, correlates fixture
request/focus lines with receiver nativeHook Flash/Foreground lines by HWND and
timestamps, calculates pass/fail latency against a 250ms bound, and cleans up
spawned processes and temp files.

.PARAMETER ExePath
Path to the JasonShell exe under test. Defaults to the built debug exe when present.

.PARAMETER Mode
Optional manual Explorer-visible/hidden matrix mode. Deterministic and nonpersistent.

.PARAMETER ResultsDir
Directory for smoke evidence output.

.PARAMETER TimeoutMs
Per-case process timeout.

.EXAMPLE
pwsh -File scripts/smoke-taskbar-attention.ps1
#>
[CmdletBinding()]
param(
    [string]$ExePath,
    [ValidateSet('auto', 'manual-matrix')]
    [string]$Mode = 'auto',
    [string]$ResultsDir = 'smoke-results',
    [int]$TimeoutMs = 15000
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Resolve-JasonShellExe {
    param([string]$CandidatePath)

    if (-not [string]::IsNullOrWhiteSpace($CandidatePath)) {
        return (Resolve-Path -LiteralPath $CandidatePath).Path
    }

    $defaults = @(
        (Join-Path $PSScriptRoot '..\src-tauri\target\debug\jason-shell.exe'),
        (Join-Path $PSScriptRoot '..\target\debug\jason-shell.exe')
    )

    foreach ($path in $defaults) {
        if (Test-Path -LiteralPath $path) {
            return (Resolve-Path -LiteralPath $path).Path
        }
    }

    throw 'No built debug JasonShell exe found. Pass -ExePath.'
}

function New-TempPath {
    param([string]$Prefix)

    $name = "{0}-{1}-{2}.jsonl" -f $Prefix, $PID, ([guid]::NewGuid().ToString('N'))
    return [System.IO.Path]::Combine([System.IO.Path]::GetTempPath(), $name)
}

function Start-ReceiverProcess {
    param(
        [string]$Exe,
        [hashtable]$Env,
        [string]$StdoutPath,
        [string]$StderrPath
    )

    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $Exe
    $psi.ArgumentList.Clear()
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    foreach ($key in $Env.Keys) { $psi.Environment[$key] = [string]$Env[$key] }

    $proc = [System.Diagnostics.Process]::new()
    $proc.StartInfo = $psi
    $stdout = [System.Collections.Generic.List[string]]::new()
    [void]$proc.Start()
    return [pscustomobject]@{
        Process = $proc
        Stdout = $stdout
        PendingStdout = $proc.StandardOutput.ReadLineAsync()
        StderrTask = $proc.StandardError.ReadToEndAsync()
        StdoutPath = $StdoutPath
        StderrPath = $StderrPath
    }
}

function Update-ReceiverOutput {
    param($Receiver)

    while ($Receiver.PendingStdout -and $Receiver.PendingStdout.IsCompleted) {
        $line = $Receiver.PendingStdout.GetAwaiter().GetResult()
        if ($null -eq $line) {
            $Receiver.PendingStdout = $null
            break
        }
        $Receiver.Stdout.Add($line)
        $Receiver.PendingStdout = $Receiver.Process.StandardOutput.ReadLineAsync()
    }
}

function Start-FixtureProcess {
    param(
        [string]$Exe,
        [string[]]$Args,
        [int]$Timeout,
        [hashtable]$Env,
        [string]$StdoutPath,
        [string]$StderrPath
    )

    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $Exe
    $psi.ArgumentList.Clear()
    foreach ($arg in $Args) { [void]$psi.ArgumentList.Add($arg) }
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    foreach ($key in $Env.Keys) { $psi.Environment[$key] = [string]$Env[$key] }

    $proc = [System.Diagnostics.Process]::new()
    $proc.StartInfo = $psi
    [void]$proc.Start()
    $stdoutTask = $proc.StandardOutput.ReadToEndAsync()
    $stderrTask = $proc.StandardError.ReadToEndAsync()
    if (-not $proc.WaitForExit($Timeout)) {
        try { $proc.Kill($true) } catch { }
        throw "Fixture timed out after ${Timeout}ms"
    }
    $outText = $stdoutTask.GetAwaiter().GetResult()
    $errText = $stderrTask.GetAwaiter().GetResult()
    Set-Content -LiteralPath $StdoutPath -Value $outText -NoNewline -Encoding utf8
    Set-Content -LiteralPath $StderrPath -Value $errText -NoNewline -Encoding utf8
    return $proc
}

if (-not $IsWindows) { throw 'Windows only.' }

$exe = Resolve-JasonShellExe -CandidatePath $ExePath
New-Item -ItemType Directory -Force -Path $ResultsDir | Out-Null
$runId = Get-Date -Format 'yyyyMMdd-HHmmss'
$resultPath = Join-Path $ResultsDir "taskbar-attention-$runId.md"
$tempPaths = New-Object System.Collections.Generic.List[string]
$receiver = $null

$env = @{
    'JASONSHELL_TASKBAR_NATIVE_HOOKS' = '1'
}

$cases = @(
    @{ Name = 'visible'; Args = @('--taskbar-flash-fixture', '--flash-count', '1', '--interval-ms', '10', '--timeout-ms', '1000'); Expect = 0 },
    @{ Name = 'minimized'; Args = @('--taskbar-flash-fixture', '--minimized', '--flash-count', '1', '--interval-ms', '10', '--timeout-ms', '1000'); Expect = 0 }
)

if ($Mode -eq 'manual-matrix') {
    $cases += @{ Name = 'matrix-visible'; Args = @('--taskbar-flash-fixture', '--flash-count', '2', '--interval-ms', '10', '--timeout-ms', '1000'); Expect = 0 }
    $cases += @{ Name = 'matrix-hidden'; Args = @('--taskbar-flash-fixture', '--minimized', '--flash-count', '2', '--interval-ms', '10', '--timeout-ms', '1000'); Expect = 0 }
}

$rows = @()
$cleanupPaths = $tempPaths
try {
    $receiverStdout = New-TempPath -Prefix 'taskbar-receiver-stdout'
    $receiverStderr = New-TempPath -Prefix 'taskbar-receiver-stderr'
    $tempPaths.Add($receiverStdout) | Out-Null
    $tempPaths.Add($receiverStderr) | Out-Null
    $receiver = Start-ReceiverProcess -Exe $exe -Env $env -StdoutPath $receiverStdout -StderrPath $receiverStderr
    $readyDeadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
    while ($true) {
        Update-ReceiverOutput -Receiver $receiver
        if ($receiver.Stdout | Where-Object { $_ -like '*"kind":"nativeHookInit"*' }) { break }
        if ($receiver.Process.HasExited) { throw 'Receiver exited before native hooks initialized.' }
        if ([DateTime]::UtcNow -ge $readyDeadline) { throw 'Receiver native-hook initialization timed out.' }
        Start-Sleep -Milliseconds 25
    }
    foreach ($case in $cases) {
        $fixtureStdout = New-TempPath -Prefix "taskbar-$($case.Name)-stdout"
        $fixtureStderr = New-TempPath -Prefix "taskbar-$($case.Name)-stderr"
        $tempPaths.Add($fixtureStdout) | Out-Null
        $tempPaths.Add($fixtureStderr) | Out-Null
        $fixture = Start-FixtureProcess -Exe $exe -Args $case.Args -Env @{} -Timeout $TimeoutMs -StdoutPath $fixtureStdout -StderrPath $fixtureStderr
        $fixtureEvidence = Join-Path $ResultsDir "taskbar-attention-$runId-$($case.Name).jsonl"
        Copy-Item -LiteralPath $fixtureStdout -Destination $fixtureEvidence -Force
        $fixtureOut = Get-Content -LiteralPath $fixtureStdout | Where-Object { $_ }
        $fixtureEvents = @($fixtureOut | ForEach-Object { $_ | ConvertFrom-Json -AsHashtable })
        $request = $fixtureEvents | Where-Object { $_.event -eq 'request' } | Select-Object -First 1
        $focus = $fixtureEvents | Where-Object { $_.event -eq 'focus' } | Select-Object -First 1
        $flash = $null
        $foreground = $null
        if ($request -and $focus) {
            $eventDeadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
            do {
                Update-ReceiverOutput -Receiver $receiver
                $receiverEvents = @($receiver.Stdout | ForEach-Object { $_ | ConvertFrom-Json -AsHashtable })
                $flash = $receiverEvents | Where-Object { $_.kind -eq 'nativeHook' -and $_.signal -eq 'Flash' -and $_.hwnd -eq $request.hwnd -and $_.timestampMs -ge $request.timestamp_ms } | Select-Object -First 1
                $foreground = $receiverEvents | Where-Object { $_.kind -eq 'nativeHook' -and $_.signal -eq 'Foreground' -and $_.hwnd -eq $focus.hwnd -and $_.timestampMs -ge $focus.timestamp_ms } | Select-Object -First 1
                if ($flash -and $foreground) { break }
                Start-Sleep -Milliseconds 25
            } while ([DateTime]::UtcNow -lt $eventDeadline)
        }
        $detectLatency = if ($request -and $flash) { [int]([math]::Max(([int64]$flash.timestampMs - [int64]$request.timestamp_ms), 0)) } else { [int]::MaxValue }
        $clearLatency = if ($focus -and $foreground) { [int]([math]::Max(([int64]$foreground.timestampMs - [int64]$focus.timestamp_ms), 0)) } else { [int]::MaxValue }
        $latency = [math]::Max($detectLatency, $clearLatency)
        $pass = $fixture.ExitCode -eq $case.Expect -and $detectLatency -le 250 -and $clearLatency -le 250
        $rows += [pscustomobject]@{
            Case = $case.Name
            ExitCode = $fixture.ExitCode
            LatencyMs = $latency
            DetectionLatencyMs = $detectLatency
            ClearLatencyMs = $clearLatency
            Pass = $pass
            ReceiverHwnd = if ($request) { $request.hwnd } else { $null }
            RequestHwnd = if ($request) { $request.hwnd } else { $null }
            ForegroundHwnd = if ($focus) { $focus.hwnd } else { $null }
            ClearHwnd = if ($foreground) { $foreground.hwnd } else { $null }
            RequestAt = if ($request) { $request.timestamp_ms } else { $null }
            DetectedAt = if ($flash) { $flash.timestampMs } else { $null }
            ForegroundAt = if ($focus) { $focus.timestamp_ms } else { $null }
            ClearAt = if ($foreground) { $foreground.timestampMs } else { $null }
            EvidencePath = $fixtureEvidence
        }
    }
} finally {
    if ($receiver -and -not $receiver.Process.HasExited) { try { $receiver.Process.Kill($true) } catch { } }
    if ($receiver) { try { $receiver.Process.WaitForExit(2000) | Out-Null } catch { } }
    if ($receiver) {
        Update-ReceiverOutput -Receiver $receiver
        $receiver.Stdout | Set-Content -LiteralPath $receiverStdout -Encoding utf8
        $receiver.StderrTask.GetAwaiter().GetResult() | Set-Content -LiteralPath $receiverStderr -NoNewline -Encoding utf8
        Copy-Item -LiteralPath $receiverStdout -Destination (Join-Path $ResultsDir "taskbar-attention-$runId-receiver.jsonl") -Force
        Copy-Item -LiteralPath $receiverStderr -Destination (Join-Path $ResultsDir "taskbar-attention-$runId-receiver.stderr.txt") -Force
    }
    foreach ($p in $cleanupPaths) { if (Test-Path -LiteralPath $p) { Remove-Item -LiteralPath $p -Force } }
}

$failures = @($rows | Where-Object { -not $_.Pass })
$lines = @(
    "# Taskbar Attention Smoke - $runId",
    '',
    "## Env",
    "- Exe: $exe",
    "- Mode: $Mode",
    "- TimeoutMs: $TimeoutMs",
    '',
    '## Results'
)
foreach ($row in $rows) {
    $lines += @(
        '',
        "### $($row.Case)",
        "- ExitCode: $($row.ExitCode)",
        "- LatencyMs: $($row.LatencyMs)",
        "- DetectionLatencyMs: $($row.DetectionLatencyMs)",
        "- ClearLatencyMs: $($row.ClearLatencyMs)",
        "- Pass: $($row.Pass)",
        "- ReceiverHwnd: $($row.ReceiverHwnd)",
        "- RequestHwnd: $($row.RequestHwnd)",
        "- ForegroundHwnd: $($row.ForegroundHwnd)",
        "- ClearHwnd: $($row.ClearHwnd)",
        "- RequestAt: $($row.RequestAt)",
        "- DetectedAt: $($row.DetectedAt)",
        "- ForegroundAt: $($row.ForegroundAt)",
        "- ClearAt: $($row.ClearAt)",
        "- Evidence: $($row.EvidencePath)"
    )
}
$lines | Set-Content -LiteralPath $resultPath -Encoding utf8

exit ($(if ($failures.Count -gt 0) { 1 } else { 0 }))
