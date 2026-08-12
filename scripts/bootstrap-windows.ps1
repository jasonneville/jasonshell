param(
  [switch]$SkipInstall,
  [switch]$SkipDevLaunch
)

# powershell -ExecutionPolicy Bypass -File .\scripts\bootstrap-windows.ps1
$ErrorActionPreference = 'Stop'

function Write-BootstrapError {
  param([string]$Message)
  throw "JasonShell bootstrap failed: $Message"
}

function Assert-Command {
  param([string]$Name, [string]$FriendlyName)
  if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
    Write-BootstrapError "$FriendlyName missing. Install it or rerun bootstrap after setup."
  }
}

function Get-VsWherePath {
  $candidates = @(
    "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe",
    "$env:ProgramFiles\Microsoft Visual Studio\Installer\vswhere.exe"
  )

  foreach ($candidate in $candidates) {
    if (Test-Path -LiteralPath $candidate) { return $candidate }
  }

  return $null
}

function Get-VsDevCmdPath {
  $vsWhere = Get-VsWherePath
  if ($vsWhere) {
    $installPath = & $vsWhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($LASTEXITCODE -eq 0 -and $installPath) {
      $candidate = Join-Path $installPath 'Common7\Tools\VsDevCmd.bat'
      if (Test-Path -LiteralPath $candidate) { return $candidate }
    }
  }

  $standardPaths = @(
    "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat",
    "$env:ProgramFiles\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat"
  )

  foreach ($candidate in $standardPaths) {
    if (Test-Path -LiteralPath $candidate) { return $candidate }
  }

  return $null
}

function Import-VsDevEnv {
  $vsDevCmd = Get-VsDevCmdPath
  if (-not $vsDevCmd) {
    Write-BootstrapError 'VsDevCmd.bat missing. Install VS Build Tools with MSVC + Windows SDK, then rerun bootstrap.'
  }

  $tempFile = $null
  try {
    $tempFile = New-TemporaryFile
    $tempPath = $tempFile.FullName
    $cmd = "`"$vsDevCmd`" -arch=x64 -host_arch=x64 && set > `"$tempPath`""
    & cmd.exe /d /s /c $cmd | Out-Null
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $tempPath)) {
      Write-BootstrapError 'MSVC developer env import failed. Fix VS Build Tools or rerun after repair.'
    }

    Get-Content -LiteralPath $tempPath | ForEach-Object {
      if ($_ -match '^(.*?)=(.*)$') {
        [Environment]::SetEnvironmentVariable($matches[1], $matches[2], 'Process')
      }
    }
  } finally {
    if ($tempFile -and (Test-Path -LiteralPath $tempFile.FullName)) {
      Remove-Item -LiteralPath $tempFile.FullName -Force -ErrorAction SilentlyContinue
    }
  }
}

function Test-AdminRights {
  $current = [Security.Principal.WindowsIdentity]::GetCurrent()
  $principal = [Security.Principal.WindowsPrincipal]::new($current)
  return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Invoke-WingetInstall {
  param(
    [string]$Id,
    [string]$Name,
    [string[]]$ExtraArgs = @()
  )

  $args = @('install', '--id', $Id, '--exact', '--silent', '--accept-package-agreements', '--accept-source-agreements') + $ExtraArgs
  & winget @args
  if ($LASTEXITCODE -ne 0) {
    if ($LASTEXITCODE -eq 194 -or $LASTEXITCODE -eq 3010) {
      Write-BootstrapError "$Name install needs reboot. Reboot Windows, then rerun bootstrap."
    }
    Write-BootstrapError "$Name install failed via winget (id=$Id, exit=$LASTEXITCODE). Fix install then rerun."
  }
}

function Refresh-CurrentSessionPath {
  $machinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine')
  $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
  $merged = @($userPath, $machinePath) -join ';'
  $env:Path = $merged
}

function Test-WebView2RuntimeInstalled {
  $edgeUpdateClientGuid = '{F3017226-FE2A-4295-8BDF-00C3A9C2BB97}'
  $clientRoots = @(
    'HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients',
    'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients',
    'HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients',
    'HKCU:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients'
  )

  foreach ($root in $clientRoots) {
    $client = Join-Path $root $edgeUpdateClientGuid
    $pv = (Get-ItemProperty -LiteralPath $client -Name pv -ErrorAction SilentlyContinue).pv
    if ($pv) { return $true }
  }

  $keys = @(
    'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'HKCU:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*'
  )

  foreach ($key in $keys) {
    $match = Get-ItemProperty -Path $key -ErrorAction SilentlyContinue | Where-Object {
      $_.DisplayName -match 'Microsoft Edge WebView2 Runtime'
    } | Select-Object -First 1
    if ($match) { return $true }
  }

  return $false
}

function Ensure-Toolchain {
  if (-not $SkipInstall) {
    if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
      Write-BootstrapError 'winget missing. Install App Installer from Microsoft Store, then rerun bootstrap.'
    }

    $haveRustup = [bool](Get-Command rustup -ErrorAction SilentlyContinue)
    $haveRustc = [bool](Get-Command rustc -ErrorAction SilentlyContinue)
    $haveCargo = [bool](Get-Command cargo -ErrorAction SilentlyContinue)
    if (-not ($haveRustup -and $haveRustc -and $haveCargo)) {
      if (-not (Test-AdminRights)) {
        Write-BootstrapError 'Admin rights required for Rust toolchain install. Run PowerShell as Administrator or ask IT to allow winget / Rust policy.'
      }
      Invoke-WingetInstall -Id 'Rustlang.Rustup' -Name 'Rust toolchain'
    }

    if (-not ((Get-Command node -ErrorAction SilentlyContinue) -and (Get-Command npm -ErrorAction SilentlyContinue))) {
      if (-not (Test-AdminRights)) {
        Write-BootstrapError 'Admin rights required for Node.js/npm install. Run PowerShell as Administrator or ask IT to allow winget / Node policy.'
      }
      Invoke-WingetInstall -Id 'OpenJS.NodeJS.LTS' -Name 'Node.js LTS / npm'
    }

    if (-not (Get-VsDevCmdPath)) {
      if (-not (Test-AdminRights)) {
        Write-BootstrapError 'Admin rights required for VS Build Tools install. Run PowerShell as Administrator or ask IT to allow winget / VS Build Tools policy.'
      }
      Invoke-WingetInstall -Id 'Microsoft.VisualStudio.2022.BuildTools' -Name 'MSVC build tools' -ExtraArgs @('--override', '"--wait --quiet --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --add Microsoft.VisualStudio.Component.Windows10SDK.19041"')
    }

    if (-not (Test-WebView2RuntimeInstalled)) {
      if (-not (Test-AdminRights)) {
        Write-BootstrapError 'Admin rights required for WebView2 install. Run PowerShell as Administrator or ask IT to allow winget / WebView2 policy.'
      }
      Invoke-WingetInstall -Id 'Microsoft.EdgeWebView2Runtime' -Name 'WebView2 runtime'
    }
  }

  Refresh-CurrentSessionPath
  Assert-Command 'rustc' 'rustc'
  Assert-Command 'cargo' 'cargo'
  Assert-Command 'node' 'Node.js'
  Assert-Command 'npm' 'npm'

  if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    Write-BootstrapError 'rustup missing after install. Fix Rust install then rerun bootstrap.'
  }

  if (-not $SkipInstall) {
    & rustup toolchain install stable --profile default
    if ($LASTEXITCODE -ne 0) { Write-BootstrapError 'Rust stable toolchain install failed. Fix rustup then rerun.' }
    & rustup default stable-msvc
    if ($LASTEXITCODE -ne 0) { Write-BootstrapError 'Rust stable MSVC default failed. Fix rustup then rerun.' }
  }
  & rustup show active-toolchain
  if ($LASTEXITCODE -ne 0) { Write-BootstrapError 'Rust host/toolchain verification failed.' }

  Import-VsDevEnv
}

Set-Location $PSScriptRoot\..
Ensure-Toolchain

if (-not $SkipInstall) {
  npm ci
  if ($LASTEXITCODE -ne 0) {
    Write-BootstrapError 'repo dependency install failed. Fix lockfile or package manager state, then rerun.'
  }
}

if (-not $SkipDevLaunch) {
  npm run tauri dev
  if ($LASTEXITCODE -ne 0) {
    Write-BootstrapError 'dev launch failed. Check previous output for build/runtime errors.'
  }
}
