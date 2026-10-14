# Same-machine move between two user accounts (Task List 0.7), Windows.
#
# Makes two ordinary local accounts with different names, gives each a private folder the other
# cannot read, builds a synthetic old laptop in the first, then moves it to the second through the
# real engine: pairing code, number match, sealed link, safety gate and change journal. Passes only
# if every file is accounted for on both sides, every file that landed matches its original byte
# for byte, and the old laptop's folder is unchanged.
#
# Needs an administrator (as on CI runners). Usage: run.ps1 <folder with pctwin-lab.exe and
# pctwin-labmove.exe> [work folder]. Leaves its evidence (reports and snapshots) in <work>\report.
param(
    [Parameter(Mandatory = $true)][string]$Bin,
    [string]$Work = 'C:\pctwin-two-accounts'
)
$ErrorActionPreference = 'Stop'

$oldUser = 'pctwin-old-ada'
$newUser = 'pctwin-new-bob'

# Throwaway passwords for throwaway accounts on a throwaway machine; never printed.
function New-Password {
    $bytes = New-Object byte[] 18
    [System.Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($bytes)
    return [Convert]::ToBase64String($bytes) + 'a1!'
}

$accounts = @{}
foreach ($u in @($oldUser, $newUser)) {
    $pw = New-Password
    $secure = ConvertTo-SecureString $pw -AsPlainText -Force
    if (Get-LocalUser -Name $u -ErrorAction SilentlyContinue) { Remove-LocalUser -Name $u }
    New-LocalUser -Name $u -Password $secure -PasswordNeverExpires -AccountNeverExpires | Out-Null
    Add-LocalGroupMember -Group 'Users' -Member $u -ErrorAction SilentlyContinue
    $accounts[$u] = New-Object System.Management.Automation.PSCredential("$env:COMPUTERNAME\$u", $secure)
}

if (Test-Path $Work) { Remove-Item -Recurse -Force $Work }
$tools = Join-Path $Work 'tools'
New-Item -ItemType Directory -Force (Join-Path $Work 'old'), (Join-Path $Work 'new'), (Join-Path $Work 'handoff'), (Join-Path $Work 'report'), $tools | Out-Null
Copy-Item (Join-Path $Bin 'pctwin-lab.exe'), (Join-Path $Bin 'pctwin-labmove.exe') $tools

# Private folders: only their own account (and administrators, to clean up) may enter.
icacls (Join-Path $Work 'old') /inheritance:r /grant:r "${oldUser}:(OI)(CI)F" '*S-1-5-32-544:(OI)(CI)F' '*S-1-5-18:(OI)(CI)F' | Out-Null
icacls (Join-Path $Work 'new') /inheritance:r /grant:r "${newUser}:(OI)(CI)F" '*S-1-5-32-544:(OI)(CI)F' '*S-1-5-18:(OI)(CI)F' | Out-Null
# Shared: both may write the handoff and their reports, and run the tools.
foreach ($d in @('handoff', 'report')) {
    icacls (Join-Path $Work $d) /grant "*S-1-1-0:(OI)(CI)M" | Out-Null
}
icacls $tools /grant "*S-1-1-0:(OI)(CI)RX" | Out-Null

$lab = Join-Path $tools 'pctwin-lab.exe'
$labmove = Join-Path $tools 'pctwin-labmove.exe'
$report = Join-Path $Work 'report'

# Starts a program as an account; its output goes to a log in the report folder.
function Start-As([string]$user, [string]$exe, [string[]]$arguments, [string]$log) {
    $quoted = ($arguments | ForEach-Object { '"' + $_ + '"' }) -join ' '
    return Start-Process -FilePath $exe -ArgumentList $quoted -Credential $accounts[$user] `
        -LoadUserProfile -WorkingDirectory $tools -PassThru -WindowStyle Hidden `
        -RedirectStandardOutput (Join-Path $report "$log.out.txt") `
        -RedirectStandardError (Join-Path $report "$log.err.txt")
}

function Wait-Ok($process, [string]$what, [string]$log) {
    if (-not $process.WaitForExit(600000)) { throw "$what did not finish in ten minutes" }
    if ($process.ExitCode -ne 0) {
        Get-Content (Join-Path $report "$log.err.txt") -ErrorAction SilentlyContinue | Write-Host
        throw "$what failed with exit code $($process.ExitCode)"
    }
}

function Invoke-As([string]$user, [string]$exe, [string[]]$arguments, [string]$log) {
    $p = Start-As $user $exe $arguments $log
    Wait-Ok $p "$log ($user)" $log
}

$oldLaptop = Join-Path $Work 'old\laptop'
$newHome = Join-Path $Work 'new\home'

Invoke-As $oldUser $lab @('make', $oldLaptop, '21', '150', '4194304') 'make'
Invoke-As $oldUser $lab @('snapshot', $oldLaptop, (Join-Path $report 'before.json')) 'snapshot-before'

# The accounts really are separate: the new laptop's account cannot read the old one's files.
$peek = Start-As $newUser 'cmd.exe' @('/c', 'dir', $oldLaptop) 'peek'
$null = $peek.WaitForExit(60000)
if ($peek.ExitCode -eq 0) { throw "the two accounts can read each other's folders; the test would prove nothing" }

$receiver = Start-As $newUser $labmove @('receive', (Join-Path $Work 'handoff'), (Join-Path $report 'received.json'), (Join-Path $Work 'new\journal.redb'), '--lab', $newHome, $newUser) 'receive'
$sender = Start-As $oldUser $labmove @('send', (Join-Path $Work 'handoff'), (Join-Path $report 'sent.json'), '--lab', $oldLaptop, $oldUser) 'send'
Wait-Ok $sender 'the old laptop (send)' 'send'
Wait-Ok $receiver 'the new laptop (receive)' 'receive'

Invoke-As $oldUser $lab @('snapshot', $oldLaptop, (Join-Path $report 'after.json')) 'snapshot-after'
Invoke-As $newUser $lab @('snapshot', $newHome, (Join-Path $report 'dest.json')) 'snapshot-dest'

& $labmove check (Join-Path $report 'before.json') (Join-Path $report 'after.json') (Join-Path $report 'dest.json') (Join-Path $report 'sent.json') (Join-Path $report 'received.json')
if ($LASTEXITCODE -ne 0) { throw 'the move check found problems' }

# ---------------------------------------------------------------------------------------------
# Real-laptop fixtures: the same move between the accounts' real special folders, set up the way
# real laptops are. The old laptop keeps Documents in OneDrive. The new laptop keeps Pictures on
# another drive (D:) and Music under a German name, and is already in use: its own file sits
# where an incoming one would land and must be left exactly as it was.
Write-Host '== real special folders =='
$profileWork = Join-Path $Work 'profile'
$pHandoff = Join-Path $profileWork 'handoff'
$pReport = Join-Path $profileWork 'report'
New-Item -ItemType Directory -Force $pHandoff, $pReport | Out-Null
foreach ($d in @($pHandoff, $pReport)) { icacls $d /grant "*S-1-1-0:(OI)(CI)M" | Out-Null }
$report = $pReport

# A program started as another account inherits this one's environment, so %USERPROFILE% would
# name the administrator's folder: each account's real profile folder is looked up instead, and
# Windows' own reg.exe is named in full (Git's tools come first on the runner's PATH).
function Get-ProfileOf([string]$user) {
    $sid = (Get-LocalUser -Name $user).SID.Value
    return (Get-CimInstance Win32_UserProfile | Where-Object { $_.SID -eq $sid }).LocalPath
}
$reg = Join-Path $env:SystemRoot 'System32\reg.exe'
$oldProfile = Get-ProfileOf $oldUser
$newProfile = Get-ProfileOf $newUser
if (-not $oldProfile -or -not $newProfile) { throw 'a test account has no profile folder' }

$shell = 'HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders'
# Folders made inside a profile take that profile's permissions, so its owner can use them.
New-Item -ItemType Directory -Force (Join-Path $oldProfile 'OneDrive\Documents') | Out-Null
Invoke-As $oldUser $reg @('add', 'HKCU\Software\Microsoft\OneDrive\Accounts\Personal', '/v', 'UserFolder', '/d', (Join-Path $oldProfile 'OneDrive'), '/f') 'old-onedrive-account'
Invoke-As $oldUser $reg @('add', $shell, '/v', 'Personal', '/t', 'REG_EXPAND_SZ', '/d', (Join-Path $oldProfile 'OneDrive\Documents'), '/f') 'old-documents-in-onedrive'

$otherDrive = if (Test-Path 'D:\') { 'D:' } else { $null }
if ($otherDrive) {
    $pictures = "$otherDrive\pctwin-new-bob-pictures"
    New-Item -ItemType Directory -Force $pictures | Out-Null
    icacls $pictures /grant "${newUser}:(OI)(CI)F" | Out-Null
    Invoke-As $newUser $reg @('add', $shell, '/v', 'My Pictures', '/t', 'REG_EXPAND_SZ', '/d', $pictures, '/f') 'new-pictures-on-d'
} else {
    Write-Host 'No D: drive on this runner: Pictures stays on the system drive.'
}
New-Item -ItemType Directory -Force (Join-Path $newProfile 'Musik') | Out-Null
Invoke-As $newUser $reg @('add', $shell, '/v', 'My Music', '/t', 'REG_EXPAND_SZ', '/d', (Join-Path $newProfile 'Musik'), '/f') 'new-music-german'

# Where each account's folders really are, against the plain guess; IDs stable across scans.
Invoke-As $oldUser $labmove @('probe', (Join-Path $pReport 'probe-old.json')) 'probe-old'
Invoke-As $newUser $labmove @('probe', (Join-Path $pReport 'probe-new.json')) 'probe-new'
$probeOld = Get-Content (Join-Path $pReport 'probe-old.json') -Raw | ConvertFrom-Json
$docs = $probeOld.probe.folders | Where-Object { $_.role -eq 'documents' }
if (-not $docs.moved -or -not $docs.storage.cloud) { throw "the old laptop's Documents in OneDrive was not seen as moved into OneDrive" }

# The new laptop is already in use: its own file sits where an incoming one would land.
$mine = Join-Path $newProfile 'Documents\empty.txt'
Set-Content -Path $mine -Value 'mine, before the move' -Encoding utf8
icacls $mine /grant "${newUser}:F" | Out-Null

Invoke-As $oldUser $labmove @('fill-profile', '31', '80', '2097152') 'fill-profile'
Copy-Item (Join-Path $pReport 'fill-profile.out.txt') (Join-Path $pReport 'made.json')
Invoke-As $oldUser $labmove @('snapshot-profile', (Join-Path $pReport 'before.json')) 'snapshot-before'
$receiver = Start-As $newUser $labmove @('receive', $pHandoff, (Join-Path $pReport 'received.json'), (Join-Path $Work 'new\journal-profile.redb')) 'receive'
$sender = Start-As $oldUser $labmove @('send', $pHandoff, (Join-Path $pReport 'sent.json')) 'send'
Wait-Ok $sender 'the old laptop (send, real folders)' 'send'
Wait-Ok $receiver 'the new laptop (receive, real folders)' 'receive'
Invoke-As $oldUser $labmove @('snapshot-profile', (Join-Path $pReport 'after.json')) 'snapshot-after'
Invoke-As $newUser $labmove @('snapshot-profile', (Join-Path $pReport 'dest.json')) 'snapshot-dest'

if ((Get-Content $mine -Raw).Trim() -ne 'mine, before the move') { throw "the new laptop's own file was changed by the move" }
# A real profile also holds the system's own files (shared desktop shortcuts and the like): each
# must have an outcome on both sides, but only the lab's own files are judged.
& $labmove check (Join-Path $pReport 'before.json') (Join-Path $pReport 'after.json') (Join-Path $pReport 'dest.json') (Join-Path $pReport 'sent.json') (Join-Path $pReport 'received.json') --only (Join-Path $pReport 'made.json')
if ($LASTEXITCODE -ne 0) { throw 'the move check found problems (real folders)' }
