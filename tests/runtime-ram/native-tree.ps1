# Pure admission rule: only a currently observed, identity-matched parent can
# introduce a child. Previously admitted identities remain tracked after reparenting.
function Update-OwnedTree {
 param([hashtable]$Known,[object[]]$Snapshot,[hashtable]$Excluded)
 $live=@{}
 foreach($row in $Snapshot) { $live[[string]$row.ProcessId]=$row }
 do {
  $changed=$false
  foreach($row in $Snapshot) {
   $key="$($row.ProcessId):$($row.CreationDate.ToUniversalTime().Ticks)"
   if($Known.ContainsKey($key) -or $Excluded.ContainsKey($key)) { continue }
   $parent=$live[[string]$row.ParentProcessId]
   if($null -eq $parent) { continue }
   $parentKey="$($parent.ProcessId):$($parent.CreationDate.ToUniversalTime().Ticks)"
   if($Known.ContainsKey($parentKey) -and $row.CreationDate -ge $parent.CreationDate) {
    $Known[$key]=$row; $changed=$true
   }
  }
 } while($changed)
 return @($Snapshot | Where-Object { $Known.ContainsKey("$($_.ProcessId):$($_.CreationDate.ToUniversalTime().Ticks)") })
}
