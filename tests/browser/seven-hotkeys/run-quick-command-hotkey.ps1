param([string]$Output = "$env:LOCALAPPDATA\Temp\opencode\quick-command-hotkey-browser-red")
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $Output | Out-Null
try {
  agent-browser --session quick-command-hotkey open http://127.0.0.1:4187
  if ($LASTEXITCODE -ne 0) { throw 'Browser fixture could not open' }
  agent-browser --session quick-command-hotkey wait --fn '!!window.__runEight && !!document.querySelector("#settings-panel").contentDocument.querySelector("[aria-label=\"Capture Search shortcut\"]")'
  agent-browser --session quick-command-hotkey eval 'window.__runEight()' | Set-Content -Encoding utf8 "$Output\summary.json"
  agent-browser --session quick-command-hotkey eval 'window.__eightEvidence' | Set-Content -Encoding utf8 "$Output\evidence.json"
  agent-browser --session quick-command-hotkey snapshot -i | Set-Content -Encoding utf8 "$Output\accessibility.txt"
  agent-browser --session quick-command-hotkey screenshot "$Output\completed.png"
  $summary = Get-Content -Raw "$Output\summary.json" | ConvertFrom-Json
  $summary | ConvertTo-Json -Depth 10
  if ($summary.status -ne 'PASS') { throw "Quick Commands shortcut journey RED. See $Output" }
} finally { agent-browser --session quick-command-hotkey close }
