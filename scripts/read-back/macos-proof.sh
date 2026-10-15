#!/usr/bin/env bash
# macOS: does the read-back really read the drive? Prints numbers; does not assert them.
# For APFS (the runner's drive) and hdiutil ExFAT and FAT32 images:
#   - how much of the file is in the cache right after the gate wrote it (mincore),
#   - the same after the read-back (verify),
#   - the bytes the disk driver reports reading around the read-back, and around an ordinary read,
#   - the contrast: a file written the ordinary way (no F_NOCACHE) and read again.
# Fails only on a crash or a clean file reported damaged.
# usage: macos-proof.sh <read_back_tool> <work dir> <resident.c> [summary file]
set -euo pipefail
TOOL=$1
WORK=$2
RESIDENT_C=$3
SUMMARY=${4:-/dev/null}
HERE=$(cd "$(dirname "$0")" && pwd)
SEED=20261018
SIZE=$((256 * 1024 * 1024))
mkdir -p "$WORK"
cc -O2 -o "$WORK/resident" "$RESIDENT_C"

field() { python3 -I "$HERE/jf.py" "$1"; }

# Bytes read, summed over every block storage driver, since boot.
io_read() {
  ioreg -c IOBlockStorageDriver -r -w 0 | grep -o '"Bytes (Read)"=[0-9]*' | cut -d= -f2 | awk '{ s += $1 } END { printf "%d\n", s }'
}
pct() { "$WORK/resident" "$1" | sed -n 's/.*percent=//p'; }

fail=0
{
  echo "### macOS: does the read-back reach the drive?"
  echo
  echo "| Drive | Probe says | In cache after the gate wrote it | Verify read from | In cache after verify | Bytes read around verify | Bytes read around an ordinary read | Bad blocks |"
  echo "| --- | --- | --- | --- | --- | --- | --- | --- |"
} >>"$SUMMARY"

run_case() { # name dir
  local name=$1 dir=$2
  echo "::group::$name ($dir)"
  mount | grep -F "$dir" || true
  local p w stored v r_write r_verify r_after io0 io1 io2 bad from
  p=$("$TOOL" probe "$dir")
  echo "$p"
  w=$("$TOOL" write "$dir" case/one.bin "$SIZE" "$SEED")
  echo "$w"
  stored=$(echo "$w" | sed -n 's/^stored=//p')
  r_write=$(pct "$dir/$stored")
  echo "resident after write: $r_write %"
  io0=$(io_read)
  v=$("$TOOL" verify "$dir" case/one.bin "$stored" "$SIZE" "$SEED")
  io1=$(io_read)
  echo "$v"
  r_verify=$(pct "$dir/$stored")
  echo "resident after verify: $r_verify %"
  # An ordinary read of the same file, now.
  "$TOOL" cached "$dir" "$stored" "$SIZE" "$SEED" >/dev/null
  io2=$(io_read)
  bad=$(echo "$v" | field bad_blocks)
  from=$(echo "$v" | field from)
  echo "| $name | $(echo "$p" | field from) | $r_write % | $from | $r_verify % | $((io1 - io0)) | $((io2 - io1)) | $bad |" >>"$SUMMARY"
  if [ "$bad" != 0 ]; then
    echo "::error::clean file on $name reported $bad bad blocks"
    fail=1
  fi

  # Contrast: written the ordinary way, the file sits in the cache.
  head -c "$SIZE" /dev/zero >"$dir/plain.bin"
  r_after=$(pct "$dir/plain.bin")
  io0=$(io_read)
  cat "$dir/plain.bin" >/dev/null
  io1=$(io_read)
  echo "contrast (plain write, no F_NOCACHE): resident $r_after %, bytes read by cat $((io1 - io0))"
  echo "|  contrast, plain write on $name | - | $r_after % | - | - | - | $((io1 - io0)) | - |" >>"$SUMMARY"
  echo "::endgroup::"
}

# APFS: the runner's own drive.
mkdir -p "$WORK/apfs"
run_case "APFS" "$WORK/apfs"

# hdiutil images.
for fs in ExFAT "MS-DOS FAT32"; do
  name="PCT$(echo "$fs" | tr -dc 'A-Z' | head -c 5)"
  dmg="$WORK/$name.dmg"
  mnt="$WORK/$name"
  hdiutil create -size 512m -fs "$fs" -volname "$name" "$dmg" >/dev/null
  mkdir -p "$mnt"
  hdiutil attach "$dmg" -mountpoint "$mnt" >/dev/null
  run_case "$fs" "$mnt" || fail=1
  hdiutil detach "$mnt" >/dev/null || true
done
exit "$fail"
