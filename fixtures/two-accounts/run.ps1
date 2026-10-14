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
