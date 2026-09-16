param(
  [Parameter(Mandatory = $true)]
  [ValidatePattern('^[A-Za-z0-9._-]+$')]
  [string]$RunId
)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$runDir = Join-Path $repo "test-results/stack-text-editor/P03/$RunId"
if (Test-Path -LiteralPath $runDir) { throw "Run directory already exists: $runDir" }
New-Item -ItemType Directory -Path $runDir | Out-Null
Copy-Item -LiteralPath (Join-Path $repo 'docs/stack-text-editor-p03-native-packet.md') -Destination (Join-Path $runDir 'review.md')

Push-Location $repo
try {
  npm exec vite build -- --config vite.p03-experiment.config.ts --outDir dist-p03-experiment --emptyOutDir
  if ($LASTEXITCODE -ne 0) { throw 'P03 frontend build failed' }
  cargo run --manifest-path src-tauri/Cargo.toml --example stack_text_p03 -- --p03-experiment $runDir
  if ($LASTEXITCODE -ne 0) { throw 'P03 native host failed' }
} finally {
  Pop-Location
}
