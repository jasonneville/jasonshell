param([string]$Output = "$env:LOCALAPPDATA\Temp\opencode\seven-hotkeys-browser-e2e")
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $Output | Out-Null
# Start Vite separately with the README command. No Tauri/native shell launch.
agent-browser --session seven-hotkeys open http://127.0.0.1:4187
if ($LASTEXITCODE -ne 0) { throw 'Browser fixture could not open' }
agent-browser --session seven-hotkeys wait --fn '!!window.__runSeven && !!document.querySelector("#settings-panel").contentDocument.querySelector("[aria-label=\"Capture Search shortcut\"]")'
agent-browser --session seven-hotkeys screenshot "$Output\01-mounted.png"
agent-browser --session seven-hotkeys eval 'window.__runSeven().then(e=>({status:e.status,checks:e.checks,error:e.error}))' | Set-Content -Encoding utf8 "$Output\summary.json"
agent-browser --session seven-hotkeys eval 'window.__sevenEvidence' | Set-Content -Encoding utf8 "$Output\evidence.json"
agent-browser --session seven-hotkeys snapshot -i | Set-Content -Encoding utf8 "$Output\accessibility.txt"
agent-browser --session seven-hotkeys screenshot "$Output\02-completed.png"
agent-browser --session seven-hotkeys eval 'document.querySelector("#settings-panel").contentDocument.querySelector("#hotkeys-heading").scrollIntoView()'
agent-browser --session seven-hotkeys screenshot "$Output\03-shortcuts.png"
$result = agent-browser --session seven-hotkeys eval 'window.__sevenEvidence.status'
if ($result -notmatch 'PASS') { throw "Hotkey journey failed. See $Output" }
$summary = Get-Content -Raw "$Output\summary.json" | ConvertFrom-Json
$cases = @($summary.checks | ForEach-Object {
  $name = [System.Security.SecurityElement]::Escape([string]$_)
  "<testcase classname=`"seven-hotkeys.mocked-production-journey`" name=`"$name`" />"
}) -join "`n"
"<?xml version=`"1.0`" encoding=`"utf-8`"?><testsuite name=`"Seven hotkeys mocked production journey`" tests=`"$($summary.checks.Count)`" failures=`"0`">$cases</testsuite>" | Set-Content -Encoding utf8 "$Output\junit.xml"
$checks = @($summary.checks | ForEach-Object { '<li>' + [System.Net.WebUtility]::HtmlEncode([string]$_) + '</li>' }) -join ''
"<!doctype html><html lang=`"en`"><meta charset=`"utf-8`"><title>Seven hotkeys mocked journey PASS</title><h1>PASS: mocked production-mounted journey</h1><p>Not live Windows shell/OS delivery evidence.</p><ol>$checks</ol><p><a href=`"evidence.json`">IPC/event trace</a> · <a href=`"accessibility.txt`">Accessibility snapshot</a> · <a href=`"junit.xml`">JUnit</a></p><img width=`"100%`" src=`"01-mounted.png`" alt=`"Mounted production surfaces`"><img width=`"100%`" src=`"02-completed.png`" alt=`"Completed journey`"><img width=`"100%`" src=`"03-shortcuts.png`" alt=`"Committed seven shortcut bindings`"></html>" | Set-Content -Encoding utf8 "$Output\report.html"
Write-Output "PASS: artifacts in $Output"
agent-browser --session seven-hotkeys close
