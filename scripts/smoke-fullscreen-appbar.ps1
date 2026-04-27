<#
.SYNOPSIS
Runs the live Windows fullscreen appbar smoke checklist for JasonShell.

.DESCRIPTION
This script is intentionally interactive because the behavior under test depends on
real Win32 foreground-window, browser fullscreen, and appbar behavior. It does not
send keys, force fullscreen, or close applications. It records pass/fail evidence
to a markdown file under smoke-results by default.

.PARAMETER BrowserUrl
URL opened in the default browser for the browser fullscreen path.

.PARAMETER ResultsDir
Directory where the markdown smoke-test result is written.

.PARAMETER SkipBrowserLaunch
Do not launch the default browser; use an already-open browser instead.

.EXAMPLE
npm run smoke:fullscreen

.EXAMPLE
npm run smoke:fullscreen -- -SkipBrowserLaunch
#>
[CmdletBinding()]
param(
    [string]$BrowserUrl = "https://example.com",
    [string]$ResultsDir = "smoke-results",
    [switch]$SkipBrowserLaunch
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Read-RequiredText {
    param(
        [string]$Prompt,
        [string]$Default = ""
    )

    $value = Read-Host $Prompt
    if ([string]::IsNullOrWhiteSpace($value)) {
        return $Default
    }

    return $value.Trim()
}

function Read-SmokeStep {
    param(
        [string]$Id,
        [string]$Title,
        [string]$Instruction,
        [string]$PassCriteria
    )

    Write-Host ""
    Write-Host "[$Id] $Title" -ForegroundColor Cyan
    Write-Host $Instruction
    Write-Host "PASS when: $PassCriteria" -ForegroundColor DarkGreen

    do {
        $status = (Read-Host "Result for ${Id}: [P]ass / [F]ail / [S]kip").Trim().ToUpperInvariant()
    } while ($status -notin @("P", "F", "S"))

    $notes = Read-RequiredText "Evidence notes, foreground app/window title, screenshot path, or log excerpt"

    $statusText = switch ($status) {
        "P" { "PASS" }
        "F" { "FAIL" }
        "S" { "SKIP" }
    }

    return [pscustomobject]@{
        Id = $Id
        Title = $Title
        Status = $statusText
        Notes = $notes
        PassCriteria = $PassCriteria
    }
}

if (-not $IsWindows) {
    throw "This smoke test must be run on Windows because it validates Win32 appbar behavior."
}

$startedAt = Get-Date
$timestamp = $startedAt.ToString("yyyyMMdd-HHmmss")
New-Item -ItemType Directory -Force -Path $ResultsDir | Out-Null
$resultPath = Join-Path $ResultsDir "fullscreen-appbar-$timestamp.md"

Write-Host "JasonShell live fullscreen appbar smoke test" -ForegroundColor Cyan
Write-Host "This checklist validates real browser/game fullscreen behavior against the top and bottom bars."
Write-Host "Start JasonShell first in another terminal with: npm run tauri dev"
Write-Host "Do not close apps for this test; use F11, Esc, Alt+Tab, or normal app controls."
Write-Host "Results will be written to: $resultPath"

$tester = Read-RequiredText "Tester name or initials" "UNSPECIFIED"
$buildRef = Read-RequiredText "Build/commit/ref under test" "UNSPECIFIED"
$displaySetup = Read-RequiredText "Display setup, scale, and primary monitor notes" "UNSPECIFIED"
$jasonShellLog = Read-RequiredText "JasonShell terminal log path or notable live metrics" "UNSPECIFIED"

$steps = @()

$steps += Read-SmokeStep `
    -Id "PRE-001" `
    -Title "JasonShell bars are visible before fullscreen" `
    -Instruction "Confirm JasonShell is running and both the top bar and bottom bar are visible on the primary display before opening a fullscreen app." `
    -PassCriteria "Top and bottom bars are visible and stable on the primary monitor."

if (-not $SkipBrowserLaunch) {
    Write-Host ""
    Write-Host "Opening default browser: $BrowserUrl" -ForegroundColor Cyan
    Start-Process $BrowserUrl
}

$steps += Read-SmokeStep `
    -Id "BRW-001" `
    -Title "Browser fullscreen hides JasonShell bars" `
    -Instruction "Focus Edge, Chrome, or the default browser. Enter fullscreen with F11, or use a video's fullscreen button if F11 is unavailable. Wait two seconds after the browser fills the primary monitor." `
    -PassCriteria "While the fullscreen browser is the foreground window, both JasonShell bars are hidden and the fullscreen content reaches the screen edges."

$steps += Read-SmokeStep `
    -Id "BRW-002" `
    -Title "Focus away from fullscreen browser restores bars" `
    -Instruction "Without exiting browser fullscreen, switch focus to a non-fullscreen window such as the JasonShell terminal, Explorer, or another app using Alt+Tab or the task switcher." `
    -PassCriteria "Top and bottom JasonShell bars reappear after focus leaves the fullscreen browser."

$steps += Read-SmokeStep `
    -Id "BRW-003" `
    -Title "Returning to fullscreen browser hides bars again" `
    -Instruction "Switch focus back to the still-fullscreen browser window." `
    -PassCriteria "Both JasonShell bars hide again while the fullscreen browser is foreground."

$steps += Read-SmokeStep `
    -Id "BRW-004" `
    -Title "Exiting browser fullscreen restores bars" `
    -Instruction "Exit browser fullscreen with F11 or Esc, then keep the browser focused." `
    -PassCriteria "Top and bottom JasonShell bars reappear and remain visible after fullscreen exits."

$steps += Read-SmokeStep `
    -Id "APP-001" `
    -Title "Optional fullscreen game or app path" `
    -Instruction "If a fullscreen game or exclusive/borderless fullscreen app is available, launch it, enter fullscreen on the primary display, then repeat: focused fullscreen hides bars, Alt+Tab/focus away restores bars, exiting fullscreen restores bars. Choose Skip if no suitable app is available." `
    -PassCriteria "A real fullscreen game/app hides both bars only while it is the foreground fullscreen window, and bars restore on focus-away or fullscreen exit."

$finishedAt = Get-Date
$failed = @($steps | Where-Object { $_.Status -eq "FAIL" })
$passed = @($steps | Where-Object { $_.Status -eq "PASS" })
$skipped = @($steps | Where-Object { $_.Status -eq "SKIP" })
$requiredIds = @("PRE-001", "BRW-001", "BRW-002", "BRW-003", "BRW-004")
$requiredNotPassed = @($steps | Where-Object { $requiredIds -contains $_.Id -and $_.Status -ne "PASS" })

$lines = @(
    "# Fullscreen Appbar Smoke Test - $timestamp",
    "",
    "## Environment",
    "- Started: $($startedAt.ToString('o'))",
    "- Finished: $($finishedAt.ToString('o'))",
    "- Tester: $tester",
    "- Build/ref: $buildRef",
    "- Machine: $env:COMPUTERNAME",
    "- OS: $([System.Environment]::OSVersion.VersionString)",
    "- PowerShell: $($PSVersionTable.PSVersion)",
    "- Display setup: $displaySetup",
    "- JasonShell log/metrics: $jasonShellLog",
    "",
    "## Summary",
    "- Pass: $($passed.Count)",
    "- Fail: $($failed.Count)",
    "- Skip: $($skipped.Count)",
    "",
    "## Results"
)

foreach ($step in $steps) {
    $lines += @(
        "",
        "### $($step.Id) - $($step.Title)",
        "- Status: $($step.Status)",
        "- Pass criteria: $($step.PassCriteria)",
        "- Evidence: $($step.Notes)"
    )
}

$lines += @(
    "",
    "## Pass/Fail Rule",
    "- Overall PASS requires PRE-001 and all browser steps BRW-001 through BRW-004 to pass.",
    "- APP-001 may be skipped when no fullscreen game/app is available.",
    "- Any FAIL means the live fullscreen appbar behavior needs investigation before release."
)

$lines | Out-File -FilePath $resultPath -Encoding utf8

Write-Host ""
Write-Host "Smoke test result written to $resultPath" -ForegroundColor Cyan
Write-Host "Pass: $($passed.Count)  Fail: $($failed.Count)  Skip: $($skipped.Count)"

if ($failed.Count -gt 0 -or $requiredNotPassed.Count -gt 0) {
    exit 1
}

exit 0
