#!/usr/bin/env bash
# Real FAT-family drives for the gate's real-drive tests, on a Linux runner.
#
# Usage: scripts/real-drives-linux.sh setup | remount
#   setup    makes three drives (vfat, the kernel's exFAT, exfat-fuse), mounts them under
#            $RUNNER_TEMP and exports PCTWIN_FAT_DIRS (kind=path, separated by |) to later steps.
#   remount  unmounts every drive and mounts it again at the same path with the same driver.
set -euo pipefail

root="${RUNNER_TEMP:?RUNNER_TEMP is not set}"
list="$root/fat-drives.txt" # one line per drive: kind|image|mountpoint
apt=(sudo apt-get -o Acquire::Retries=3 -o Acquire::http::Timeout=30 -o Acquire::https::Timeout=30)

# What the OS says each kind is mounted as (findmnt's FSTYPE; stat's name for it is printed too).
expected_type() {
  case "$1" in
    vfat) echo '^vfat$' ;;
    exfat) echo '^exfat$' ;;
    exfat-fuse) echo '^fuse' ;;
  esac
}

mount_one() {
  local kind="$1" img="$2" mnt="$3"
  mkdir -p "$mnt"
  case "$kind" in
    # Explicit UTF-8 and mixed-case short names, as desktop automounters (udisks2) mount it.
    vfat) sudo mount -t vfat -o "loop,uid=$(id -u),gid=$(id -g),shortname=mixed,utf8=1" "$img" "$mnt" ;;
    exfat) sudo mount -t exfat -o "loop,uid=$(id -u),gid=$(id -g),iocharset=utf8" "$img" "$mnt" ;;
    # The FUSE driver through its own helper, so it is the FUSE driver even with the kernel module
    # loaded. Run as the runner user: the mount is then the user's, with no allow_other needed.
    exfat-fuse) mount.exfat-fuse -o "uid=$(id -u),gid=$(id -g)" "$img" "$mnt" ;;
  esac
}

unmount_one() {
  local kind="$1" mnt="$2" i
  sync
  for i in 1 2 3 4 5; do
    if [ "$kind" = exfat-fuse ]; then
      fusermount -u "$mnt" && return 0
    else
      sudo umount "$mnt" && return 0
    fi
    sleep 2
  done
  return 1
}

# Prints and checks what the OS reports for the mount point.
verify() {
  local kind="$1" mnt="$2" fstype
  fstype="$(findmnt -no FSTYPE "$mnt" || true)"
  echo "$kind at $mnt: findmnt FSTYPE=$fstype, stat -f type=$(stat -f -c %T "$mnt")"
  echo "  options: $(findmnt -no OPTIONS "$mnt")"
  if ! echo "$fstype" | grep -Eq "$(expected_type "$kind")"; then
    echo "::error::$kind at $mnt is mounted as '$fstype', not the driver this drive is for"
    return 1
  fi
}

case "${1:-}" in
setup)
  "${apt[@]}" update
  "${apt[@]}" install -y dosfstools exfatprogs exfat-fuse
  # The runner's Azure kernel does not ship the exFAT module in its base package.
  if ! sudo modprobe exfat 2>/dev/null; then
    if "${apt[@]}" install -y "linux-modules-extra-$(uname -r)"; then
      sudo modprobe exfat || true
    else
      echo "::warning::no linux-modules-extra package for kernel $(uname -r), so no kernel exFAT driver here"
    fi
  fi
  if grep -qw exfat /proc/filesystems; then
    echo "kernel exFAT driver present: $(grep -w exfat /proc/filesystems)"
  fi
  : >"$list"
  dirs=""
  for kind in vfat exfat exfat-fuse; do
    img="$root/$kind.img"
    mnt="$root/$kind"
    if [ "$kind" = exfat ] && ! grep -qw exfat /proc/filesystems; then
      echo "::warning::kernel exFAT driver unavailable on $(uname -r): skipping the kernel exFAT drive (exfat-fuse still covers exFAT)"
      continue
    fi
    truncate -s 64M "$img"
    if [ "$kind" = vfat ]; then mkfs.vfat "$img"; else mkfs.exfat "$img"; fi
    mount_one "$kind" "$img" "$mnt"
    verify "$kind" "$mnt"
    echo "$kind|$img|$mnt" >>"$list"
    dirs="$dirs|$kind=$mnt"
  done
  mkdir -p "$root/fat-state"
  {
    echo "PCTWIN_FAT_DIRS=$dirs"
    echo "PCTWIN_FAT_STATE=$root/fat-state"
  } >>"$GITHUB_ENV"
  echo "== mounts"
  findmnt -t vfat,exfat,fuseblk,fuse.exfat || true
  ;;
remount)
  echo "== mount table before"
  findmnt -t vfat,exfat,fuseblk,fuse.exfat || true
  while IFS='|' read -r kind img mnt; do
    # A marker written from the shell must survive the round trip.
    echo "kept" >"$mnt/.pctwin-remount-marker"
    unmount_one "$kind" "$mnt"
    if mountpoint -q "$mnt"; then
      echo "::error::$mnt is still mounted after unmounting"
      exit 1
    fi
    if [ -e "$mnt/.pctwin-remount-marker" ]; then
      echo "::error::$mnt still shows the drive's files after it was unmounted"
      exit 1
    fi
    mount_one "$kind" "$img" "$mnt"
    verify "$kind" "$mnt"
    if [ "$(cat "$mnt/.pctwin-remount-marker")" != kept ]; then
      echo "::error::the marker did not survive the remount of $mnt"
      exit 1
    fi
    echo "remounted $kind at $mnt: it was really unmounted and mounted again"
  done <"$list"
  echo "== mount table after"
  findmnt -t vfat,exfat,fuseblk,fuse.exfat || true
  ;;
*)
  echo "usage: $0 setup|remount" >&2
  exit 2
  ;;
esac
