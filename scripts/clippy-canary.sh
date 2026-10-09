#!/usr/bin/env bash
# Proves the transfer crate's file-change fence still fires.
# It runs clippy on a deliberately bad crate (fixtures/clippy-canary) using the REAL
# crates/pctwin-transfer/clippy.toml. Clippy must fail, and must name each banned call.
# If clippy passes, or misses one, the fence is broken and this script exits non-zero.
set -u
root="$(cd "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cp -r "$root/fixtures/clippy-canary/." "$work/"
cp "$root/crates/pctwin-transfer/clippy.toml" "$work/clippy.toml"
cp "$root/rust-toolchain.toml" "$work/"

out="$work/out.txt"
(cd "$work" && cargo clippy --all-targets --quiet 2>&1) >"$out"
status=$?
cat "$out"

if [ "$status" -eq 0 ]; then
  echo "CANARY FAILED: clippy accepted code that removes a file. The fence is not firing." >&2
  exit 1
fi
missing=0
for m in 'std::fs::remove_file' 'std::fs::rename' 'std::fs::File::create' 'std::fs::OpenOptions::write'; do
  if ! grep -q "use of a disallowed method \`$m\`" "$out"; then
    echo "CANARY FAILED: clippy did not reject $m" >&2
    missing=1
  fi
done
[ "$missing" -eq 0 ] || exit 1
echo "CANARY OK: clippy rejected all four banned calls."
