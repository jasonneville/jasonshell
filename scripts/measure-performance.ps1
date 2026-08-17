<#
.SYNOPSIS
Measures JasonShell performance in explicit release and dev diagnostic modes.

.DESCRIPTION
Plan 01 harness only. Creates a dated ignored artifact root under
test-results/performance-regression/<timestamp>/, captures exactly three attempts
for each of the seven required scenarios, writes per-run JSON with a versioned
schema, and emits summary.md plus residual-risk.md.

Release evidence is distinct from diagnostic evidence. Release mode must use a
discovered jason-shell.exe release binary from src-tauri/target/release or
target/release after a normal release build. Dev diagnostic mode is separate and
never counts as release acceptance evidence.
#>
[CmdletBinding()]
param(
    [ValidateSet('release', 'dev')]
    [string]$Mode = 'release',

    [switch]$NonInteractive,

    [string[]]$Scenarios = @(
        'cold-idle',
        '20+-windows',
        'notifications',
        'noisy-quick-commands-output',
        'terminal-hidden-prewarm',
        'fullscreen',
        'multi-monitor'
    ),

    [ValidateRange(3, 3)]
    [int]$RunsPerScenario = 3,

    [string]$OutputRoot = 'test-results/performance-regression',

    [string]$ReleaseBinaryPath,

    [switch]$ControlIoProbe,

    [ValidateRange(1, 86400)]
    [int]$ScenarioTimeoutSeconds = 300
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-DateStamp { (Get-Date).ToString('yyyyMMdd-HHmmssfff') + '-' + ([guid]::NewGuid().ToString('N')) }

function Write-JsonArtifact {
    param([string]$Path, [object]$Value)
    $Value | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $Path -Encoding utf8
}

function New-ScenarioResultSchema {
    param([Parameter(Mandatory)] [string]$SchemaVersion)
    @{
        schemaVersion = $SchemaVersion
        mode = $Mode
        startedAt = $null
        finishedAt = $null
        scenario = @{ name = $null; metadata = @{}; status = $null; reason = $null }
        processMetrics = @{ cpuDeltaMs = $null; sampleElapsedMs = $null; cpuPercent = $null; cpu = 0; privateBytes = $null; workingSet = $null; threadCount = $null; handleCount = $null }
        controlIo = @{ available = $null; readCount = $null; writeCount = $null; note = $null }
        error = $null
    }
}

function Get-Median {
    param([double[]]$Values)
    if (-not $Values -or $Values.Count -eq 0) { return $null }
    $sorted = $Values | Sort-Object
    $mid = [math]::Floor(($sorted.Count - 1) / 2)
    if ($sorted.Count % 2 -eq 1) { return [double]$sorted[$mid] }
    return ([double]$sorted[$mid] + [double]$sorted[$mid + 1]) / 2
}

function Get-ReleaseBinaryCandidates {
    $roots = @(
        [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\src-tauri\target\release')),
        [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\target\release'))
    )
    $candidates = @()
    foreach ($root in $roots) {
        if (Test-Path -LiteralPath $root) {
            $candidates += Get-ChildItem -LiteralPath $root -Recurse -File -Filter 'jason-shell.exe' -ErrorAction SilentlyContinue
        }
    }
    $candidates |
        Where-Object { $_.Name -ieq 'jason-shell.exe' } |
        Where-Object { $_.Name -notmatch '(setup|installer|uninstall|package|helper|msi|msix)' } |
        Sort-Object FullName
}

function Test-PathWithinRoot {
    param(
        [Parameter(Mandatory)] [string]$Path,
        [Parameter(Mandatory)] [string]$Root
    )

    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $fullRoot = [System.IO.Path]::GetFullPath($Root).TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
    $rootPrefix = $fullRoot + [System.IO.Path]::DirectorySeparatorChar
    return $fullPath.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase) -or $fullPath.Equals($fullRoot, [System.StringComparison]::OrdinalIgnoreCase)
}

function Resolve-ReleaseBinary {
    $releaseRoots = @()
    foreach ($root in @(
        (Join-Path $PSScriptRoot '..\src-tauri\target\release'),
        (Join-Path $PSScriptRoot '..\target\release')
    )) {
        if (Test-Path -LiteralPath $root) {
            $releaseRoots += (Resolve-Path -LiteralPath $root).ProviderPath
        }
    }
    if ($ReleaseBinaryPath) {
        $canonicalPath = (Resolve-Path -LiteralPath $ReleaseBinaryPath).ProviderPath
        if (-not (Test-Path -LiteralPath $canonicalPath)) { throw "Release binary not found: $ReleaseBinaryPath" }
        $item = Get-Item -LiteralPath $canonicalPath
        if ($item.Name -ine 'jason-shell.exe') { throw 'Release binary must be jason-shell.exe only.' }
        if ($item.Name -match '(setup|installer|uninstall|package|helper|msi|msix)') { throw 'Release binary cannot be setup/installer/uninstall/package helper artifact.' }
        $rootMatch = $false
        foreach ($root in $releaseRoots) {
            $rootTrim = $root.TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
            $rootPrefix = $rootTrim + [System.IO.Path]::DirectorySeparatorChar
            if ($canonicalPath.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase) -or $canonicalPath.Equals($rootTrim, [System.StringComparison]::OrdinalIgnoreCase)) { $rootMatch = $true; break }
        }
        if (-not $rootMatch) { throw 'Release binary path must be beneath src-tauri/target/release or target/release.' }
        return $item
    }
    $candidate = (Get-ReleaseBinaryCandidates | Sort-Object FullName)[0]
    if (-not $candidate) { throw 'Release binary jason-shell.exe not discovered under src-tauri/target/release or target/release.' }
    return $candidate
}

function Get-ProcessMetricsSnapshot {
    param([Parameter(Mandatory)] [System.Diagnostics.Process]$Process)
    [pscustomobject]@{
        privateBytes = $Process.PrivateMemorySize64
        workingSet = $Process.WorkingSet64
        threadCount = $Process.Threads.Count
        handleCount = $Process.HandleCount
    }
}

function Get-ProcessCpuDelta {
    param(
        [Parameter(Mandatory)] [System.Diagnostics.Process]$Process,
        [Parameter(Mandatory)] [int]$IntervalMilliseconds
    )
    $startCpu = $Process.TotalProcessorTime
    $startAt = Get-Date
    Start-Sleep -Milliseconds $IntervalMilliseconds
    $Process.Refresh()
    $endCpu = $Process.TotalProcessorTime
    $elapsed = (Get-Date) - $startAt
    [pscustomobject]@{
        cpuDeltaMs = [math]::Max(0, ($endCpu - $startCpu).TotalMilliseconds)
        sampleElapsedMs = [math]::Max(1, [math]::Round($elapsed.TotalMilliseconds))
    }
}

function Get-LogicalProcessorCount {
    try { return [int]$env:NUMBER_OF_PROCESSORS } catch { return [Environment]::ProcessorCount }
}

function Get-ScenarioCpuSample {
    param(
        [Parameter(Mandatory)] [System.Diagnostics.Process]$Process,
        [Parameter(Mandatory)] [int]$IntervalMilliseconds
    )
    $logicalProcessors = [math]::Max(1, (Get-LogicalProcessorCount))
    $sample = Get-ProcessCpuDelta -Process $Process -IntervalMilliseconds $IntervalMilliseconds
    $cpuPercent = [math]::Round((($sample.cpuDeltaMs / [double]$sample.sampleElapsedMs) / $logicalProcessors) * 100, 2)
    [pscustomobject]@{
        cpuDeltaMs = $sample.cpuDeltaMs
        sampleElapsedMs = $sample.sampleElapsedMs
        cpuPercent = $cpuPercent
    }
}

function Stop-ProcessTree {
    param([Parameter(Mandatory)] [int]$RootProcessId)
    $children = Get-CimInstance Win32_Process -Filter "ParentProcessId=$RootProcessId" -ErrorAction SilentlyContinue
    foreach ($child in @($children)) { Stop-ProcessTree -RootProcessId $child.ProcessId }
    Stop-Process -Id $RootProcessId -Force -ErrorAction SilentlyContinue
}

function Get-ProcessTreeProcessIds {
    param([Parameter(Mandatory)] [int]$RootProcessId)
    $seen = New-Object 'System.Collections.Generic.HashSet[int]'
    $queue = New-Object 'System.Collections.Generic.Queue[int]'
    $queue.Enqueue($RootProcessId)
    while ($queue.Count -gt 0) {
        $current = $queue.Dequeue()
        if (-not $seen.Add($current)) { continue }
        $children = Get-CimInstance Win32_Process -Filter "ParentProcessId=$current" -ErrorAction SilentlyContinue
        foreach ($child in @($children)) { $queue.Enqueue([int]$child.ProcessId) }
    }
    return @($seen)
}

function Wait-ProcessReadyWithinDeadline {
    param(
        [Parameter(Mandatory)] [System.Diagnostics.Process]$Process,
        [Parameter(Mandatory)] [datetime]$Deadline,
        [int]$PollMilliseconds = 100
    )
    while (-not $Process.HasExited -and (Get-Date) -lt $Deadline) {
        Start-Sleep -Milliseconds $PollMilliseconds
        $Process.Refresh()
        if (-not $Process.HasExited) { return $true }
    }
    return -not $Process.HasExited
}

function Confirm-ScenarioPrereq {
    param([Parameter(Mandatory)] [string]$Scenario, [Parameter(Mandatory)] [string]$Statement)
    Write-Host $Statement
    $reply = Read-Host "Continue with $Scenario? [y/N]"
    return $reply -match '^(y|yes)$'
}

function Assert-RequiredScenarios {
    param([string[]]$Requested)
    $required = @('cold-idle','20+-windows','notifications','noisy-quick-commands-output','terminal-hidden-prewarm','fullscreen','multi-monitor')
    if (-not [System.Linq.Enumerable]::SequenceEqual([string[]]$Requested, [string[]]$required)) {
        throw "Scenario matrix must match Plan 01 required names exactly and in order."
    }
}

function Invoke-ScenarioPrereqCheck {
    param([Parameter(Mandatory)] [string]$Scenario)
    switch ($Scenario) {
        'cold-idle' { return @{ ok = $true; reason = $null } }
        '20+-windows' {
            if ($NonInteractive) { return @{ ok = $false; reason = 'blocked: complex manual setup requires operator confirmation'; manual = $true } }
            if (-not (Confirm-ScenarioPrereq -Scenario $Scenario -Statement 'Setup: create 20+ open windows on target desktop; do not launch JasonShell from this prompt.')) { return @{ ok = $false; reason = 'blocked: operator did not confirm 20+ windows setup'; manual = $true } }
            return @{ ok = $true; reason = $null }
        }
        'notifications' {
            if ($NonInteractive) { return @{ ok = $false; reason = 'blocked: complex manual setup requires operator confirmation'; manual = $true } }
            if (-not (Confirm-ScenarioPrereq -Scenario $Scenario -Statement 'Setup: generate the notification workload on the host, then confirm readiness.')) { return @{ ok = $false; reason = 'blocked: operator did not confirm notification setup'; manual = $true } }
            return @{ ok = $true; reason = $null }
        }
        'noisy-quick-commands-output' {
            if ($NonInteractive) { return @{ ok = $false; reason = 'blocked: complex manual setup requires operator confirmation'; manual = $true } }
            if (-not (Confirm-ScenarioPrereq -Scenario $Scenario -Statement 'Setup: create noisy Quick Commands output, then confirm readiness.')) { return @{ ok = $false; reason = 'blocked: operator did not confirm Quick Commands setup'; manual = $true } }
            return @{ ok = $true; reason = $null }
        }
        'terminal-hidden-prewarm' {
            if ($NonInteractive) { return @{ ok = $false; reason = 'blocked: complex manual setup requires operator confirmation'; manual = $true } }
            if (-not (Confirm-ScenarioPrereq -Scenario $Scenario -Statement 'Setup: hide terminal panel and prewarm the hidden session, then confirm readiness.')) { return @{ ok = $false; reason = 'blocked: operator did not confirm terminal prewarm setup'; manual = $true } }
            return @{ ok = $true; reason = $null }
        }
        'fullscreen' {
            if ($NonInteractive) { return @{ ok = $false; reason = 'blocked: complex manual setup requires operator confirmation'; manual = $true } }
            if (-not (Confirm-ScenarioPrereq -Scenario $Scenario -Statement 'Setup: enter a fullscreen app/window state, then confirm readiness.')) { return @{ ok = $false; reason = 'blocked: operator did not confirm fullscreen setup'; manual = $true } }
            return @{ ok = $true; reason = $null }
        }
        'multi-monitor' {
            if ($NonInteractive) { return @{ ok = $false; reason = 'blocked: complex manual setup requires operator confirmation'; manual = $true } }
            if (-not (Confirm-ScenarioPrereq -Scenario $Scenario -Statement 'Setup: ensure multiple monitors are connected and active, then confirm readiness.')) { return @{ ok = $false; reason = 'blocked: operator did not confirm multi-monitor setup'; manual = $true } }
            return @{ ok = $true; reason = $null }
        }
        default { throw "Unsupported scenario: $Scenario" }
    }
}

function New-ResultRecord {
    param([string]$Scenario, [int]$Attempt)
    $record = New-ScenarioResultSchema -SchemaVersion 'performance-baseline-contract/v2'
    $record.startedAt = (Get-Date).ToString('o')
    $record.scenario.name = $Scenario
    $record.scenario.metadata = @{ attempt = $Attempt; totalAttempts = $RunsPerScenario; modeLabel = $Mode; releaseAcceptance = ($Mode -eq 'release') }
    $record.scenario.status = 'running'
    return $record
}

$RequiredScenarios = @(
    'cold-idle',
    '20+-windows',
    'notifications',
    'noisy-quick-commands-output',
    'terminal-hidden-prewarm',
    'fullscreen',
    'multi-monitor'
)

Assert-RequiredScenarios -Requested $Scenarios

$timestamp = Get-DateStamp
$outputDir = Join-Path $OutputRoot $timestamp
if (Test-Path -LiteralPath $outputDir) { throw "Output directory collision: $outputDir" }
New-Item -ItemType Directory -Path $outputDir | Out-Null

$releaseBinary = $null
if ($Mode -eq 'release') { $releaseBinary = Resolve-ReleaseBinary }

$results = @()
foreach ($scenario in $Scenarios) {
    $attempts = @()
    1..3 | ForEach-Object {
        $i = $_
        $artifactPath = Join-Path $outputDir "run-$scenario-$i.json"
        $record = New-ResultRecord -Scenario $scenario -Attempt $i
        try {
            $prereq = Invoke-ScenarioPrereqCheck -Scenario $scenario
            $record.controlIo = if ($ControlIoProbe) { @{ available = $false; readCount = $null; writeCount = $null; note = 'probe unavailable; not fabricated' } } else { @{ available = $false; readCount = $null; writeCount = $null; note = 'not measured' } }
            if (-not $prereq.ok) {
                $record.scenario.status = 'blocked'
                $record.scenario.reason = $prereq.reason
            } else {
                $proc = $null
                try {
                    $proc = if ($Mode -eq 'release') { Start-Process -FilePath $releaseBinary.FullName -PassThru } else { Start-Process -FilePath 'npm' -ArgumentList @('run','dev') -PassThru } # readiness settle -> sample live process; Win32_Process tree tracked for dev cleanup
                    $launchDeadline = (Get-Date).AddSeconds([math]::Min(30, [math]::Max(5, [int]([math]::Floor($ScenarioTimeoutSeconds / 6)))))
                    $readinessDeadline = (Get-Date).AddSeconds([math]::Max(3, [math]::Min(15, [int]([math]::Floor($ScenarioTimeoutSeconds / 10)))))
                    $sampleDeadline = (Get-Date).AddSeconds($ScenarioTimeoutSeconds)
                    $devChildProcessIds = @()
                    if ($Mode -eq 'dev') { $devChildProcessIds = Get-ProcessTreeProcessIds -RootProcessId $proc.Id }
                    if (-not (Wait-ProcessReadyWithinDeadline -Process $proc -Deadline $launchDeadline)) { $record.scenario.status = 'error'; throw 'process exited before launch deadline' }
                    if ($proc.HasExited) { $record.scenario.status = 'error'; throw 'process exited before readiness wait' }
                    $settled = Wait-ProcessReadyWithinDeadline -Process $proc -Deadline $readinessDeadline
                    if (-not $settled) { $record.scenario.status = 'error'; throw 'readiness settle not reached before sample window' }
                    if ($proc.HasExited) { $record.scenario.status = 'error'; throw 'process exited before live sampling' }
                    if ((Get-Date) -ge $sampleDeadline) { $record.scenario.status = 'error'; throw "timeout after $ScenarioTimeoutSeconds seconds" }
                    $cpuDelta = Get-ProcessCpuDelta -Process $proc -IntervalMilliseconds 1000 # live CPU sample before exit
                    $logicalProcessors = [math]::Max(1, (Get-LogicalProcessorCount))
                    $cpu = [pscustomobject]@{ cpuDeltaMs = $cpuDelta.cpuDeltaMs; sampleElapsedMs = $cpuDelta.sampleElapsedMs; cpuPercent = [math]::Round((($cpuDelta.cpuDeltaMs / [double]$cpuDelta.sampleElapsedMs) / $logicalProcessors) * 100, 2) }
                    if ((Get-Date) -ge $sampleDeadline) { $record.scenario.status = 'error'; throw 'sample deadline exceeded' }
                    Wait-Process -Id $proc.Id -Timeout 0 -ErrorAction SilentlyContinue
                    $proc.Refresh()
                    $metrics = Get-ProcessMetricsSnapshot -Process $proc
                    $record.processMetrics.cpuDeltaMs = $cpu.cpuDeltaMs
                    $record.processMetrics.sampleElapsedMs = $cpu.sampleElapsedMs
                    $record.processMetrics.cpuPercent = $cpu.cpuPercent
                    $record.processMetrics.cpu = $cpu.cpuPercent
                    $record.processMetrics.privateBytes = $metrics.privateBytes
                    $record.processMetrics.workingSet = $metrics.workingSet
                    $record.processMetrics.threadCount = $metrics.threadCount
                    $record.processMetrics.handleCount = $metrics.handleCount
                    if ($metrics -and $cpu -and $cpu.sampleElapsedMs -gt 0 -and -not $proc.HasExited) { $record.scenario.status = 'pass' } else { $record.scenario.status = 'error'; $record.scenario.reason = 'measurement unavailable' }
                } finally {
                    if ($proc) {
                        if ($Mode -eq 'dev') { $devChildProcessIds = @(Get-ProcessTreeProcessIds -RootProcessId $proc.Id) }
                        if ($Mode -eq 'dev' -and $devChildProcessIds) { foreach ($childPid in @($devChildProcessIds | Sort-Object -Descending)) { Stop-Process -Id $childPid -Force -ErrorAction SilentlyContinue } }
                        Stop-ProcessTree -RootProcessId $proc.Id # Win32_Process tree cleanup
                    }
                }
            }
        } catch {
            $record.scenario.status = 'error'
            $record.error = @{ message = $_.Exception.Message }
        } finally {
            $record.finishedAt = (Get-Date).ToString('o')
            Write-JsonArtifact -Path $artifactPath -Value $record
        }
        $attempts += $record
    }

    $successes = @($attempts | Where-Object { $_.scenario.status -eq 'pass' })
    $results += [pscustomobject]@{
        scenario = $scenario
        mode = $Mode
        runs = $attempts
        medians = @{ cpu = Get-Median (@($successes | ForEach-Object { $_.processMetrics.cpuPercent })); privateBytes = Get-Median (@($successes | ForEach-Object { $_.processMetrics.privateBytes })); workingSet = Get-Median (@($successes | ForEach-Object { $_.processMetrics.workingSet })); threadCount = Get-Median (@($successes | ForEach-Object { $_.processMetrics.threadCount })); handleCount = Get-Median (@($successes | ForEach-Object { $_.processMetrics.handleCount })) }
        controlIo = if ($successes.Count -gt 0) { $successes[0].controlIo } else { @{ available = $false; readCount = $null; writeCount = $null; note = 'not measured' } }
        budgetPolicy = 'measured-baseline-relative non-regression only'
    }
}

$allRuns = @($results | ForEach-Object { $_.runs } | ForEach-Object { $_ })
$passCount = @($allRuns | Where-Object { $_.scenario.status -eq 'pass' }).Count
$blockedCount = @($allRuns | Where-Object { $_.scenario.status -eq 'blocked' }).Count
$errorCount = @($allRuns | Where-Object { $_.scenario.status -eq 'error' }).Count
$releaseAccepted = ($Mode -eq 'release' -and $blockedCount -eq 0 -and $errorCount -eq 0 -and @($allRuns | Where-Object { $_.scenario.status -eq 'pass' }).Count -eq ($RunsPerScenario * $Scenarios.Count))

$summaryPath = Join-Path $outputDir 'summary.md'
$riskPath = Join-Path $outputDir 'residual-risk.md'
$summary = @(
    "# Performance Baseline Summary - $timestamp",
    '',
    '## Mode separation',
    "- Release acceptance: $releaseAccepted",
    "- Dev diagnostic: $([bool]($Mode -eq 'dev'))",
    "- passCount=$passCount blockedCount=$blockedCount errorCount=$errorCount",
    '',
    '## Scenario medians (measured successes only)',
    ($results | ForEach-Object { "- $($_.mode) / $($_.scenario): cpu=$($_.medians.cpu) privateBytes=$($_.medians.privateBytes) workingSet=$($_.medians.workingSet) threadCount=$($_.medians.threadCount) handleCount=$($_.medians.handleCount) controlIo.available=$($_.controlIo.available)" }),
    '',
    '## Budget policy',
    '- measured-baseline-relative non-regression only',
    '',
    '## Release evidence note',
    '- release evidence for acceptance cannot be replaced by dev diagnostic evidence',
    '- release accepted only if all 3 successful measured runs exist for every required scenario'
)
$summary | Set-Content -LiteralPath $summaryPath -Encoding utf8

$risk = @(
    '# Residual Risk',
    '',
    '- notifications may be blocked or not measured when prereq unavailable',
    '- fullscreen may be blocked or not measured when prereq unavailable',
    '- multi-monitor may be blocked or not measured when prereq unavailable',
    '- control I/O availability is not fabricated when unavailable',
    '- diagnostic output is not release-evidence acceptance evidence'
)
$risk | Set-Content -LiteralPath $riskPath -Encoding utf8

exit 0
