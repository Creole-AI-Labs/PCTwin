#!/usr/bin/env bash
# Which CI jobs a change needs (decided 26 October 2026). Reads the changed paths, one per line,
# on standard input; EVENT is the GitHub event name and FULL_LABEL is "true" when the pull request
# carries the full-check label. Writes key=value lines for the jobs to standard output.
#
# Pull requests get the quick set for the parts they touch. The merge queue, pushes to main, the
# nightly run and the full-check label get the heavy set. A change to the workflows, or to any
# path not named below, runs everything. Documentation alone runs nothing.
set -euo pipefail

app=false installer=false rust=false fuzz=false code=false everything=false files=0
while IFS= read -r f; do
  [ -n "$f" ] || continue
  files=$((files + 1))
  case "$f" in
    .github/*) everything=true ;;
    # What the installers are made from (bundle settings, Windows hooks, the Mac's lists).
    app/src-tauri/windows/* | app/src-tauri/bundle/* | app/src-tauri/tauri*.json | app/src-tauri/Info.plist | app/src-tauri/icons/*)
      app=true installer=true rust=true code=true ;;
    app/src-tauri/*) app=true rust=true code=true ;;
    app/*) app=true code=true ;;
    crates/* | Cargo.toml | Cargo.lock | rust-toolchain.toml | deny.toml | about.toml | about.hbs | \
      osv-scanner.toml | supply-chain/* | scripts/* | fixtures/* | .config/*)
      rust=true code=true ;;
    fuzz/*) fuzz=true code=true ;;
    docs/* | *.md | LICENSE | NOTICE) ;;
    *) everything=true ;;
  esac
done

heavy=false
case "${EVENT:-}" in
  pull_request) ;;
  merge_group) heavy=true ;;
  # Pushes to main, the nightly run and a run by hand: everything.
  *) everything=true ;;
esac
# A list GitHub cut short cannot be trusted to name every change.
[ "$files" -ge "${MOST_FILES:-3000}" ] && everything=true
[ "${FULL_LABEL:-false}" = true ] && everything=true
if $everything; then
  heavy=true app=true installer=true rust=true fuzz=true code=true
fi

all_os='["windows-latest","windows-11-arm","macos-latest","macos-15-intel","ubuntu-latest","ubuntu-24.04-arm"]'
if $heavy && $rust; then
  rust_os=$all_os
elif $heavy && $app; then
  # The screens are built into the app: build, package and open it once per system.
  rust_os='["windows-latest","macos-latest","ubuntu-latest"]'
elif $installer; then
  rust_os='["windows-latest"]'
else
  rust_os='[]'
fi

codeql=()
{ $rust || $fuzz; } && codeql+=('"rust"')
$app && codeql+=('"javascript-typescript"')
$everything && codeql+=('"actions"')

quick=false
{ $rust && ! $heavy; } && quick=true
fuzz_job=false
{ $fuzz || { $heavy && $rust; }; } && fuzz_job=true
heavy_rust=false
{ $heavy && $rust; } && heavy_rust=true
motion=false
{ $heavy && $app; } && motion=true

echo "heavy=$heavy"
echo "ui=$app"
echo "motion=$motion"
echo "rust_quick=$quick"
echo "rust_os=$rust_os"
echo "fuzz=$fuzz_job"
echo "real_drives=$heavy_rust"
echo "labs=$heavy_rust"
echo "security=$code"
echo "codeql=[$(IFS=,; echo "${codeql[*]}")]"
