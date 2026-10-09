#!/usr/bin/env bash
# Checks that `unsafe` stays forbidden everywhere except in one crate, pctwin-lease.
#
# Every crate must say `[lints] workspace = true`, which brings in the workspace's
# `unsafe_code = "forbid"`. A crate that sets its own unsafe_code, or whose sources contain
# allow/expect/warn of unsafe code, fails this check. The one exception, pctwin-lease
# (approved by the user, October 2026), may set unsafe_code = "deny" and may have at most
# one `expect(unsafe_code)` in its sources.
#
# Usage: scripts/unsafe-fence.sh [repo folder]   (default: the repo this script is in)
set -u
root="${1:-$(cd "$(dirname "$0")/.." && pwd)}"
cd "$root" || exit 2
fail=0
bad() { echo "UNSAFE FENCE: $*" >&2; fail=1; }

# The workspace itself must still forbid it.
grep -Eq '^unsafe_code *= *"forbid"' Cargo.toml || bad "Cargo.toml no longer sets unsafe_code = \"forbid\""

# A line in the sources that loosens the rule (comments are not skipped: fail closed).
loosen='(allow|expect|warn)\(([^)]*[ ,(])?unsafe(_code)?'

for toml in crates/*/Cargo.toml app/src-tauri/Cargo.toml; do
  [ -f "$toml" ] || continue
  dir="$(dirname "$toml")"
  name="$(basename "$dir")"
  srcs=()
  for d in src tests benches examples; do [ -d "$dir/$d" ] && srcs+=("$dir/$d"); done
  [ -f "$dir/build.rs" ] && srcs+=("$dir/build.rs")

  if [ "$name" = "pctwin-lease" ]; then
    # May set unsafe_code only to "deny".
    if grep -E '^unsafe_code *=' "$toml" | grep -vq '"deny"'; then bad "$toml: unsafe_code must be \"deny\" here"; fi
    if grep -rEq '(allow|warn)\(([^)]*[ ,(])?unsafe(_code)?' "${srcs[@]}" 2>/dev/null; then
      bad "$dir: allow/warn of unsafe code (only one expect(unsafe_code) is permitted)"
    fi
    n=$(grep -rE 'expect\(([^)]*[ ,(])?unsafe(_code)?' "${srcs[@]}" 2>/dev/null | wc -l)
    [ "$n" -le 1 ] || bad "$dir: $n expect(unsafe_code) found, at most 1 is permitted"
    continue
  fi

  # Everyone else: lints come from the workspace, and nothing in the file overrides unsafe_code.
  awk '/^\[/{s=($0=="[lints]")} s && /^workspace *= *true/{ok=1} END{exit !ok}' "$toml" \
    || bad "$toml: needs [lints] workspace = true"
  if grep -Eq 'unsafe' "$toml"; then bad "$toml: mentions unsafe in its lints"; fi
  if [ "${#srcs[@]}" -gt 0 ] && grep -rEn "$loosen" "${srcs[@]}" >&2; then
    bad "$dir: sources loosen unsafe_code (lines above)"
  fi
done

[ "$fail" -eq 0 ] && echo "unsafe fence OK: unsafe is forbidden everywhere except pctwin-lease."
exit $fail
