# "Nothing forgotten" on a real machine (Engineering Plan 5.1 and 9.16), Windows.
#
# Makes four real local accounts (ada the mover, bea another person, cal, dee removed but leaving
# her profile folder behind), a second real volume (a virtual disk with a drive letter), and plants
# the answer-key things as each account so the system records the owners. Then ada runs the real
# drive-wide sweep, and the answer key is checked: nothing of ada's or the shared things forgotten,
# nothing of the others' named, nothing invented, the old laptop untouched.
#
# Needs an administrator (as on CI runners). Usage: run.ps1 <folder with pctwin-lab.exe and
# pctwin-sweep.exe> [work folder] [seed]. Evidence is left in <work>\report.
param(
    [Parameter(Mandatory = $true)][string]$Bin,
    [string]$Work = 'C:\pctwin-nf',
    [string]$Seed = '1'
)
$ErrorActionPreference = 'Stop'
$script:fail = $false
$utf8 = New-Object System.Text.UTF8Encoding $false

function Note([string]$m) { Write-Host "== $m" }
function Bad([string]$m) { Write-Host "::error::$m"; $script:fail = $true }

if (Test-Path $Work) { Remove-Item -Recurse -Force $Work -ErrorAction SilentlyContinue }
$tools = Join-Path $Work 'tools'
$report = Join-Path $Work 'report'
$out = Join-Path $Work 'out'
New-Item -ItemType Directory -Force $tools, $report, $out | Out-Null
icacls $Work /grant '*S-1-1-0:(OI)(CI)RX' | Out-Null
foreach ($d in @($report, $out)) { icacls $d /grant '*S-1-1-0:(OI)(CI)M' | Out-Null }
Copy-Item (Join-Path $Bin 'pctwin-lab.exe'), (Join-Path $Bin 'pctwin-sweep.exe') $tools
$lab = Join-Path $tools 'pctwin-lab.exe'
$sweepExe = Join-Path $tools 'pctwin-sweep.exe'

# Facts the summary step reads.
$facts = [ordered]@{ os = 'Windows'; seed = [int64]$Seed; labels = @() }
function Save-Facts { [IO.File]::WriteAllText((Join-Path $report 'facts.json'), ($facts | ConvertTo-Json -Depth 6), $utf8) }
function Label([string]$element, [string]$status, [string]$noteText) {
    $script:facts.labels += [ordered]@{ element = $element; status = $status; note = $noteText }
    Save-Facts
}
Save-Facts

# 2. The plan.
Note "plan (seed $Seed)"
& $lab plan $Seed 4 (Join-Path $report 'spec.json')
if ($LASTEXITCODE -ne 0) { throw 'plan failed' }
$specPath = Join-Path $Work 'spec.json'
Copy-Item (Join-Path $report 'spec.json') $specPath
$spec = Get-Content $specPath -Raw | ConvertFrom-Json
Write-Host ("people: " + (($spec.people | ForEach-Object { "$($_.label)=$($_.role)" }) -join ', '))
Write-Host "plants: $($spec.plants.Count)"

# 3. Real accounts, with real profile folders made by the system.
function New-Password {
    $bytes = New-Object byte[] 18
    [System.Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($bytes)
    return [Convert]::ToBase64String($bytes) + 'a1!'
}
function Acct([string]$label) { return "pctwin-nf-$label" }
$labels = @('ada', 'bea', 'cal', 'dee')
$creds = @{}
foreach ($l in $labels) {
    $u = Acct $l
    $secure = ConvertTo-SecureString (New-Password) -AsPlainText -Force
    if (Get-LocalUser -Name $u -ErrorAction SilentlyContinue) { Remove-LocalUser -Name $u }
    New-LocalUser -Name $u -Password $secure -PasswordNeverExpires -AccountNeverExpires | Out-Null
    Add-LocalGroupMember -Group 'Users' -Member $u -ErrorAction SilentlyContinue
    $creds[$l] = New-Object System.Management.Automation.PSCredential("$env:COMPUTERNAME\$u", $secure)
}

function Start-As([string]$label, [string]$exe, [string[]]$arguments, [string]$log, [switch]$NewEnvironment) {
    $quoted = ($arguments | ForEach-Object { '"' + $_ + '"' }) -join ' '
    $args2 = @{
        FilePath = $exe; Credential = $creds[$label]; LoadUserProfile = $true
        WorkingDirectory = $tools; PassThru = $true; WindowStyle = 'Hidden'
        RedirectStandardOutput = (Join-Path $report "$log.out.txt")
        RedirectStandardError = (Join-Path $report "$log.err.txt")
    }
    if ($quoted) { $args2.ArgumentList = $quoted }
    if ($NewEnvironment) { $args2.UseNewEnvironment = $true }
    return Start-Process @args2
}
function Wait-Ok($process, [string]$what, [string]$log, [int]$minutes = 10) {
    if (-not $process.WaitForExit($minutes * 60000)) { throw "$what did not finish in $minutes minutes" }
    if ($process.ExitCode -ne 0) {
        Get-Content (Join-Path $report "$log.err.txt") -ErrorAction SilentlyContinue | Write-Host
        throw "$what failed with exit code $($process.ExitCode)"
    }
}
function Invoke-As([string]$label, [string]$exe, [string[]]$arguments, [string]$log) {
    $p = Start-As $label $exe $arguments $log
    Wait-Ok $p "$log ($label)" $log
}

# One run as each account makes its profile folder, the way a first sign-in would.
$whoami = Join-Path $env:SystemRoot 'System32\whoami.exe'
foreach ($l in $labels) { Invoke-As $l $whoami @() "profile-$l" }
# Whether an account can list a folder: the lab's own snapshot does the listing.
function Can-List([string]$label, [string]$dir) {
    $q = Start-As $label $lab @('snapshot', $dir, (Join-Path $report 'peek.json')) 'peek'
    $null = $q.WaitForExit(120000)
    return ($q.ExitCode -eq 0)
}
function Get-ProfileOf([string]$label) {
    $sid = (Get-LocalUser -Name (Acct $label)).SID.Value
    return (Get-CimInstance Win32_UserProfile | Where-Object { $_.SID -eq $sid }).LocalPath
}
$homes = [ordered]@{}
foreach ($l in $labels) {
    $h = Get-ProfileOf $l
    if (-not $h -or -not (Test-Path $h)) { throw "no profile folder for $l" }
    $homes[$l] = $h
    Write-Host "home $l = $h"
}
# Profile folders already refuse other standard users: verify, never assume.
if (Can-List 'ada' $homes['cal']) { Bad "ada can list cal's profile folder; the test would prove nothing" }
if (Can-List 'ada' $homes['bea']) {
    Label "bea's home listable by others" 'real' 'the system let ada list bea profile folder'
} else {
    Label "bea's home listable by others" 'not reachable' 'Windows profile folders refuse other standard users; bea plants inside are refused to ada by the system default'
}

# 4. A second real volume: a virtual disk, NTFS, with a drive letter.
Note 'second volume'
$letter = 'R'
if (Test-Path "${letter}:\") { throw "drive letter $letter is already in use" }
$vhd = Join-Path $env:RUNNER_TEMP 'pctwin-nf.vhd'
if (-not $env:RUNNER_TEMP) { $vhd = Join-Path $Work 'pctwin-nf.vhd' }
$dp = Join-Path $Work 'diskpart.txt'
[IO.File]::WriteAllLines($dp, @(
    "create vdisk file=`"$vhd`" maximum=512 type=expandable",
    "select vdisk file=`"$vhd`"",
    'attach vdisk',
    'create partition primary',
    'format fs=ntfs quick label=PCTWINNF',
    "assign letter=$letter",
    'exit'), (New-Object System.Text.ASCIIEncoding))
$o = & diskpart /s $dp | Out-String
Write-Host $o
if ($LASTEXITCODE -ne 0 -or $o -match 'DiskPart has encountered an error|Virtual Disk Service error') { throw 'diskpart failed' }
$other = "${letter}:\"
Start-Sleep -Seconds 2
$acl = icacls $other | Out-String
Write-Host $acl
if ($acl -notmatch 'Authenticated Users:.*\(AD\)|Authenticated Users:\(CI\)\(AD\)|Everyone:.*\(AD\)') {
    Write-Host 'the drive root does not let accounts create folders by default: granting it'
    icacls $other /grant '*S-1-5-11:(AD)' | Out-Null
    icacls $other | Write-Host
}

$roots = [ordered]@{ system_root = 'C:\'; other_drive = $other; homes = $homes }
$rootsPath = Join-Path $Work 'roots.json'
[IO.File]::WriteAllText($rootsPath, ($roots | ConvertTo-Json -Depth 4), $utf8)
Copy-Item $rootsPath (Join-Path $report 'roots.json')

function Top-Dir($plant) {
    $top = $plant.folder.Split('/')[0]
    switch ($plant.spot) {
        'system-root' { return Join-Path 'C:\' $top }
        'other-drive' { return Join-Path $other $top }
        'home' { return Join-Path $homes[$plant.person] $top }
    }
}

# 5. Each account plants its own things. Dee's account is removed after, keeping her profile
# folder and its profile-list entry (as deleting the account from Settings or `net user /delete`
# does): she is a leftover.
foreach ($l in $labels) {
    Note "plant as $l"
    try { Invoke-As $l $lab @('plant', $specPath, $rootsPath, $l) "plant-$l" }
    catch { Bad "planting as $l failed: $_" }
}
$deeSid = (Get-LocalUser -Name (Acct 'dee')).SID.Value
Remove-LocalUser -Name (Acct 'dee')
if (Get-LocalUser -Name (Acct 'dee') -ErrorAction SilentlyContinue) { Bad "dee's account still exists" }
if (-not (Test-Path $homes['dee'])) { Bad "dee's profile folder is gone" }
$stillListed = Get-CimInstance Win32_UserProfile | Where-Object { $_.SID -eq $deeSid }
Write-Host "dee's profile-list entry kept: $([bool]$stillListed)"

# The folders really are as refused / open as the plan says, for the mover.
Note "the mover's real view"
$observed = [ordered]@{ not_reachable = @() }
foreach ($p in $spec.plants) {
    $dir = Top-Dir $p
    $listed = Can-List 'ada' $dir
    Write-Host "ada lists $dir ($($p.by), $($p.opens)): $listed"
    if ($p.by -eq 'ada' -and $p.opens -eq 'nobody' -and $listed) {
        $observed.not_reachable += $p.token
        Label "refused folder $($p.token)" 'not reachable' 'the mover could list a folder set to refuse everyone'
    }
}
$observedPath = Join-Path $report 'observed.json'
[IO.File]::WriteAllText($observedPath, ($observed | ConvertTo-Json -Depth 3), $utf8)
$facts.macos_privacy = 'not-applicable'
Label 'macOS privacy refusal' 'not reachable' 'not a Mac'

# 6. Before: the planted folders as an administrator sees them. The refused plant denies only its
# owner, so an administrator can still read it.
function Snap-All([string]$dir) {
    New-Item -ItemType Directory -Force $dir | Out-Null
    foreach ($p in $spec.plants) {
        & $lab snapshot (Top-Dir $p) (Join-Path $dir "$($p.token).json")
        if ($LASTEXITCODE -ne 0) { Bad "snapshot of $($p.token) failed" }
    }
}
Note 'snapshot before'
Snap-All (Join-Path $report 'before')

# 8. The mover sweeps the whole laptop, in a fresh process with her own environment.
Note 'sweep as ada'
$sweepOut = Join-Path $out 'sweep.json'
$clock = [Diagnostics.Stopwatch]::StartNew()
$sp = Start-As 'ada' $sweepExe @($sweepOut) 'sweep' -NewEnvironment
$finished = $sp.WaitForExit(85 * 60000)
$clock.Stop()
$facts.wall_seconds = [int]$clock.Elapsed.TotalSeconds
if (-not $finished) {
    Stop-Process -Id $sp.Id -Force
    Save-Facts
    throw 'the sweep did not finish in 85 minutes'
}
$facts.sweep_exit = $sp.ExitCode
Save-Facts
Get-Content (Join-Path $report 'sweep.out.txt') | Write-Host
Get-Content (Join-Path $report 'sweep.err.txt') -TotalCount 50 | Write-Host
if ($sp.ExitCode -ne 0) { throw "the sweep failed with exit $($sp.ExitCode)" }
Copy-Item $sweepOut (Join-Path $report 'sweep.json')

# 9. After: the old laptop is unchanged.
Note 'snapshot after and compare'
Snap-All (Join-Path $report 'after')
$cmpBad = $false
$cmpFile = Join-Path $report 'compare.txt'
Set-Content -Path $cmpFile -Value '' -Encoding ascii
foreach ($f in Get-ChildItem (Join-Path $report 'before') -Filter *.json) {
    $lines = & $lab compare $f.FullName (Join-Path $report "after\$($f.Name)")
    if ($LASTEXITCODE -ne 0) { $cmpBad = $true; Add-Content $cmpFile $lines }
}
if ($cmpBad) { Bad 'the sweep changed the old laptop'; Get-Content $cmpFile | Write-Host }
$facts.compare_clean = (-not $cmpBad)
Save-Facts

# 10. The answer key, as an administrator who may look everywhere.
Note 'check-sweep'
$checkFile = Join-Path $report 'check.txt'
& $lab check-sweep $specPath $rootsPath (Join-Path $report 'sweep.json') $observedPath *>&1 | Tee-Object -FilePath $checkFile
$checkRc = $LASTEXITCODE
$facts.check_exit = $checkRc
if ($checkRc -ne 0) { Bad 'check-sweep found problems' }

# None of the other people's tokens may appear in what the sweep printed, or in its result.
$moverLabel = $spec.people[0].label
$texts = @{}
foreach ($f in @('sweep.out.txt', 'sweep.err.txt', 'sweep.json')) { $texts[$f] = [IO.File]::ReadAllText((Join-Path $report $f)) }
$leaks = 0
foreach ($p in @($spec.plants | Where-Object { $_.by -ne $moverLabel })) {
    foreach ($f in $texts.Keys) {
        if ($texts[$f].Contains($p.token)) { Write-Host "LEAK: $($p.token) named in $f"; $leaks++ }
    }
}
$facts.leaks = $leaks
if ($leaks -gt 0) { Bad "$leaks other-people tokens leaked" }

# 11. Evidence.
$sweepObj = $texts['sweep.json'] | ConvertFrom-Json -Depth 100
[IO.File]::WriteAllText((Join-Path $report 'census.json'), ($sweepObj.census | ConvertTo-Json -Depth 20), $utf8)
[IO.File]::WriteAllText((Join-Path $report 'top_line.json'), ($sweepObj.top_line | ConvertTo-Json -Depth 5), $utf8)
$texts = $null; $sweepObj = $null
$src = Join-Path $report 'sweep.json'
$gz = New-Object IO.Compression.GZipStream([IO.File]::Create("$src.gz"), [IO.Compression.CompressionMode]::Compress)
$in = [IO.File]::OpenRead($src); $in.CopyTo($gz); $in.Dispose(); $gz.Dispose()
Remove-Item $src
Label 'accounts and home folders' 'real' 'four local accounts made by the system; dee removed, her profile folder and list entry kept'
Label 'second volume' 'real' 'NTFS virtual disk with drive letter R'
Label 'planted things' 'real' 'planted by each account, the system recorded the owners'
Label 'snapshots before and after' 'real' 'the planted folders only (not the whole machine), taken as administrator; the refused folder denies only its owner, so the administrator reads it too'
Save-Facts
if ($script:fail) { exit 1 }
