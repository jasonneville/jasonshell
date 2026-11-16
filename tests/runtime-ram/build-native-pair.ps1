param([Parameter(Mandatory)][string]$Stage,[Parameter(Mandatory)][string]$Evidence)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path "$PSScriptRoot/../..").Path
$source=Join-Path $Stage source
$files=@('src/components/StackPopupSurface.svelte','src/components/TerminalPanelSurface.svelte','src/components/ProcessManagerSurface.svelte','src/components/StackGitPanel.svelte','src/lib/stackGitPanelState.ts','src/lib/stackIconCache.ts')
New-Item -ItemType Directory -Force $Evidence | Out-Null
$env:CARGO_TARGET_DIR='C:\dev\jasonshell\src-tauri\target'
$config=Join-Path $Stage 'measurement-config.json'
'{"identifier":"com.jnev1.jasonshell.rammeasurement20261113"}' | Set-Content $config
Push-Location $source
try {
 foreach($variant in @('baseline','current')) {
  if($variant -eq 'current') { foreach($file in $files) { Copy-Item (Join-Path $repo $file) (Join-Path $source $file) } }
  $hashes=@(foreach($file in $files) { if(Test-Path $file) { Get-FileHash $file -Algorithm SHA256 | Select-Object Path,Hash } })
  $hashes | ConvertTo-Json | Set-Content "$Evidence/$variant-source.json"
  $start=Get-Date
  & npm.cmd run tauri -- build --no-bundle --config $config *> "$Evidence/$variant-build.log"
  $code=$LASTEXITCODE
  @{command="npm run tauri -- build --no-bundle --config $config";started=$start.ToString('o');finished=(Get-Date).ToString('o');exitCode=$code;target=$env:CARGO_TARGET_DIR;source=$source;mode='release'} | ConvertTo-Json | Set-Content "$Evidence/$variant-build.json"
  if($code -ne 0) { throw "$variant build failed; see $Evidence/$variant-build.log" }
  $out=Join-Path $Stage $variant
  New-Item -ItemType Directory $out | Out-Null
  Copy-Item "$env:CARGO_TARGET_DIR/release/jason-shell.exe" "$out/jason-shell.exe"
  Get-ChildItem "$env:CARGO_TARGET_DIR/release/*.dll" | Copy-Item -Destination $out
  Get-ChildItem $out -File | Get-FileHash -Algorithm SHA256 | Select-Object Path,Hash | ConvertTo-Json | Set-Content "$Evidence/$variant-binary.json"
 }
} finally { Pop-Location }
