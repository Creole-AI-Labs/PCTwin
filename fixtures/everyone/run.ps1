# Everyone in one move (Engineering Plan 5.1: "Several people in one move" and "Nothing
# forgotten"; Task List, moving everyone, step 8), Windows. The same as run.sh: four people (the
# mover a fresh standard account; Bea and Carl approved; Dan not), about forty planted things,
# the answer key the administrator's. This script is the administrator and starts the helper the
# way the system's prompt would, from outside the product.
#
# 1. A second helper while one runs is refused; another account cannot call the helper.
# 2. The move, as the mover, with a junction planted in a holding folder; checked against the key.
# 3. The mover cannot look into the helper's process.
# 5. No administrator anywhere: the mover's things move; the others are listed, never moved.
#
# Needs an administrator (as on CI runners). Usage: run.ps1 -Bin <folder with pctwin-labmove.exe
# and pctwin-helper.exe> [-Work <folder>]. Leaves its evidence in <work>\admin.
param(
    [Parameter(Mandatory = $true)][string]$Bin,
    [string]$Work = 'C:\pctwin-everyone'
)
$ErrorActionPreference = 'Stop'
$here = $PSScriptRoot
$mover = 'pctwinmover'
$people = @('pctwinbea', 'pctwincarl', 'pctwindan')

# Throwaway passwords for throwaway accounts on a throwaway machine; never printed.
function New-Password {
    $bytes = New-Object byte[] 18
    [System.Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($bytes)
    return [Convert]::ToBase64String($bytes) + 'a1!'
}

function Remove-Account([string]$u) {
    $user = Get-LocalUser -Name $u -ErrorAction SilentlyContinue
    if ($user) {
        Get-CimInstance Win32_UserProfile | Where-Object { $_.SID -eq $user.SID.Value } | Remove-CimInstance
        Remove-LocalUser -Name $u
    }
}

$accounts = @{}
foreach ($u in @($mover) + $people) {
    Remove-Account $u
    $secure = ConvertTo-SecureString (New-Password) -AsPlainText -Force
    New-LocalUser -Name $u -Password $secure -PasswordNeverExpires -AccountNeverExpires | Out-Null
    Add-LocalGroupMember -Group 'Users' -Member $u -ErrorAction SilentlyContinue
    $accounts[$u] = New-Object System.Management.Automation.PSCredential("$env:COMPUTERNAME\$u", $secure)
}
foreach ($n in 1..3) { Remove-Account "pctwinnew$n" }

if (Test-Path $Work) { Remove-Item -Recurse -Force $Work }
foreach ($old in @('C:\pctwin-lab', 'C:\Users\Public\Documents\pctwin-shared')) {
    if (Test-Path $old) { Remove-Item -Recurse -Force $old }
}
# The lab's own folder outside any home, open like the drive's top; each person's folder in it is
# theirs alone.
New-Item -ItemType Directory -Force 'C:\pctwin-lab' | Out-Null
$admin = Join-Path $Work 'admin'
$moverDir = Join-Path $Work 'mover'
$tools = Join-Path $Work 'tools'
New-Item -ItemType Directory -Force $admin, $moverDir, $tools | Out-Null
# The administrator's folder holds the key and the evidence; the mover's what the mover can read.
icacls $admin /inheritance:r /grant:r '*S-1-5-32-544:(OI)(CI)F' '*S-1-5-18:(OI)(CI)F' | Out-Null
icacls $moverDir /inheritance:r /grant:r "${mover}:(OI)(CI)F" '*S-1-5-32-544:(OI)(CI)F' '*S-1-5-18:(OI)(CI)F' | Out-Null
# The helper runs the hand-over program from its own folder, as in the app.
Copy-Item (Join-Path $Bin 'pctwin-labmove.exe'), (Join-Path $Bin 'pctwin-helper.exe'), (Join-Path $Bin 'pctwin-handover.exe') $tools
icacls $tools /grant '*S-1-1-0:(OI)(CI)RX' | Out-Null
$labmove = Join-Path $tools 'pctwin-labmove.exe'
$helperExe = Join-Path $tools 'pctwin-helper.exe'

# Starts a program as an account with its profile (made the first time); its output goes to
# `out` and `err`, files this script makes, so where they are decides who can read them.
function Start-As([string]$user, [string]$exe, [string[]]$arguments, [string]$out, [string]$err) {
    $quoted = ($arguments | ForEach-Object { '"' + $_ + '"' }) -join ' '
    return Start-Process -FilePath $exe -ArgumentList $quoted -Credential $accounts[$user] `
        -LoadUserProfile -WorkingDirectory $tools -PassThru -WindowStyle Hidden `
        -RedirectStandardOutput $out -RedirectStandardError $err
}

function Wait-Exit($process, [string]$what) {
    if (-not $process.WaitForExit(1800000)) { throw "$what did not finish in thirty minutes" }
    return $process.ExitCode
}

# Each person signs in once, so their home exists.
foreach ($u in @($mover) + $people) {
    # A real program, not a cmd built-in: Start-As quotes every argument, and cmd cannot run a
    # quoted built-in ("exit").
    $whoami = Join-Path ([Environment]::GetFolderPath('System')) 'whoami.exe'
    $p = Start-As $u $whoami @('/user') (Join-Path $admin "first-$u.out.txt") (Join-Path $admin "first-$u.err.txt")
    if ((Wait-Exit $p "first sign-in of $u") -ne 0) { throw "$u could not sign in" }
}

python (Join-Path $here 'plant.py') plant (Join-Path $here 'spec-windows.tsv') (Join-Path $admin 'planted.json')
if ($LASTEXITCODE -ne 0) { throw 'planting failed' }

function Hex([int]$n) {
    $bytes = New-Object byte[] $n
    [System.Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($bytes)
    return -join ($bytes | ForEach-Object { $_.ToString('x2') })
}
$laptop = Hex 16
$token = Hex 16
function Sid([string]$u) { return (Get-LocalUser -Name $u).SID.Value }
$approve = "$(Sid 'pctwinbea'),$(Sid 'pctwincarl'),pctwinnew1,pctwinnew2"

# The system's prompt, said yes to: the helper started as an administrator for this move.
function Start-Helper([string]$pipe, [string]$log) {
    return Start-Process -FilePath $helperExe -PassThru -WindowStyle Hidden `
        -ArgumentList "--session $token --mover $(Sid $mover) --approve $approve --channel $pipe --laptop $laptop" `
        -RedirectStandardOutput "$log.out.txt" -RedirectStandardError "$log.err.txt"
}

Write-Host '== 1. one helper at a time; only the mover''s account may call it =='
# Stops a program and everything it started (the helper on Windows runs as a clean copy
# of itself; stopping only the first would leave the copy behind).
function Stop-Tree($proc) {
    if ($proc) { & taskkill.exe /T /F /PID $proc.Id 2>&1 | Out-Null }
}

$pipe1 = "pctwin-helper-$(Hex 16)"
$first = Start-Helper $pipe1 (Join-Path $admin 'helper-1')
Start-Sleep -Seconds 2
$second = Start-Helper "pctwin-helper-$(Hex 16)" (Join-Path $admin 'helper-second')
if ((Wait-Exit $second 'the second helper') -eq 0) { throw 'a second helper ran while one was running' }
$carl = Start-As 'pctwincarl' $labmove @('hello', $token, $pipe1) (Join-Path $admin 'carl.out.txt') (Join-Path $admin 'carl.err.txt')
if ((Wait-Exit $carl 'Carl calling the helper') -eq 0) { throw "another account was served by the mover's helper" }
Stop-Tree $first
foreach ($n in 1..2) { if (Get-LocalUser -Name "pctwinnew$n" -ErrorAction SilentlyContinue) { throw "Carl's call made an account" } }

Write-Host '== 2. the move of everyone =='
$etc = Join-Path $env:SystemRoot 'System32\drivers\etc'
$etcOwner = (Get-Acl $etc).Owner
$pipe3 = "pctwin-helper-$(Hex 16)"
$helper = Start-Helper $pipe3 (Join-Path $admin 'helper-3')
Start-Sleep -Seconds 1

Write-Host '== 3. the mover cannot look into the helper =='
$probe = 'try { if ((Get-Process -Id ' + $helper.Id + ').Modules.Count -gt 0) { exit 0 } else { exit 1 } } catch { exit 1 }'
$look = Start-As $mover 'powershell.exe' @('-NoProfile', '-NonInteractive', '-Command', $probe) (Join-Path $admin 'debugger.out.txt') (Join-Path $admin 'debugger.err.txt')
if ((Wait-Exit $look 'the mover looking into the helper') -eq 0) { throw "the mover opened the helper's process" }

$newHome = Join-Path $moverDir 'new-home'
# Lab only: the runner's work drive (D: on hosted Windows runners) holds the runner's own build
# and nothing planted (every planted file is on C:), so the mover's sweep leaves it out.
# Written without a closing backslash: Start-As quotes each argument, and a backslash before the
# closing quote would swallow it ("D:\" became D:" and matched no drive, so D: was swept).
# The runner's own folders that no rules row names (nothing is planted in them) are left out too.
$leaveOut = @('--leave-out-folder', 'C:\actionarchivecache,C:\agent,C:\cobertura-2.1.1,C:\Julia')
if (Test-Path 'D:\') { $leaveOut += @('--leave-out-volume', 'D:') }
$run = Start-As $mover $labmove (@('everyone', (Join-Path $moverDir 'report.json'), '--laptop', $laptop,
    '--new-home', $newHome, '--journal', (Join-Path $moverDir 'journal.redb'), '--new-names', 'pctwinnew',
    '--hostile', 'yes', '--session', $token, '--channel', $pipe3) + $leaveOut) (Join-Path $admin 'evidence.json') (Join-Path $moverDir 'log.txt')
# The mover's own engine, while it carries other people's things, is closed to the mover too:
# reading it lists no modules.
Start-Sleep -Seconds 5
$engineProbe = 'try { if ((Get-Process -Id ' + $run.Id + ').Modules.Count -gt 0) { exit 0 } else { exit 1 } } catch { exit 1 }'
$lookEngine = Start-As $mover 'powershell.exe' @('-NoProfile', '-NonInteractive', '-Command', $engineProbe) (Join-Path $admin 'engine-look.out.txt') (Join-Path $admin 'engine-look.err.txt')
if ((Wait-Exit $lookEngine 'the mover looking into their engine') -eq 0) { throw 'the mover read their own engine while it carried other people''s things' }
# The mover's log (where the move got to, stage by stage) is kept with the evidence whatever
# happens, a move that never ends included: stopped, then saved, then the reason.
try {
    if ((Wait-Exit $run 'the move') -ne 0) { throw 'the move failed' }
    $null = Wait-Exit $helper 'the helper'
} catch {
    Stop-Tree $run
    Stop-Tree $helper
    Start-Sleep -Seconds 2
    throw
} finally {
    Copy-Item (Join-Path $moverDir 'log.txt') (Join-Path $admin 'mover-log.txt') -ErrorAction SilentlyContinue
    Get-Content (Join-Path $admin 'mover-log.txt') -ErrorAction SilentlyContinue | Write-Host
    Get-Content (Join-Path $admin 'helper-3.err.txt') -ErrorAction SilentlyContinue | Write-Host
}

python (Join-Path $here 'plant.py') key (Join-Path $admin 'planted.json') $mover $newHome (Join-Path $admin 'evidence.json') (Join-Path $admin 'key.json')
if ($LASTEXITCODE -ne 0) { throw 'the key could not be written' }
& $labmove check-everyone (Join-Path $admin 'key.json') (Join-Path $admin 'evidence.json') (Join-Path $moverDir 'report.json') (Join-Path $moverDir 'log.txt')
if ($LASTEXITCODE -ne 0) { throw 'the move of everyone found problems' }
if ((Get-Acl $etc).Owner -ne $etcOwner) { throw 'the planted junction changed who owns the system''s folder' }

Write-Host '== 5. no administrator anywhere =='
$none = Join-Path $moverDir 'none'
New-Item -ItemType Directory -Force $none | Out-Null
$run = Start-As $mover $labmove (@('everyone', (Join-Path $none 'report.json'), '--laptop', $laptop,
    '--new-home', (Join-Path $none 'new-home'), '--journal', (Join-Path $none 'journal.redb'),
    '--new-names', 'pctwinnone') + $leaveOut) (Join-Path $admin 'evidence-none.json') (Join-Path $none 'log.txt')
if ((Wait-Exit $run 'the move with no administrator') -ne 0) {
    Get-Content (Join-Path $none 'log.txt') -ErrorAction SilentlyContinue | Write-Host
    throw 'the move with no administrator failed'
}
$planted = Get-Content (Join-Path $admin 'planted.json') -Raw | ConvertFrom-Json
$moverSid = Sid $mover
$locked = 0
foreach ($p in $planted.planted) {
    if ($p.owner -ne $moverSid -and $p.owner -ne 'shared' -and $p.expect -eq 'moves') { $p.expect = 'stays' }
    # Windows: an ordinary account cannot read who owns a folder it may not open, so without the
    # helper an administrators-only place cannot be told from another person's and is counted,
    # never named (unlike a Mac or Linux, where the owner shows). It must stay, and be counted.
    if ($p.expect -eq 'blind') { $p.expect = 'stays'; $locked++ }
}
$planted | ConvertTo-Json -Depth 5 | Out-File -Encoding utf8 (Join-Path $admin 'planted-none.json')
python (Join-Path $here 'plant.py') key (Join-Path $admin 'planted-none.json') $mover (Join-Path $none 'new-home') (Join-Path $admin 'evidence-none.json') (Join-Path $admin 'key-none.json')
& $labmove check-everyone (Join-Path $admin 'key-none.json') (Join-Path $admin 'evidence-none.json') (Join-Path $none 'report.json') (Join-Path $none 'log.txt')
if ($LASTEXITCODE -ne 0) { throw 'the move with no administrator found problems' }
$e = Get-Content (Join-Path $admin 'evidence-none.json') -Raw | ConvertFrom-Json
$unreadable = (@($e.blind_spots) | Where-Object { $_.reason -eq 'access-not-readable' } | Measure-Object -Property count -Sum).Sum
if ($locked -gt 0 -and $unreadable -lt 1) { throw 'without an administrator, the places the mover could not read are not counted' }
if ($unreadable -gt 0 -and $e.top_line.line -eq 'everything') { throw 'the report says everything moved, though places could not be read' }
if (-not $e.others -or (@($e.declined) | Sort-Object) -join ',' -ne (@($e.others) | Sort-Object) -join ',') {
    throw 'without an administrator, the others must be listed, and none scanned'
}
Write-Host 'everyone in one move: passed'
