<#
.SYNOPSIS
Plan 13 runtime smoke harness.

.DESCRIPTION
Default dry-run only. No live Tauri launch, no desktop mutation, no process termination.
#>
[CmdletBinding()]
param(
    [switch]$DryRun = $true,
    [switch]$ConsentDesktopMutation,
    [switch]$ConsentGlobalHooks,
    [switch]$ConsentProcessTermination,
    [string]$ManualEvidenceFile,
    [string]$Notes = '',
    [string]$ResultsRoot = (Join-Path $PSScriptRoot '..\test-results\runtime-smoke')
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function New-EvidenceStatus {
    param([string]$Name, [string]$Classification, [string]$Status, [string[]]$Notes)
    [pscustomobject]@{ name = $Name; classification = $Classification; status = $Status; notes = $Notes }
}

function Get-AllowlistedRuntimeData {
    [ordered]@{
        os = [System.Environment]::OSVersion.VersionString
        powershell = $PSVersionTable.PSVersion.ToString()
        isWindows = [bool]$IsWindows
        nodeAvailable = [bool](Get-Command node -ErrorAction SilentlyContinue)
        npmAvailable = [bool](Get-Command npm -ErrorAction SilentlyContinue)
    }
}

$startedAt = Get-Date
$timestamp = $startedAt.ToString('yyyyMMdd-HHmmss-fff')
$runSuffix = [guid]::NewGuid().ToString('N').Substring(0, 8)
$runId = "runtime-smoke-$timestamp-$runSuffix"
$artifactRoot = Join-Path $ResultsRoot "$timestamp-$runSuffix"

New-Item -ItemType Directory -Force -Path $artifactRoot | Out-Null

$consent = [ordered]@{
    DryRun = [bool]$DryRun
    DesktopMutation = [bool]$ConsentDesktopMutation
    GlobalHooks = [bool]$ConsentGlobalHooks
    ProcessTermination = [bool]$ConsentProcessTermination
}

$requiredPaths = @('package.json', 'src-tauri\Cargo.toml', 'docs\smoke-test-windows.md')
$missingPaths = @($requiredPaths | Where-Object { -not (Test-Path -LiteralPath (Join-Path $PSScriptRoot "..\$_")) })
$runtime = Get-AllowlistedRuntimeData
$preflightPassed = $missingPaths.Count -eq 0 -and $runtime.nodeAvailable -and $runtime.npmAvailable
$preflightNotes = @()
if ($missingPaths.Count -gt 0) { $preflightNotes += "Missing repo paths: $($missingPaths -join ', ')" }
if (-not $runtime.nodeAvailable) { $preflightNotes += 'node unavailable' }
if (-not $runtime.npmAvailable) { $preflightNotes += 'npm unavailable' }
if ($preflightNotes.Count -eq 0) { $preflightNotes += 'Required repo paths, node, and npm detected' }

$statuses = @(
    (New-EvidenceStatus 'preflight' 'automated' $(if ($preflightPassed) { 'passed' } else { 'failed' }) $preflightNotes)
    (New-EvidenceStatus 'desktop-mutation' 'blocked' 'blocked' @('Dry-run enforced; live mutation not implemented'))
    (New-EvidenceStatus 'global-hooks' 'blocked' 'blocked' @('Dry-run enforced; live hook checks not implemented'))
    (New-EvidenceStatus 'process-termination' 'blocked' 'blocked' @('Dry-run enforced; live termination checks not implemented'))
    (New-EvidenceStatus 'at-dpi-multi-monitor' 'manual' 'not-run' @('Requires direct human observation and manual evidence'))
)

$blocked = @()
if (-not $DryRun) { $blocked += 'DryRun=false live smoke not implemented' }
if (-not $ConsentDesktopMutation) { $blocked += 'ConsentDesktopMutation missing' }
if (-not $ConsentGlobalHooks) { $blocked += 'ConsentGlobalHooks missing' }
if (-not $ConsentProcessTermination) { $blocked += 'ConsentProcessTermination missing' }

$commandParts = @('scripts/runtime-smoke.ps1', "-DryRun:$([bool]$DryRun)")
if ($ConsentDesktopMutation) { $commandParts += '-ConsentDesktopMutation' }
if ($ConsentGlobalHooks) { $commandParts += '-ConsentGlobalHooks' }
if ($ConsentProcessTermination) { $commandParts += '-ConsentProcessTermination' }
$command = $commandParts -join ' '
$finishedAt = Get-Date

$evidence = [ordered]@{
    schemaVersion = 1
    runId = $runId
    mode = $(if ($DryRun) { 'dry-run' } else { 'live-requested' })
    overallStatus = $(if (-not $preflightPassed) { 'failed' } elseif ($blocked.Count -gt 0) { 'blocked' } else { 'passed' })
    startedAt = $startedAt.ToString('o')
    finishedAt = $finishedAt.ToString('o')
    command = $command
    artifactRoot = $artifactRoot
    allowlistedRuntime = $runtime
    consent = $consent
    skipped = @('Tauri launch', 'live desktop mutation', 'global hooks', 'process termination')
    statuses = $statuses
    blocked = $blocked
    notes = $Notes
    manualEvidenceFile = $(if ($ManualEvidenceFile) { $ManualEvidenceFile } else { $null })
}

$evidence | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $artifactRoot 'evidence.json') -Encoding utf8

$summary = @(
    '# Runtime Smoke Summary',
    '',
    "- Artifact root: $artifactRoot",
    "- Command: $command",
    "- DryRun: $DryRun",
    "- ConsentDesktopMutation: $ConsentDesktopMutation",
    "- ConsentGlobalHooks: $ConsentGlobalHooks",
    "- ConsentProcessTermination: $ConsentProcessTermination",
    "- Status: $($evidence.overallStatus.ToUpperInvariant())",
    '',
    '## Statuses',
    ($statuses | ForEach-Object { "- $($_.name): $($_.classification) / $($_.status) / $($_.notes -join '; ')" })
)
$summary | Set-Content -LiteralPath (Join-Path $artifactRoot 'summary.md') -Encoding utf8

$manualTemplate = @(
    '# Manual Evidence Template',
    '',
    '- Scope: AT / DPI / multi-monitor only',
    '- Record display scale, monitor layout, and exact manual steps',
    '- Do not claim assistive technology evidence without direct observation',
    '- Add screenshots/logs here'
)
$manualTemplate | Set-Content -LiteralPath (Join-Path $artifactRoot 'manual-evidence-template.md') -Encoding utf8

if (-not $DryRun) {
    throw 'Live runtime smoke disabled. DryRun only.'
}

Write-Host "Artifact root: $artifactRoot"
Write-Host ($blocked -join '; ')
