#!/usr/bin/env bash
# Real FAT-family drives for the gate's real-drive tests, on a Mac runner.
#
# Usage: scripts/real-drives-mac.sh setup | remount
#   setup    makes a FAT32 and an exFAT disk image, attaches each at a fixed folder under
#            $RUNNER_TEMP and exports PCTWIN_FAT_DIRS (kind=path, separated by |) to later steps.
#   remount  detaches every image and attaches it again at the same folder.
set -euo pipefail

root="${RUNNER_TEMP:?RUNNER_TEMP is not set}"
list="$root/fat-drives.txt" # one line per drive: kind|image|mountpoint|expected file system

attach() {
  mkdir -p "$3"
  hdiutil attach "$2" -mountpoint "$3" -nobrowse
  # Spotlight indexing a fresh volume is what holds it busy when it is detached.
  sudo mdutil -i off "$3" >/dev/null 2>&1 || true
}

# The whole disk an image is attached as (/dev/diskN), or nothing if it is not attached.
device_of() {
  local img="$1"
  hdiutil info | awk -v img="$img" '
    /^image-path/ { found = (index($0, img) > 0) }
    found && /^\/dev\/disk[0-9]+[ 	]/ { print $1; exit }'
}

# Detaches the image `img` (mounted at `mnt`). A busy volume can be left half-detached: its mount
# point gone, its disk still attached, so every later try works by the disk, not the mount point.
# Waits, then forces; fails loudly, with who holds it, if the image is still attached.
detach() {
  local img="$1" mnt="$2" dev i out
  sync
  dev="$(device_of "$img")"
  if [ -z "$dev" ]; then
    echo "::error::$img is not attached, so it cannot be detached"
    return 1
  fi
  for i in 1 2 3 4 5; do
    if out="$(hdiutil detach "$dev" 2>&1)"; then
      echo "$out"
      break
    fi
    echo "detach $dev (try $i): $out"
    sleep 3
  done
  if [ -n "$(device_of "$img")" ]; then
    echo "::warning::$dev is still attached after five tries; what holds $mnt:"
    sudo lsof +f -- "$mnt" 2>/dev/null | head -20 || true
    sleep 5
    if ! out="$(hdiutil detach -force "$dev" 2>&1)"; then
      echo "::error::could not detach $dev ($img at $mnt) even by force: $out"
      return 1
    fi
    echo "$out"
  fi
  if [ -n "$(device_of "$img")" ]; then
    echo "::error::$img is still attached as $(device_of "$img") after detaching"
    return 1
  fi
}

verify() {
  local kind="$1" mnt="$2" want="$3" have
  echo "$kind at $mnt:"
  mount | grep -F " on $mnt " || true
  have="$(diskutil info "$mnt" | sed -n 's/^ *File System Personality: *//p')"
  echo "  File System Personality: $have"
  if [ "$have" != "$want" ]; then
    echo "::error::$kind at $mnt reports '$have', expected '$want'"
    return 1
  fi
}

case "${1:-}" in
setup)
  : >"$list"
  dirs=""
  for entry in "msdos-fat32|MS-DOS FAT32|MS-DOS FAT32|PCTFAT" "exfat-mac|ExFAT|ExFAT|PCTEXF"; do
    IFS='|' read -r kind fs want name <<<"$entry"
    img="$root/$name.dmg"
    mnt="$root/$name"
    hdiutil create -size 64m -fs "$fs" -volname "$name" "$img"
    attach "$kind" "$img" "$mnt"
    verify "$kind" "$mnt" "$want"
    echo "$kind|$img|$mnt|$want" >>"$list"
    dirs="$dirs|$kind=$mnt"
  done
  mkdir -p "$root/fat-state"
  {
    echo "PCTWIN_FAT_DIRS=$dirs"
    echo "PCTWIN_FAT_STATE=$root/fat-state"
  } >>"$GITHUB_ENV"
  ;;
remount)
  echo "== mount table before"
  mount | grep -F "$root" || true
  while IFS='|' read -r kind img mnt want; do
    echo "kept" >"$mnt/.pctwin-remount-marker"
    detach "$img" "$mnt"
    if mount | grep -qF " on $mnt "; then
      echo "::error::$mnt is still mounted after detaching"
      exit 1
    fi
    attach "$kind" "$img" "$mnt"
    verify "$kind" "$mnt" "$want"
    if [ "$(cat "$mnt/.pctwin-remount-marker")" != kept ]; then
      echo "::error::the marker did not survive the remount of $mnt"
      exit 1
    fi
    echo "remounted $kind at $mnt: it was really detached and attached again"
  done <"$list"
  echo "== mount table after"
  mount | grep -F "$root" || true
  ;;
*)
  echo "usage: $0 setup|remount" >&2
  exit 2
  ;;
esac
