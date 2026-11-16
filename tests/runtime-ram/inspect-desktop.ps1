param([Parameter(Mandatory)][string]$Output,[ValidateRange(1,10000)][int]$Samples=1,[ValidateRange(100,1000)][int]$IntervalMs=200)
$ErrorActionPreference='Stop'
if(-not ('DesktopInspector' -as [type])) { Add-Type -Path "$PSScriptRoot/native-desktop-observer.cs" }
if(Test-Path $Output) { throw 'Refuse overwriting desktop evidence' }
$series=[Collections.Generic.List[object]]::new()
try {
 for($i=0;$i -lt $Samples;$i++) {
  $series.Add([DesktopInspector]::Read())
  if($i+1 -lt $Samples) { Start-Sleep -Milliseconds $IntervalMs }
 }
} catch {
 $series.Add([pscustomobject]@{Error=$_.Exception.Message;Utc=[datetime]::UtcNow})
 throw
} finally { $series.ToArray() | ConvertTo-Json -Depth 5 | Set-Content $Output }
Get-Content $Output
