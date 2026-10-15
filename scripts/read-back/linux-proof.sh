#!/usr/bin/env bash
# Linux: proof that the read-back skips the page cache.
# A file is written through the gate onto a loop-mounted ext4 image and synced; one byte of it is
# then flipped in the BACKING IMAGE. The mounted file's page cache still has the old byte, the
# drive has the new one. An ordinary read must say "fine" (the cache answers); the read-back must
# say Drive and report exactly the block holding that byte.
# usage: linux-proof.sh <read_back_tool> <work dir> [summary file]
set -euo pipefail
TOOL=$1
WORK=$2
SUMMARY=${3:-/dev/null}
HERE=$(cd "$(dirname "$0")" && pwd)
SEED=20261017
SIZE=$((8 * 1024 * 1024))
BLOCK=$((1024 * 1024))
FLIP_AT=$((3 * BLOCK + 17))
WANT_BLOCK=3
mkdir -p "$WORK"
mounted=()
cleanup() {
  for m in "${mounted[@]}"; do sudo umount "$m" 2>/dev/null || true; done
}
trap cleanup EXIT

field() { python3 -I "$HERE/jf.py" "$1"; }

mount_image() { # fs -> prints the mount point; prints nothing when the runner cannot mount it
  local fs=$1 img="$WORK/$1.img" mnt="$WORK/$1"
  truncate -s 128M "$img"
  sudo "mkfs.$fs" "$img" >/dev/null
  mkdir -p "$mnt"
  if sudo mount -t "$fs" -o "loop,uid=$(id -u),gid=$(id -g)" "$img" "$mnt"; then
    mounted+=("$mnt")
    echo "$mnt"
  elif [ "$fs" = exfat ]; then
    echo "::notice::this runner cannot mount exFAT; vfat only here" >&2
  else
    echo "mount $fs failed" >&2
    return 1
  fi
}

# ---- ext4: the cache-bypass proof ----
img="$WORK/ext4.img"
mnt="$WORK/ext4"
truncate -s 128M "$img"
sudo mkfs.ext4 -q -F "$img"
mkdir -p "$mnt"
sudo mount -o loop "$img" "$mnt"
mounted+=("$mnt")
sudo chown "$(id -u):$(id -g)" "$mnt"
dev=$(findmnt -no SOURCE "$mnt")
devname=${dev#/dev/}
echo "ext4 image mounted from $dev"

out=$("$TOOL" probe "$mnt")
echo "$out"
probe_from=$(echo "$out" | field from)

out=$("$TOOL" write "$mnt" proof/one.bin "$SIZE" "$SEED")
echo "$out"
stored=$(echo "$out" | sed -n 's/^stored=//p')
sync
sync
sudo python3 -I "$HERE/flip_byte.py" "$img" "$mnt/$stored" "$FLIP_AT"

read0=$("$HERE/diskstat.sh" "$devname")
cached=$("$TOOL" cached "$mnt" "$stored" "$SIZE" "$SEED")
echo "$cached"
read1=$("$HERE/diskstat.sh" "$devname")
verify=$("$TOOL" verify "$mnt" proof/one.bin "$stored" "$SIZE" "$SEED")
echo "$verify"
read2=$("$HERE/diskstat.sh" "$devname")

cached_bad=$(echo "$cached" | field bad_blocks)
verify_bad=$(echo "$verify" | field bad_blocks)
verify_list=$(echo "$verify" | field bad_block_list)
verify_from=$(echo "$verify" | field from)
cached_phys=$((read1 - read0))
verify_phys=$((read2 - read1))

{
  echo "### Linux cache-bypass proof (ext4 on a loop device)"
  echo
  echo "One byte of a ${SIZE}-byte file was flipped in the drive image after the file was written and synced. The file's cached pages still hold the old byte."
  echo
  echo "| Read | Checked from | Bad blocks | Bad block list | Bytes read from the device |"
  echo "| --- | --- | --- | --- | --- |"
  echo "| Ordinary (cached) | cache | $cached_bad | - | $cached_phys |"
  echo "| Read-back (verify) | $verify_from | $verify_bad | $verify_list | $verify_phys |"
  echo
  echo "Expected: the ordinary read finds nothing (the cache answers); the read-back says Drive and reports block $WANT_BLOCK only. Probe on ext4: **$probe_from**."
} | tee -a "$SUMMARY"

fail=0
if [ "$verify_from" != Drive ]; then
  echo "::error::verify on ext4 said $verify_from, not Drive; the proof did not run"
  fail=1
elif [ "$verify_list" != "[$WANT_BLOCK]" ]; then
  echo "::error::verify reported $verify_list, expected [$WANT_BLOCK]: the flipped byte was not caught, or something else was reported"
  fail=1
fi
if [ "$cached_bad" != 0 ]; then
  echo "::warning::the ordinary read saw the damage too, so the page cache was not holding the file; the contrast is inconclusive"
fi

# ---- FAT drives: what does the probe say ----
{
  echo
  echo "### Linux FAT drives (loop)"
  echo
  echo "| Drive | Probe says | Verify read from | Bad blocks |"
  echo "| --- | --- | --- | --- |"
} >>"$SUMMARY"
sudo apt-get -o Acquire::Retries=3 -o Acquire::http::Timeout=30 -o Acquire::https::Timeout=30 install -y dosfstools exfatprogs >/dev/null
for fs in vfat exfat; do
  m=$(mount_image "$fs") || { fail=1; continue; }
  [ -n "$m" ] || { echo "| $fs | not mountable on this runner | - | - |" >>"$SUMMARY"; continue; }
  p=$("$TOOL" probe "$m")
  echo "$p"
  w=$("$TOOL" write "$m" fat/f.bin $((4 * BLOCK)) "$SEED")
  s=$(echo "$w" | sed -n 's/^stored=//p')
  v=$("$TOOL" verify "$m" fat/f.bin "$s" $((4 * BLOCK)) "$SEED")
  echo "$v"
  echo "| $fs | $(echo "$p" | field from) | $(echo "$v" | field from) | $(echo "$v" | field bad_blocks) |" >>"$SUMMARY"
  if [ "$(echo "$v" | field bad_blocks)" != 0 ]; then
    echo "::error::clean file on $fs reported damaged"
    fail=1
  fi
done
exit "$fail"
