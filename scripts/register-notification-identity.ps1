param(
  [string]$ExecutablePath = (Join-Path $PSScriptRoot '..\src-tauri\target\debug\jason-shell.exe')
)

if ($PSVersionTable.PSEdition -eq 'Core') {
  & "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ExecutionPolicy Bypass -File $PSCommandPath -ExecutablePath $ExecutablePath
  exit $LASTEXITCODE
}

$ErrorActionPreference = 'Stop'
$packageName = 'JasonShell.Dev'
$publisher = 'CN=JasonShell Dev'
$packageRoot = Join-Path $PSScriptRoot '..\.local\notification-package'
$manifestPath = Join-Path $packageRoot 'AppxManifest.xml'

if (-not (Test-Path -LiteralPath $ExecutablePath)) {
  throw "JasonShell executable missing: $ExecutablePath. Run cargo build --manifest-path src-tauri/Cargo.toml first."
}

$certificate = Get-ChildItem Cert:\CurrentUser\My |
  Where-Object { $_.Subject -eq $publisher -and $_.FriendlyName -eq 'JasonShell notification development identity' } |
  Select-Object -First 1
if (-not $certificate) {
  $certificate = New-SelfSignedCertificate -Type Custom -Subject $publisher -FriendlyName 'JasonShell notification development identity' -CertStoreLocation Cert:\CurrentUser\My -KeyUsage DigitalSignature -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3')
}
if (-not (Get-ChildItem Cert:\CurrentUser\TrustedPeople | Where-Object { $_.Thumbprint -eq $certificate.Thumbprint })) {
  $certificateFile = Join-Path $env:TEMP "$($certificate.Thumbprint).cer"
  Export-Certificate -Cert $certificate -FilePath $certificateFile | Out-Null
  Import-Certificate -FilePath $certificateFile -CertStoreLocation Cert:\CurrentUser\TrustedPeople | Out-Null
  Remove-Item -LiteralPath $certificateFile -Force
}

New-Item -ItemType Directory -Force -Path $packageRoot | Out-Null
$executableName = [System.IO.Path]::GetFileName($ExecutablePath)
$packagedExecutablePath = Join-Path $packageRoot $executableName
Copy-Item -LiteralPath $ExecutablePath -Destination $packagedExecutablePath -Force
$manifest = @"
<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10" xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10" xmlns:desktop="http://schemas.microsoft.com/appx/manifest/desktop/windows10" xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities" IgnorableNamespaces="uap desktop rescap">
  <Identity Name="$packageName" Publisher="$publisher" Version="1.0.0.0" ProcessorArchitecture="x64" />
  <Properties>
    <DisplayName>JasonShell</DisplayName>
    <PublisherDisplayName>JasonShell</PublisherDisplayName>
    <Logo>Assets\StoreLogo.png</Logo>
  </Properties>
  <Resources><Resource Language="en-us" /></Resources>
  <Dependencies><TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.17763.0" MaxVersionTested="10.0.26100.0" /></Dependencies>
  <Applications>
    <Application Id="JasonShell" Executable="$executableName" EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements DisplayName="JasonShell" Description="JasonShell notification development identity" BackgroundColor="transparent" Square150x150Logo="Assets\Square150x150Logo.png" Square44x44Logo="Assets\Square44x44Logo.png" />
      <Extensions><desktop:Extension Category="windows.fullTrustProcess" Executable="$executableName" /></Extensions>
    </Application>
  </Applications>
  <Capabilities><rescap:Capability Name="runFullTrust" /></Capabilities>
</Package>
"@
Set-Content -LiteralPath $manifestPath -Value $manifest -Encoding utf8

$assetDirectory = Join-Path $packageRoot 'Assets'
New-Item -ItemType Directory -Force -Path $assetDirectory | Out-Null
Copy-Item -LiteralPath (Join-Path $PSScriptRoot '..\src-tauri\icons\icon.png') -Destination (Join-Path $assetDirectory 'StoreLogo.png') -Force
Copy-Item -LiteralPath (Join-Path $PSScriptRoot '..\src-tauri\icons\icon.png') -Destination (Join-Path $assetDirectory 'Square150x150Logo.png') -Force
Copy-Item -LiteralPath (Join-Path $PSScriptRoot '..\src-tauri\icons\icon.png') -Destination (Join-Path $assetDirectory 'Square44x44Logo.png') -Force

Get-AppxPackage -Name $packageName | Remove-AppxPackage -ErrorAction SilentlyContinue
Add-AppxPackage -Register $manifestPath

$registered = Get-AppxPackage -Name $packageName
Start-Process "shell:AppsFolder\$($registered.PackageFamilyName)!JasonShell"
