# Windows: exFAT and FAT32 volumes made from VHD files (diskpart, needs an administrator, which
# the runner is). On each: what the read-back probe says, and a smaller verify. Prints the
# answers; fails only on a crash or a clean file reported damaged.
param(
    [Parameter(Mandatory = $true)][string]$Tool,
    [Parameter(Mandatory = $true)][string]$Work,
    [long]$Bytes = 67108864,
    [string]$Summary = ''
)
$ErrorActionPreference = 'Stop'
$seed = 20261016

$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) {
    Write-Host '::notice::not an administrator; the exFAT and FAT32 volumes are left to the runner'
    exit 0
}

function Invoke-Tool([string[]]$ToolArgs) {
    $out = & $Tool @ToolArgs 2>&1
    if ($LASTEXITCODE -ne 0) { throw "read_back_tool $($ToolArgs -join ' ') failed: $out" }
    $line = @($out | Where-Object { "$_".StartsWith('{') })[-1]
    Write-Host $line
    $obj = $line | ConvertFrom-Json
    $obj | Add-Member -NotePropertyName Lines -NotePropertyValue @($out) -Force
    $obj
}

function Invoke-Diskpart([string]$Script) {
    $f = Join-Path $Work 'diskpart.txt'
    Set-Content -Path $f -Value $Script -Encoding ascii
    $out = diskpart /s $f
    $out | ForEach-Object { Write-Host "  $_" }
    if ($LASTEXITCODE -ne 0) { throw 'diskpart failed' }
}

New-Item -ItemType Directory -Force -Path $Work | Out-Null
$rows = @()
$used = @((Get-PSDrive -PSProvider FileSystem).Name)
$letters = [char[]]'ZYXWVUTSRQ' | Where-Object { $used -notcontains "$_" }
$i = 0
foreach ($fs in @('exfat', 'fat32')) {
    $letter = $letters[$i]; $i++
    $vhd = Join-Path $Work "$fs.vhd"
    $mb = if ($fs -eq 'fat32') { 512 } else { 256 }
    $attached = $false
    try {
        Invoke-Diskpart @"
create vdisk file="$vhd" maximum=$mb type=expandable
select vdisk file="$vhd"
attach vdisk
create partition primary
format fs=$fs quick label=PCT$($fs.ToUpper())
assign letter=$letter
"@
        $attached = $true
        $dest = "${letter}:\dest"
        New-Item -ItemType Directory -Force -Path $dest | Out-Null
        $probe = Invoke-Tool @('probe', $dest)
        $wr = Invoke-Tool @('write', $dest, 'a/f.bin', "$Bytes", "$seed")
        $stored = (@($wr.Lines | Where-Object { "$_".StartsWith('stored=') })[0]).Substring(7)
        $v = Invoke-Tool @('verify', $dest, 'a/f.bin', $stored, "$Bytes", "$seed")
        if ($v.bad_blocks -ne 0) { throw "$fs : clean file reported $($v.bad_blocks) bad blocks" }
        $rows += "| $fs | $($probe.from) | $($v.from) | $($v.bad_blocks) | $([math]::Round($v.seconds, 2)) s |"
    }
    catch {
        $rows += "| $fs | error | error | - | $($_.Exception.Message) |"
        throw
    }
    finally {
        if ($attached) {
            try { Invoke-Diskpart "select vdisk file=`"$vhd`"`ndetach vdisk" } catch { Write-Host "detach failed: $_" }
        }
    }
}
$table = @(
    '### Windows exFAT and FAT32 volumes (VHD)',
    '',
    '| Volume | Probe says | Verify read from | Bad blocks | Time |',
    '| --- | --- | --- | --- | --- |'
) + $rows
$table | ForEach-Object { Write-Host $_ }
if ($Summary) { $table | Out-File -FilePath $Summary -Append -Encoding utf8 }
