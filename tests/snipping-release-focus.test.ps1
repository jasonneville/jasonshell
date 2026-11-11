# Pure post-click baseline decision and ordering tests. No app/native execution.
$ErrorActionPreference='Stop'
$path=Join-Path $PSScriptRoot 'snipping-release-native-e2e.ps1'
$tokens=$null;$errors=$null
$ast=[System.Management.Automation.Language.Parser]::ParseFile($path,[ref]$tokens,[ref]$errors)
if($errors.Count){throw 'release-probe-parser-errors'}
foreach($name in @('Test-SnipIdentitySame','Test-SnipOwnedInventorySame','Test-SnipInventoryWithImeAdditions','Test-SnipPostClickBaseline','Get-SnipInventoryDelta')){
    $definition=@($ast.FindAll({param($n) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq $name},$true))
    if($definition.Count -ne 1){throw 'pure-focus-function-missing-or-ambiguous'}
    . ([scriptblock]::Create($definition[0].Extent.Text))
}
function Copy-Identity($row){[pscustomobject]@{Hwnd=$row.Hwnd;Root=$row.Root;Pid=$row.Pid;Thread=$row.Thread;Epoch=$row.Epoch;Live=$row.Live;Visible=$row.Visible}}
function New-Identity([long]$hwnd,[uint32]$owner,[long]$epoch,[bool]$visible=$true){[pscustomobject]@{Hwnd=$hwnd;Root=$hwnd;Pid=$owner;Thread=($owner*10+1);Epoch=$epoch;Live=$true;Visible=$visible}}
function New-Case {
    $bar=New-Identity 100 10 7
    $hidden=New-Identity 101 10 0 $false
    $fixture=New-Identity 200 20 0
    @{
        PreOwned=@((Copy-Identity $bar),(Copy-Identity $hidden))
        BeforeOwned=@((Copy-Identity $bar),(Copy-Identity $hidden))
        AfterOwned=@((Copy-Identity $bar),(Copy-Identity $hidden))
        PreFixture=(Copy-Identity $fixture);CurrentFixture=(Copy-Identity $fixture)
        PreBar=(Copy-Identity $bar);CurrentBar=(Copy-Identity $bar);Observed=(Copy-Identity $fixture)
        ProtectedBarHwnd=100;ProtectedBarEpoch=7;ShellPid=10;FixturePid=20
    }
}
$accepted=0;$rejected=0
foreach($variant in @('fixtureB','protectedbar','reordered-inventory')){
    $case=New-Case
    if($variant -eq 'protectedbar'){$case.Observed=Copy-Identity $case.PreBar}
    if($variant -eq 'reordered-inventory'){$case.BeforeOwned=@($case.BeforeOwned[1],$case.BeforeOwned[0])}
    $result=Test-SnipPostClickBaseline @case
    $kind=if($variant -eq 'protectedbar'){'protectedbar'}else{'fixtureB'}
    if(!$result.Accepted -or $result.Classification -ne $kind -or $result.Reason -ne 'accepted'){throw "approved-baseline-rejected:$variant"}
    $accepted++
}
$negative=@(
    @{Name='changed-observed-PID';Change={param($c) $c.Observed=Copy-Identity $c.PreBar;$c.Observed.Pid=99}},
    @{Name='zero-protected-epoch';Change={param($c) $c.Observed=Copy-Identity $c.PreBar;$c.ProtectedBarEpoch=0}},
    @{Name='changed-current-bar-epoch';Change={param($c) $c.Observed=Copy-Identity $c.PreBar;$c.CurrentBar.Epoch=8}},
    @{Name='zero-current-bar-epoch';Change={param($c) $c.Observed=Copy-Identity $c.PreBar;$c.CurrentBar.Epoch=0}},
    @{Name='retired-bar';Change={param($c) $c.Observed=Copy-Identity $c.PreBar;$c.CurrentBar.Live=$false}},
    @{Name='retired-fixture';Change={param($c) $c.Observed=Copy-Identity $c.PreBar;$c.CurrentFixture.Live=$false}},
    @{Name='retired-observed';Change={param($c) $c.Observed.Live=$false}},
    @{Name='other-owned-root';Change={param($c) $c.Observed=New-Identity 101 10 0}},
    @{Name='external-other';Change={param($c) $c.Observed=New-Identity 999 99 0}},
    @{Name='fixture-A-not-B';Change={param($c) $c.Observed=New-Identity 201 20 0}},
    @{Name='zero-foreground';Change={param($c) $c.Observed=New-Identity 0 0 0}},
    @{Name='changed-root';Change={param($c) $c.Observed.Root=999}},
    @{Name='changed-thread';Change={param($c) $c.CurrentBar.Thread=999}},
    @{Name='hidden-unbound-added-before';Change={param($c) $c.Observed=Copy-Identity $c.PreBar;$c.BeforeOwned+=New-Identity 300 10 0 $false;$c.AfterOwned+=New-Identity 300 10 0 $false}},
    @{Name='hidden-unbound-added-between';Change={param($c) $c.Observed=Copy-Identity $c.PreBar;$c.AfterOwned+=New-Identity 300 10 0 $false}},
    @{Name='hidden-unbound-before-then-retired';Change={param($c) $c.Observed=Copy-Identity $c.PreBar;$c.BeforeOwned+=New-Identity 300 10 0 $false}},
    @{Name='new-registered-overlay';Change={param($c) $c.Observed=Copy-Identity $c.PreBar;$c.AfterOwned+=New-Identity 300 10 2}},
    @{Name='root-retired-from-inventory';Change={param($c) $c.BeforeOwned=@($c.BeforeOwned[0])}},
    @{Name='inventory-PID-changed';Change={param($c) $c.AfterOwned[0].Pid=99}},
    @{Name='inventory-epoch-changed';Change={param($c) $c.BeforeOwned[0].Epoch=8}},
    @{Name='duplicate-inventory-root';Change={param($c) $c.AfterOwned[1]=Copy-Identity $c.AfterOwned[0]}},
    @{Name='fixture-no-longer-visible';Change={param($c) $c.CurrentFixture.Visible=$false}},
    @{Name='wrong-protected-bar';Change={param($c) $c.ProtectedBarHwnd=101}},
    @{Name='pre-bar-does-not-match-inventory';Change={param($c) $c.PreBar.Epoch=8;$c.CurrentBar.Epoch=8;$c.ProtectedBarEpoch=8}}
)
foreach($test in $negative){
    $case=New-Case
    & $test.Change $case
    $result=Test-SnipPostClickBaseline @case
    if($result.Accepted -or $result.Classification -ne 'inconclusive' -or $result.Reason -eq 'accepted'){throw "unsafe-baseline-accepted:$($test.Name)"}
    $rejected++
}
$source=$ast.Extent.Text
$ordering='\$beforeForegroundInventory=\[SnipReleaseProbe\]::AllOwnedIdentities\(\$shellPid\)\s+\$baselineTicks.BeforeEnd=\[System.Diagnostics.Stopwatch\]::GetTimestamp\(\)\s+\$beforeHelperFacts=\[SnipReleaseProbe\]::VerifyAddedRootClasses\([^\r\n]+\)\s+Release-InjectedLeft\s+\$postClickForeground=\[SnipReleaseProbe\]::GetForegroundWindow\(\)\s+\$baselineTicks.ForegroundEnd=\[System.Diagnostics.Stopwatch\]::GetTimestamp\(\)\s+\$postClickObserved=\[SnipReleaseProbe\]::Identity\(\$postClickForeground\)\s+\$baselineTicks.AfterStart=\[System.Diagnostics.Stopwatch\]::GetTimestamp\(\)\s+\$afterForegroundInventory=\[SnipReleaseProbe\]::AllOwnedIdentities\(\$shellPid\)'
if($source -notmatch $ordering){throw 'foreground-must-be-immediate-after-UP-inside-owned-inventory-bracket'}
$ownedBody=($source -split 'public static WindowIdentity\[\] AllOwnedIdentities',2)[1] -split 'public static IntPtr\[\] Registered',2 | Select-Object -First 1
if(!$ownedBody.Contains('EnumWindows') -or $ownedBody -match 'IsWindowVisible|GetPropW'){throw 'all-owned-inventory-must-not-filter-hidden-or-unbound-roots'}
if(!$source.Contains('Require ($foreground -eq $fixture) ''external-foreground-not-restored''') -or
   !$source.Contains('Require ($foreground -ne $preview) ''preview-activated''')){throw 'original-preview-foreground-contract-changed'}
if(!$source.Contains('RESULT INCONCLUSIVE stage=$stage') -or $source.Contains('[SnipReleaseProbe]::SetForegroundWindow(')){throw 'must-report-inconclusive-without-forcing-focus'}
$before=@((New-Identity 100 10 7),(New-Identity 101 10 0 $false),(New-Identity 102 10 9 $false))
$after=@((New-Identity 100 10 8),(New-Identity 103 10 0 $false))
$delta=@(Get-SnipInventoryDelta $before $after 'pre-to-beforeUP' 100)
if($delta.Count -ne 4 -or $delta[0].Change -ne 'identitychanged' -or $delta[0].ChangedMask -ne 8 -or $delta[0].OldRole -ne 'protectedbar' -or
   $delta[1].Change -ne 'retired' -or $delta[1].OldRole -ne 'unknown' -or $delta[1].Old.Pid -ne 10 -or $delta[1].Old.Thread -ne 101 -or $delta[1].Old.Visible -ne $false -or
   $delta[2].Change -ne 'retired' -or $delta[2].OldRole -ne 'registeredother' -or $delta[2].Old.Epoch -ne 9 -or
   $delta[3].Change -ne 'added' -or $delta[3].NewRole -ne 'unknown' -or $delta[3].New.Visible -ne $false -or $delta[3].New.Epoch -ne 0){throw 'numeric-inventory-delta-oracle-failed'}
$unchanged=@(Get-SnipInventoryDelta $before @($before[2],$before[1],$before[0]) 'beforeUP-to-afterFG' 100)
$visible=Copy-Identity $before[1];$visible.Visible=$true
$visibility=@(Get-SnipInventoryDelta @($before[1]) @($visible) 'beforeUP-to-afterFG' 100)
if($unchanged.Count -ne 0 -or $visibility.Count -ne 1 -or $visibility[0].ChangedMask -ne 32 -or $visibility[0].Change -ne 'visibilitychanged'){throw 'inventory-delta-order-visibility-oracle-failed'}
if(!$source.Contains('if(!$postClickBaseline.Accepted){Write-SnipBaselineFailure;throw') -or !$source.Contains('deltaDropped=$dropped')){throw 'diagnostics-must-be-failure-only-and-bounded'}
if($source -match 'ObserveInconclusiveProfile|Wait-SnipInconclusiveProfile|JASONSHELL_SNIP_PROFILE|profileTracePath|PASSIVE_PROFILE|jasonshell-snip-profile\.tsv|ReadAllLinesAsync'){
    throw 'temporary-profiler-observation-must-not-remain-in-acceptance-harness'
}
function New-ImeCase {
    $c=New-Case;$c.Observed=Copy-Identity $c.PreBar
    $helper=New-Identity 300 10 0 $false
    $c.BeforeOwned+=Copy-Identity $helper;$c.AfterOwned+=Copy-Identity $helper
    $first=Copy-Identity $helper;$first|Add-Member -NotePropertyName ClassCode -NotePropertyValue 2;$first|Add-Member -NotePropertyName ClassVerified -NotePropertyValue $true
    $last=Copy-Identity $helper;$last|Add-Member -NotePropertyName ClassCode -NotePropertyValue 2;$last|Add-Member -NotePropertyName ClassVerified -NotePropertyValue $true
    $c.BeforeHelpers=@($first);$c.AfterHelpers=@($last);return $c
}
$ime=New-ImeCase;$result=Test-SnipPostClickBaseline @ime
if(!$result.Accepted -or $result.Classification -ne 'protectedbar'){throw 'verified-hidden-same-thread-IME-helper-rejected'}
$imeNegative=@(
    {param($c) $c.BeforeHelpers[0].ClassCode=4},
    {param($c) $c.BeforeHelpers[0].ClassCode=0},
    {param($c) $c.BeforeHelpers[0].ClassVerified=$false},
    {param($c) $c.AfterHelpers[0].ClassVerified=$false},
    {param($c) $c.BeforeHelpers[0].Visible=$true},
    {param($c) $c.AfterHelpers[0].Visible=$true},
    {param($c) $c.BeforeHelpers[0].Pid=99},
    {param($c) $c.BeforeHelpers[0].Thread=999},
    {param($c) $c.BeforeHelpers[0].Epoch=9},
    {param($c) $c.BeforeHelpers[0].Root=999},
    {param($c) $c.BeforeHelpers[0].Live=$false},
    {param($c) $c.AfterHelpers[0].Live=$false},
    {param($c) $c.BeforeOwned=@($c.BeforeOwned[0],$c.BeforeOwned[1]);$c.BeforeHelpers=@()},
    {param($c) $c.AfterHelpers[0].ClassCode=4},
    {param($c) $c.AfterOwned[2].Thread=999},
    {param($c) $c.AfterOwned[0].Epoch=99},
    {param($c) $c.BeforeOwned=@($c.BeforeOwned[0],$c.BeforeOwned[2]);$c.AfterOwned=@($c.AfterOwned[0],$c.AfterOwned[2])},
    {param($c) $c.AfterOwned[2].Visible=$true}
)
foreach($change in $imeNegative){$c=New-ImeCase;& $change $c;$result=Test-SnipPostClickBaseline @c;if($result.Accepted){throw 'unsafe-IME-helper-exception-accepted'}}
$mainTry=@($ast.EndBlock.Statements|Where-Object {$_ -is [System.Management.Automation.Language.TryStatementAst]})[0]
$catch=$mainTry.CatchClauses[0].Extent.Text
if(!$catch.Contains("if(`$stage -eq 'post-click-foreground-baseline'){Release-InjectedLeft}") -or
   $catch.IndexOf('RESULT INCONCLUSIVE') -lt 0 -or
   $catch.IndexOf('Release-InjectedLeft') -gt $catch.IndexOf('RESULT INCONCLUSIVE')){throw 'recorded-input-must-release-before-catch-diagnostics'}
if($mainTry.Finally.Statements[0].Extent.Text -cne 'Release-InjectedLeft'){
    throw 'finally-must-retain-input-release-before-owned-cleanup'
}
"FOCUS PURE PASS acceptedCases=$accepted rejectedCases=$rejected imeAcceptedCases=1 imeRejectedCases=$($imeNegative.Count) diagnosticCases=2 parserErrors=0 immediateUPOrdering=true hiddenUnboundInventory=true releaseBeforeDiagnostics=true temporaryProfilerAbsent=true previewFixtureBOracleRetained=true nativeMethodsExecuted=0"
