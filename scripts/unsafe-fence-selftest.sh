#!/usr/bin/env bash
# Proves scripts/unsafe-fence.sh catches what it must, including attributes spread over several
# lines, and still passes the real repository. Each case builds a small copy of the parts the
# fence reads, changes one thing, and checks the fence's answer.
#
# Usage: scripts/unsafe-fence-selftest.sh [fence script]   (default: the one beside this script)
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/.." && pwd)"
fence="${1:-$here/unsafe-fence.sh}"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
failed=0

# A copy of what the fence reads: the workspace manifest and every crate's manifest and sources.
fresh() {
  rm -rf "$work/r"; mkdir -p "$work/r/crates"
  cp "$repo/Cargo.toml" "$work/r/"
  for dir in "$repo"/crates/*/; do
    name="$(basename "$dir")"
    mkdir -p "$work/r/crates/$name"
    cp "$dir/Cargo.toml" "$work/r/crates/$name/"
    for d in src tests benches examples; do [ -d "$dir/$d" ] && cp -r "$dir/$d" "$work/r/crates/$name/"; done
    [ -f "$dir/build.rs" ] && cp "$dir/build.rs" "$work/r/crates/$name/"
  done
}

expect() { # expect pass|fail "what"
  local want="$1" what="$2" got
  if bash "$fence" "$work/r" >"$work/out" 2>&1; then got=pass; else got=fail; fi
  if [ "$got" = "$want" ]; then
    echo "ok: $what ($got)"
  else
    echo "WRONG: $what: wanted $want, got $got"; sed 's/^/    /' "$work/out"; failed=1
  fi
}

lease="crates/pctwin-lease/src/lib.rs"

fresh
expect pass "the real repository"

fresh
cat >>"$work/r/$lease" <<'EOF'

#[expect(
    unsafe_code,
    reason = "a second, multi-line exception"
)]
pub fn extra() -> i32 {
    unsafe { libc::getpid() }
}
EOF
expect fail "a second expect(unsafe_code) spread over lines in pctwin-lease"

fresh
cat >>"$work/r/$lease" <<'EOF'

#[allow(
    unsafe_code
)]
pub fn extra() {}
EOF
expect fail "an allow(unsafe_code) spread over lines in pctwin-lease"

fresh
# The one exception moved out of fn fcntl_int: same count, wrong place.
perl -0pi -e 's/(fn fcntl_int)/fn fcntl_moved() -> i32 { 0 }\n    $1/' "$work/r/$lease"
perl -0pi -e 's/let r = unsafe \{ libc::fcntl\(fd, command, arg\) \};/let r = fcntl_moved();/' "$work/r/$lease"
perl -0pi -e 's/fn fcntl_moved\(\) -> i32 \{ 0 \}/#[expect(unsafe_code, reason = "moved")]\n    fn fcntl_moved() -> i32 { unsafe { libc::getpid() } }/' "$work/r/$lease"
perl -0pi -e 's/#\[expect\(\s*unsafe_code,\s*reason = "no safe binding[^"]*"\s*\)\]\s*//' "$work/r/$lease"
expect fail "the one unsafe moved outside fn fcntl_int"

fresh
cat >>"$work/r/$lease" <<'EOF'

pub struct Raw;
unsafe impl Send for Raw {}
EOF
expect fail "an unsafe impl beside the permitted one"

fresh
other="$(ls -d "$work"/r/crates/*/src | grep -v pctwin-lease | head -1)"
printf '\n#[cfg_attr(\n    test,\n    allow(\n        unsafe_code\n    )\n)]\nfn f() {}\n' >>"$other/lib.rs"
expect fail "a multi-line cfg_attr(allow(unsafe_code)) in another crate"

fresh
printf '\n// Mentions expect(unsafe_code) and an unsafe block in a comment only.\nconst NOTE: &str = "allow(unsafe_code) unsafe { }";\n' >>"$work/r/$lease"
expect pass "unsafe words in comments and strings do not count"

fresh
printf '\n/* never closed\n' >>"$work/r/$lease"
expect fail "a source that cannot be read as Rust fails closed"

[ "$failed" -eq 0 ] && echo "unsafe fence self-test OK"
exit $failed
