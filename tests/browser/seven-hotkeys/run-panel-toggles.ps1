param([string]$Output = "$env:LOCALAPPDATA\Temp\opencode\panel-toggles-browser-red")
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $Output | Out-Null
try {
  agent-browser --session panel-toggles open http://127.0.0.1:4187
  if ($LASTEXITCODE -ne 0) { throw 'Browser fixture could not open' }
  agent-browser --session panel-toggles wait --fn '!!window.__runPanelToggles && !!document.querySelector("#settings-panel").contentDocument.querySelector("[aria-label=\"Capture Search shortcut\"]")'
  agent-browser --session panel-toggles eval 'window.__runPanelToggles()' | Set-Content -Encoding utf8 "$Output\summary.json"
  agent-browser --session panel-toggles eval 'window.__toggleEvidence' | Set-Content -Encoding utf8 "$Output\evidence.json"
  agent-browser --session panel-toggles snapshot -i | Set-Content -Encoding utf8 "$Output\accessibility.txt"
  agent-browser --session panel-toggles screenshot "$Output\completed.png"
  $summary = Get-Content -Raw "$Output\summary.json" | ConvertFrom-Json
  $summary | ConvertTo-Json -Depth 10
  if ($summary.status -ne 'PASS') { throw "Panel toggle journey RED. See $Output" }
} finally { agent-browser --session panel-toggles close }
