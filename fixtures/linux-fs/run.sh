#!/usr/bin/env bash
# Runs PCTwin tests on real ext4, xfs and btrfs volumes, as an ordinary user (uid 1000).
#
# Why: file identity, birth times, hard links and removal behave differently on each Linux
# file system, and on a normal CI runner everything sits on one ext4 disk.
#
# Run it inside a privileged container (loop mounts need it), repo mounted read-only at /src:
#   docker run --rm --privileged -v "$PWD":/src:ro rust:latest \
#       bash /src/fixtures/linux-fs/run.sh [--prebuilt] [--fs "ext4 xfs"] <cargo test arguments>
#
# Examples:
#   ... run.sh -p pctwin-gate
#   ... run.sh --prebuilt -p pctwin-gate -p pctwin-journal -- identity
#
#  --prebuilt  build the test programs once as root, then run each one as the user. This avoids
#              the user needing write access to the build folder and the cargo home. (Doc tests
#              are not run in this mode.) Without it, `cargo test` itself runs as the user.
#  --fs "..."  only these file systems (default: ext4 xfs btrfs).
#
# Prints one PASS/FAIL line per file system and exits non-zero if any failed.
set -u

PREBUILT=0
FSLIST="ext4 xfs btrfs"
while [ $# -gt 0 ]; do
  case "$1" in
    --prebuilt) PREBUILT=1; shift ;;
    --fs) FSLIST="$2"; shift 2 ;;
    *) break ;;
  esac
done
ARGS=("$@")                       # everything else goes to cargo test
[ "${#ARGS[@]}" -gt 0 ] || ARGS=(--workspace)
# Split at a bare `--`: before it, cargo's own arguments; after it, test filters.
BUILD_ARGS=(); FILTER=(); seen=0
for a in "${ARGS[@]}"; do
  if [ "$seen" = 1 ]; then FILTER+=("$a"); elif [ "$a" = "--" ]; then seen=1; else BUILD_ARGS+=("$a"); fi
done

export CARGO_TARGET_DIR=/target
export CARGO_TERM_COLOR=never
SRC=/src

echo "== tools"
apt-get -qq update >/dev/null 2>&1
apt-get -qq install -y e2fsprogs xfsprogs btrfs-progs jq >/dev/null 2>&1 || { echo "apt failed"; exit 2; }

# The test user. su needs a shell and a home.
id -u u >/dev/null 2>&1 || useradd -m -u 1000 -s /bin/bash u

echo "== build (as root)"
cd "$SRC" || exit 2
# rust-toolchain.toml may name a newer Rust than the image: let rustup fetch it now, as root.
rustup show >/dev/null 2>&1
mkdir -p /target
if [ "$PREBUILT" = 1 ]; then
  # Test programs: path<TAB>package folder, one per line.
  cargo test --no-run --locked --message-format=json "${BUILD_ARGS[@]}" 2>/tmp/build.err \
    | jq -r 'select(.reason=="compiler-artifact" and .profile.test==true and .executable!=null)
             | "\(.executable)\t\(.manifest_path | sub("/Cargo.toml$"; ""))"' >/tmp/bins.txt
  [ -s /tmp/bins.txt ] || { cat /tmp/build.err; echo "build failed"; exit 2; }
  chmod -R a+rX /target
else
  # Cargo runs as the user: give it its own copy of the cargo home and a writable target.
  cp -a "${CARGO_HOME:-/usr/local/cargo}" /home/u/.cargo
  chown -R u:u /home/u/.cargo /target
fi

mkdir -p /img /m
overall=0
RESULTS=""

for fs in $FSLIST; do
  echo
  echo "===================== $fs ====================="
  truncate -s 300M "/img/$fs.img"
  case $fs in
    ext4)  mkfs.ext4 -q -F "/img/$fs.img" ;;
    xfs)   mkfs.xfs -q -f "/img/$fs.img" ;;      # new mkfs.xfs defaults to the format with birth times
    btrfs) mkfs.btrfs -q -f "/img/$fs.img" ;;
  esac || { echo "mkfs failed"; RESULTS="$RESULTS\n$fs: SETUP FAILED"; overall=1; continue; }
  mkdir -p "/m/$fs"
  mount -o loop "/img/$fs.img" "/m/$fs" || { echo "mount failed"; RESULTS="$RESULTS\n$fs: SETUP FAILED"; overall=1; continue; }
  mkdir -p "/m/$fs/d" && chown u:u "/m/$fs/d"

  # Do birth times really work here? (statx btime; "-" means no.)
  su u -c "touch /m/$fs/d/probe"
  echo "type=$(stat -f -c %T /m/$fs/d)  birth time of a fresh file: $(stat -c %w /m/$fs/d/probe)"
  rm -f "/m/$fs/d/probe"

  rc=0
  if [ "$PREBUILT" = 1 ]; then
    while IFS=$'\t' read -r exe dir; do
      echo "--- $(basename "$exe")"
      su u -c "cd '$dir' && TMPDIR=/m/$fs/d '$exe' ${FILTER[*]:-}" || rc=1
    done </tmp/bins.txt
  else
    su u -c "cd $SRC && TMPDIR=/m/$fs/d CARGO_TARGET_DIR=/target CARGO_TERM_COLOR=never cargo test --locked ${ARGS[*]}" || rc=1
  fi

  if [ "$rc" = 0 ]; then RESULTS="$RESULTS\n$fs: PASS"; else RESULTS="$RESULTS\n$fs: FAIL"; overall=1; fi
  umount "/m/$fs"
done

echo
echo "===================== results ====================="
printf "$RESULTS\n"
exit $overall
