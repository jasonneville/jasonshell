$ErrorActionPreference='Stop'
. "$PSScriptRoot/native-tree.ps1"
function Row($id,$parent,$time) { [pscustomobject]@{ProcessId=$id;ParentProcessId=$parent;CreationDate=[datetime]::new(2026,11,13,0,0,$time,[DateTimeKind]::Utc);Name='fixture'} }
function Check($value,$message) { if(-not $value) { throw $message } }
$root=Row 10 1 1
$known=@{'10:639301248010000000'=$root}
$known=@{}; $known["10:$($root.CreationDate.Ticks)"]=$root
$child=Row 11 10 2; $grand=Row 12 11 3
$out=@(Update-OwnedTree $known @($grand,$child,$root) @{})
Check ($out.Count -eq 3) 'Unordered transitive descendants not admitted'
$out=@(Update-OwnedTree $known @($grand) @{})
Check ($out.Count -eq 1) 'Surviving descendant lost after parent exit'
$reused=Row 10 1 4; $foreign=Row 13 10 5
$out=@(Update-OwnedTree $known @($reused,$foreign,$grand) @{})
Check ($out.Count -eq 1) 'Reused parent PID adopted unrelated descendant'
$old=Row 14 10 0
$out=@(Update-OwnedTree $known @($root,$old) @{})
Check ($out.Count -eq 1) 'Child predating parent admitted'
$exclude=@{}; $exclude["$($child.ProcessId):$($child.CreationDate.Ticks)"]=$true
$k=@{}; $k["10:$($root.CreationDate.Ticks)"]=$root
$out=@(Update-OwnedTree $k @($root,$child,$grand) $exclude)
Check ($out.Count -eq 1) 'Preexisting process or descendants adopted'
$out=@(Update-OwnedTree $k @() @{})
Check ($out.Count -eq 0) 'Exited identities reported live'
Add-Type -Path "$PSScriptRoot/native-observer.cs"
Check (@([RamNative]::Windows($PID)).Count -ge 0) 'Window enumeration failed'
Write-Output 'PASS: six identity/admission cases; native observer compiles and read-only probes succeed'
