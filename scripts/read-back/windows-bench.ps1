# Windows read-back benchmark: cached read-back vs the gate's uncached one (verify), with the raw
# physical disk read counter around each read phase, so the numbers show whether the read reached
# the disk. Fails only on things that are definitely wrong (tool crash, a clean file reported
# damaged, NTFS not read from the Drive, files said to be read from the drive while the disk read
# less than they hold, a compressed folder's files said to be read from the drive); never on
# timings.
param(
    [Parameter(Mandatory = $true)][string]$Tool,
    [Parameter(Mandatory = $true)][string]$Work,
    [long]$BigBytes = 1073741824,
    [int]$Count = 10000,
    [string]$Summary = ''
)
$ErrorActionPreference = 'Stop'
$seed = 20261015

function Get-RawRead {
    # Cumulative bytes read from all physical disks. The raw class holds the running total
    # (PERF_COUNTER_BULK_COUNT); the formatted class would give a per-second rate instead.
    $o = Get-CimInstance Win32_PerfRawData_PerfDisk_PhysicalDisk -Filter "Name='_Total'"
    [int64]$o.DiskReadBytesPersec
}

function Invoke-Tool([string[]]$ToolArgs) {
    $out = & $Tool @ToolArgs 2>&1
    if ($LASTEXITCODE -ne 0) { throw "read_back_tool $($ToolArgs -join ' ') failed: $out" }
    $line = @($out | Where-Object { "$_".StartsWith('{') })[-1]
    if (-not $line) { throw "no result line from read_back_tool $($ToolArgs -join ' '): $out" }
    $out | Where-Object { -not "$_".StartsWith('{') } | ForEach-Object { Write-Host "  $_" }
    Write-Host $line
    $obj = $line | ConvertFrom-Json
    $obj | Add-Member -NotePropertyName Lines -NotePropertyValue @($out) -Force
    $obj
}

# Runs a tool command between two reads of the raw counter.
function Measure-Phase([string]$Label, [string[]]$ToolArgs) {
    $a = Get-RawRead
    $r = Invoke-Tool $ToolArgs
    $b = Get-RawRead
    [pscustomobject]@{ Label = $Label; Result = $r; Lines = $r.Lines; PhysicalBytes = ($b - $a); CounterBefore = $a; CounterAfter = $b }
}

New-Item -ItemType Directory -Force -Path $Work | Out-Null
$dest = Join-Path $Work 'ntfs'
New-Item -ItemType Directory -Force -Path $dest | Out-Null
$driveLetter = (Split-Path -Qualifier (Resolve-Path $dest).Path).TrimEnd(':')
$fsName = try { (Get-Volume -DriveLetter $driveLetter).FileSystem } catch { 'unknown' }
Write-Host "Work folder $dest is on ${driveLetter}: ($fsName)"

# Does the counter behave as a running total? Two reads a few seconds apart must not go down.
$c0 = Get-RawRead; Start-Sleep -Seconds 3; $c1 = Get-RawRead
$idle = $c1 - $c0
Write-Host "Raw counter idle drift over 3 s: $idle bytes (before $c0, after $c1)"
$monotone = $c1 -ge $c0

$probe = Invoke-Tool @('probe', $dest)
if ($fsName -eq 'NTFS' -and $probe.from -ne 'Drive') {
    throw "probe on NTFS said '$($probe.from)', not Drive"
}

# One big file.
$big = 'big/one.bin'
$wp = Measure-Phase 'write 1 big file' @('write', $dest, $big, "$BigBytes", "$seed")
$storedBig = (@($wp.Lines | Where-Object { "$_".StartsWith('stored=') })[0]).Substring(7)
Write-Host "Big file stored as $storedBig"

$bigCached = Measure-Phase 'big, cached read-back (warm)' @('cached', $dest, $storedBig, "$BigBytes", "$seed")
$bigVerify = Measure-Phase 'big, uncached read-back (verify)' @('verify', $dest, $big, $storedBig, "$BigBytes", "$seed")

# Many small files.
$manifest = Join-Path $Work 'manifest.tsv'
$mw = Measure-Phase "write $Count small files" @('write-many', $dest, "$Count", "$seed", $manifest)
$manyCached = Measure-Phase "$Count small, cached read-back (warm)" @('cached-many', $dest, $manifest, "$seed")
$manyVerify = Measure-Phase "$Count small, uncached read-back (verify)" @('verify-many', $dest, $manifest, "$seed")

# A compressed folder: Windows reads its files through memory whatever the handle asks, so none
# may be said to be read from the drive (the first review's finding).
$packed = Join-Path $Work 'packed'
New-Item -ItemType Directory -Force -Path $packed | Out-Null
& compact.exe /c /q $packed | Out-Null
$packedOn = ((Get-Item $packed).Attributes -band [IO.FileAttributes]::Compressed) -ne 0
$packedManifest = Join-Path $Work 'packed.tsv'
if ($packedOn) {
    $null = Invoke-Tool @('write-many', $packed, '1000', "$seed", $packedManifest)
    $packedVerify = Measure-Phase '1000 small, compressed folder (verify)' @('verify-many', $packed, $packedManifest, "$seed")
} else {
    Write-Host "This drive cannot compress folders; the compressed case is not run."
}

$phases = @($bigCached, $bigVerify, $manyCached, $manyVerify)
if ($packedOn) { $phases += $packedVerify }

$rows = foreach ($p in $phases) {
    $mb = [math]::Round($p.PhysicalBytes / 1MB, 1)
    $rate = [math]::Round($p.Result.mb_per_s, 0)
    $src = if ($p.Result.from) { $p.Result.from } else { 'cache (ordinary read)' }
    "| $($p.Label) | $src | $($p.Result.bad_blocks) | $([math]::Round($p.Result.seconds, 2)) s | $rate MB/s | $mb MiB |"
}
$table = @(
    "### Windows read-back benchmark ($fsName, $([math]::Round($BigBytes / 1MB)) MiB file and $Count small files)",
    '',
    '| Phase | Checked from | Bad blocks | Time | Speed | Physical disk MiB read |',
    '| --- | --- | --- | --- | --- | --- |'
) + $rows + @(
    '',
    "Probe on ${dest}: **$($probe.from)**. Raw counter: idle drift $idle bytes over 3 s, monotone: $monotone. A cached read should show about 0 physical bytes and the uncached one about the data size (the counter also sees other programs on the runner)."
)
$table | ForEach-Object { Write-Host $_ }
if ($Summary) { $table | Out-File -FilePath $Summary -Append -Encoding utf8 }

# Definitely wrong: files said to be read from the drive while the disk read less than they hold
# (the counter also counts other programs, so it can only be more, never less).
foreach ($p in @($bigVerify, $manyVerify) + $(if ($packedOn) { @($packedVerify) } else { @() })) {
    $claimed = [int64]$p.Result.drive_bytes
    if ($claimed -gt 0 -and $p.PhysicalBytes -lt [int64]($claimed * 0.9)) {
        throw "$($p.Label): $claimed bytes said to be read from the drive, but the disk read only $($p.PhysicalBytes)"
    }
}
if ($packedOn -and [int64]$packedVerify.Result.drive_bytes -ne 0) {
    throw "compressed folder: $($packedVerify.Result.drive_bytes) bytes said to be read from the drive"
}

# Definitely wrong: a clean file reported damaged.
foreach ($p in $phases) {
    if ($p.Result.bad_blocks -ne 0) { throw "$($p.Label): $($p.Result.bad_blocks) bad blocks on a clean file" }
}
