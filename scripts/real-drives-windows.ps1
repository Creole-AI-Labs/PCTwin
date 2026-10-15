# Real FAT-family drives for the gate's real-drive tests, on a Windows runner (an administrator).
#
# Usage: scripts/real-drives-windows.ps1 setup | remount
#   setup    makes a FAT32 and an exFAT virtual disk with diskpart, mounts each at an empty folder
#            under $RUNNER_TEMP and exports PCTWIN_FAT_DIRS (kind=path, separated by |).
#   remount  detaches every virtual disk and attaches it again at the same folder.
param([Parameter(Mandatory = $true)][ValidateSet('setup', 'remount')][string]$Action)
$ErrorActionPreference = 'Stop'

$root = $env:RUNNER_TEMP
$drives = @(
    @{ Kind = 'fat32-win'; Fs = 'fat32'; Want = 'FAT32'; Name = 'pct-fat32' },
    @{ Kind = 'exfat-win'; Fs = 'exfat'; Want = 'exFAT'; Name = 'pct-exfat' }
)

# diskpart reports most failures in its text, not in its exit code: show it and look for errors.
function Invoke-Diskpart([string[]]$Lines) {
    $script = Join-Path $root 'diskpart.txt'
    Set-Content -Path $script -Value $Lines -Encoding ascii
    $out = & diskpart /s $script | Out-String
    Write-Host $out
    if ($LASTEXITCODE -ne 0 -or $out -match 'DiskPart has encountered an error|Virtual Disk Service error') {
        throw "diskpart failed"
    }
}

function Show-Volume($d) {
    $vol = Get-Volume -FilePath $d.Mount
    Write-Host "$($d.Kind) at $($d.Mount): Get-Volume FileSystem=$($vol.FileSystem) Size=$($vol.Size)"
    fsutil fsinfo volumeinfo $d.Mount
    if ($vol.FileSystem -ne $d.Want) {
        throw "$($d.Kind) at $($d.Mount) reports '$($vol.FileSystem)', expected '$($d.Want)'"
    }
}

foreach ($d in $drives) {
    $d.Vhd = Join-Path $root "$($d.Name).vhd"
    $d.Mount = Join-Path $root $d.Name
}

if ($Action -eq 'setup') {
    $dirs = ''
    foreach ($d in $drives) {
        New-Item -ItemType Directory -Force -Path $d.Mount | Out-Null
        Invoke-Diskpart @(
            "create vdisk file=`"$($d.Vhd)`" maximum=256 type=expandable",
            "select vdisk file=`"$($d.Vhd)`"",
            'attach vdisk',
            'create partition primary',
            "format fs=$($d.Fs) quick label=PCTWIN",
            "assign mount=`"$($d.Mount)`"",
            'exit')
        Show-Volume $d
        $dirs += "|$($d.Kind)=$($d.Mount)"
    }
    New-Item -ItemType Directory -Force -Path (Join-Path $root 'fat-state') | Out-Null
    "PCTWIN_FAT_DIRS=$dirs" | Out-File -FilePath $env:GITHUB_ENV -Append -Encoding utf8
    "PCTWIN_FAT_STATE=$(Join-Path $root 'fat-state')" | Out-File -FilePath $env:GITHUB_ENV -Append -Encoding utf8
}
else {
    Write-Host '== volumes before'
    Get-Volume | Format-Table -AutoSize | Out-String | Write-Host
    foreach ($d in $drives) {
        Set-Content -Path (Join-Path $d.Mount '.pctwin-remount-marker') -Value 'kept' -Encoding ascii
        Invoke-Diskpart @("select vdisk file=`"$($d.Vhd)`"", 'detach vdisk', 'exit')
        # The folder is an ordinary empty folder again while the disk is detached.
        $gone = Get-Volume -FilePath $d.Mount
        if ($gone.FileSystem -ne 'NTFS' -or (Test-Path (Join-Path $d.Mount '.pctwin-remount-marker'))) {
            throw "$($d.Mount) still shows the drive after detaching it"
        }
        Invoke-Diskpart @("select vdisk file=`"$($d.Vhd)`"", 'attach vdisk', 'exit')
        # A mount folder normally comes back with the partition; assign it again if it did not.
        $back = Get-Volume -FilePath $d.Mount
        if ($back.FileSystem -ne $d.Want) {
            Write-Host "mount folder did not come back by itself: assigning it again"
            Invoke-Diskpart @("select vdisk file=`"$($d.Vhd)`"", 'select partition 1', "assign mount=`"$($d.Mount)`"", 'exit')
        }
        Show-Volume $d
        if ((Get-Content (Join-Path $d.Mount '.pctwin-remount-marker') -Raw).Trim() -ne 'kept') {
            throw "the marker did not survive the remount of $($d.Mount)"
        }
        Write-Host "remounted $($d.Kind) at $($d.Mount): it was really detached and attached again"
    }
    Write-Host '== volumes after'
    Get-Volume | Format-Table -AutoSize | Out-String | Write-Host
}
