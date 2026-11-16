# Pure single-display acceptance math + parser/C# compilation. Never launches app,
# enumerates displays, injects input, captures pixels or accesses clipboard.
$ErrorActionPreference='Stop'
$path=Join-Path $PSScriptRoot 'snipping-release-native-e2e.ps1'
$tokens=$null;$errors=$null
$ast=[System.Management.Automation.Language.Parser]::ParseFile($path,[ref]$tokens,[ref]$errors)
if($errors.Count){throw 'release-probe-parser-errors'}
$function=@($ast.FindAll({param($n) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq 'New-SnipSingleProfile'},$true))
if($function.Count -ne 1){throw 'pure-profile-constructor-missing-or-ambiguous'}
. ([scriptblock]::Create($function[0].Extent.Text))
$blurFunction=@($ast.FindAll({param($n) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq 'New-SnipStackBlurTarget'},$true))
if($blurFunction.Count -ne 1){throw 'pure-stack-blur-target-missing-or-ambiguous'}
. ([scriptblock]::Create($blurFunction[0].Extent.Text))
$cs=@($ast.FindAll({param($n) $n -is [System.Management.Automation.Language.StringConstantExpressionAst] -and $n.Value.Contains('public static class SnipReleaseProbe')},$true))
if($cs.Count -ne 1){throw 'native-helper-source-missing-or-ambiguous'}
Add-Type -TypeDefinition $cs[0].Value
# Pure fixed-enum classifier only: no GetClassNameW/native methods are invoked.
foreach($entry in @(@('IME',1),@('MSCTFIME UI',2),@('CicMarshalWndClass',3),@('Window Class',4),@('tooltips_class32',5),@('SysShadow',6),@('#32768',7),@('synthetic-unlisted-class',0))){
    if([SnipReleaseProbe]::ClassCode($entry[0]) -ne $entry[1]){throw 'fixed-window-class-enum-oracle-failed'}
}
function Equal($actual,$expected,[string]$name){if(($actual -join ',') -cne ($expected -join ',')){throw "pure-profile-oracle:$name"}}
$cases=@(
    @{Row=@(0,0,2560,1440,0,23,2560,1370,120,120);Start=@(120,120);End=@(240,240);Size=120;Preview=@(2160,23,400,330)},
    @{Row=@(2560,0,3840,2160,2560,0,3840,2160,288,288);Start=@(2848,288);End=@(3136,576);Size=288;Preview=@(5440,0,960,792)},
    @{Row=@(-1920,-1080,1920,1080,-1920,-1040,1920,1040,96,96);Start=@(-1824,-984);End=@(-1728,-888);Size=96;Preview=@(-320,-1040,320,264)},
    # Midpoint must match positive Rust f64::round, not .NET banker's rounding:
    # 264*102/96=280.5 ->281.
    @{Row=@(0,0,1920,1080,0,0,1920,1040,102,102);Start=@(102,102);End=@(204,204);Size=102;Preview=@(1580,0,340,281)}
)
foreach($case in $cases){
    $p=New-SnipSingleProfile $case.Row
    Equal $p.Bounds $case.Row[0..3] 'exact-monitor';Equal $p.WorkArea $case.Row[4..7] 'exact-work'
    Equal $p.SelectionStart $case.Start 'selection-start';Equal $p.SelectionEnd $case.End 'selection-end'
    Equal $p.SelectionSize $case.Size 'selection-size';Equal $p.Preview $case.Preview 'preview'
    Equal (($p.SelectionEnd[0]-$p.SelectionStart[0])/$p.Scale) 96 'css-width'
    Equal (($p.SelectionEnd[1]-$p.SelectionStart[1])/$p.Scale) 96 'css-height'
}
$invalid=@(
    @(),@(0,0,1),@(0,0,0,1080,0,0,1920,1080,96,96),
    @(0,0,1920,-1,0,0,1920,1080,96,96),@(0,0,1920,1080,0,0,1920,1080,0,0),
    @(0,0,1920,1080,0,0,1920,1080,120,144),@(0,0,1920,1080,-1,0,1920,1080,96,96),
    @(0,0,1920,1080,0,-1,1920,1080,96,96),@(0,0,1920,1080,0,0,1921,1080,96,96),
    @(0,0,1920,1080,0,0,1920,1081,96,96),@(0,0,1920,1080,0,0,0,1080,96,96),
    @(0,0,1920,1080,0,0,1920,0,96,96),@(0,0,16385,1080,0,0,16385,1080,96,96),
    @(0,0,16384,16384,0,0,16384,16384,96,96),@(0,0,1920,1080,0,0,400,240,96,96),
    @(0,0,1920,1080,0,0,1920,1080,960,960),
    @(2147483600,0,1920,1080,2147483600,0,1920,1080,96,96)
)
$rejected=0
foreach($row in $invalid){$failed=$false;try{$null=New-SnipSingleProfile $row}catch{$failed=$true};if(!$failed){throw 'invalid-profile-accepted'};$rejected++}
# Retain legacy exact acceptance literals; these are not single-display oracles.
$source=$ast.Extent.Text
foreach($contract in @(
    "-eq '0,0,2560,1440'", "-eq '2560,0,3840,2160'", "-eq '5440,0,960,792'",
    '$header[1] -eq 240 -and $header[2] -eq 180',
    'Start-InjectedLeft $secondary[0] $shellPid 3001 402 $secondaryEpoch',
    '(3001+20*$step) (402+15*$step)',
    'GetClipboardSequenceNumber() -ne $sequenceBefore'
)){if(!$source.Contains($contract)){throw 'legacy-exact-oracle-removed'}}
"PROFILE PURE PASS validCases=$($cases.Count) invalidCases=$rejected parserErrors=0 CSharpCompile=true nativeMethodsExecuted=0 legacyExactOraclesPresent=true"
$blurCases=@(
    # Old SingleDisplay fixture B click (240,135) lies underneath this Stack.
    @{Work=@(0,23,2560,1370);Stack=@(0,23,2000,500);OldClick=@(240,135)},
    # Stack covers both top corners; deterministic choice must move below it.
    @{Work=@(0,23,2560,1370);Stack=@(0,23,2560,600);OldClick=@(240,135)},
    @{Work=@(-1920,-1040,1920,1040);Stack=@(-1920,-1040,1920,600);OldClick=@(-1680,-928)},
    @{Work=@(0,0,800,600);Stack=@(500,300,300,300);OldClick=@(600,400)}
)
foreach($case in $blurCases){
    $target=New-SnipStackBlurTarget $case.Work $case.Stack
    $r=$target.Bounds;$s=$case.Stack;$w=$case.Work;$p=$target.Click
    if($case.OldClick[0] -lt $s[0] -or $case.OldClick[0] -ge $s[0]+$s[2] -or
       $case.OldClick[1] -lt $s[1] -or $case.OldClick[1] -ge $s[1]+$s[3]){throw 'overlap-regression-fixture-does-not-cover-old-click'}
    if($r[0] -lt $w[0] -or $r[1] -lt $w[1] -or $r[0]+$r[2] -gt $w[0]+$w[2] -or $r[1]+$r[3] -gt $w[1]+$w[3]){throw 'blur-target-outside-work-area'}
    if(!($r[0]+$r[2]+8 -le $s[0] -or $r[0] -ge $s[0]+$s[2]+8 -or
         $r[1]+$r[3]+8 -le $s[1] -or $r[1] -ge $s[1]+$s[3]+8)){throw 'blur-target-overlaps-stack-or-clearance'}
    if($p[0] -ne $r[0]+32 -or $p[1] -ne $r[1]+32){throw 'blur-click-not-fixture-center'}
    Equal (New-SnipStackBlurTarget $case.Work $case.Stack).Bounds $r 'deterministic-blur-target'
}
$blurRejected=0
foreach($case in @(
    @{Work=@(0,0,800,600);Stack=@(0,0,800,600)},
    @{Work=@(0,0,79,600);Stack=@(0,0,10,10)},
    @{Work=@(0,0,800,600);Stack=@(0,0,0,10)},
    @{Work=@(0,0,800);Stack=@(0,0,10,10)},
    @{Work=@(2147483640,0,800,600);Stack=@(0,0,10,10)}
)){
    $failed=$false;try{$null=New-SnipStackBlurTarget $case.Work $case.Stack}catch{$failed=$true}
    if(!$failed){throw 'unsafe-blur-target-accepted'};$blurRejected++
}
# Placement and hit ownership must precede injected input and confirmed focus.
if($source -notmatch 'New-SnipStackBlurTarget \$blurWorkArea \$stackBounds[\s\S]*SetWindowPos\(\$fixture[\s\S]*Assert-InputTarget \$fixture \$PID \$bx \$by[\s\S]*Start-InjectedLeft \$fixture \$PID \$bx \$by;Release-InjectedLeft[\s\S]*stack-blur-fixture-os-focus-transfer'){
    throw 'blur-target-must-be-placed-hit-verified-and-observed-focused'
}
"STACK BLUR PURE PASS overlapCases=$($blurCases.Count) rejectedCases=$blurRejected nativeMethodsExecuted=0"
