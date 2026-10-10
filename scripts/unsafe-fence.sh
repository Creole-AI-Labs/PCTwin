#!/usr/bin/env bash
# Checks that `unsafe` stays forbidden everywhere except in one place: fn fcntl_int in
# pctwin-lease.
#
# Every crate must say `[lints] workspace = true`, which brings in the workspace's
# `unsafe_code = "forbid"`. A crate that sets its own unsafe_code, or whose sources contain an
# allow/expect/warn of unsafe code, fails this check. The one exception, pctwin-lease (approved by
# the user, October 2026), may set unsafe_code = "deny" and has exactly one `expect(unsafe_code)`
# and exactly one `unsafe`, both inside the body of `fn fcntl_int`.
#
# The sources are read as Rust, not line by line: comments, strings and character literals are
# set aside first, and an attribute counts however it is spread over lines, so
#     #[expect(
#         unsafe_code,
#         reason = "..."
#     )]
# is seen exactly as `#[expect(unsafe_code)]` is. scripts/unsafe-fence-selftest.sh proves it.
#
# Usage: scripts/unsafe-fence.sh [repo folder]   (default: the repo this script is in)
set -u
root="${1:-$(cd "$(dirname "$0")/.." && pwd)}"
cd "$root" || exit 2
fail=0
bad() { echo "UNSAFE FENCE: $*" >&2; fail=1; }

# The workspace itself must still forbid it.
grep -Eq '^unsafe_code *= *"forbid"' Cargo.toml || bad "Cargo.toml no longer sets unsafe_code = \"forbid\""

# Reads Rust sources as code and reports, one per line:
#   loosen <file>:<line> <allow|expect|warn>   an attribute loosening unsafe_code
#   unsafe <file>:<line>                       the keyword `unsafe`
#   body <file> <start> <end>                  where `fn fcntl_int`'s body is (byte offsets)
#   at <file>:<offset> <kind>                  the byte offset of each loosen/unsafe found
# Exits 3 if a file cannot be read as Rust (an unclosed comment or string): fail closed.
scan() {
  perl -e '
    use strict; use warnings;
    my $broken = 0;
    for my $file (@ARGV) {
      open(my $fh, "<", $file) or do { print "unreadable $file\n"; $broken = 1; next };
      local $/; my $s = <$fh>; close $fh;
      my $n = length $s; my $o = "";
      # Blanks out a piece, keeping its line breaks so line numbers stay true.
      my $blank = sub { my $t = shift; $t =~ s/[^\n]/ /g; $o .= $t };
      pos($s) = 0;
      while (pos($s) < $n) {
        if ($s =~ /\G(\/\/[^\n]*)/gc) { $blank->($1); next }
        if ($s =~ /\G\/\*/gc) {
          my $start = pos($s) - 2; my $d = 1;
          while ($d && $s =~ /\G(?:[^\/*]+|(\/\*)|(\*\/)|[\/*])/gc) {
            $d++ if defined $1; $d-- if defined $2;
          }
          if ($d) { print "unclosed-comment $file\n"; $broken = 1; pos($s) = $n }
          $blank->(substr($s, $start, pos($s) - $start)); next;
        }
        if ($s =~ /\G((?<![A-Za-z0-9_])b?r(#*)")/gc) {
          my ($start, $close) = (pos($s) - length $1, "\"" . $2);
          my $j = index($s, $close, pos($s));
          if ($j < 0) { print "unclosed-string $file\n"; $broken = 1; $j = $n } else { $j += length $close }
          $blank->(substr($s, $start, $j - $start)); pos($s) = $j; next;
        }
        if ($s =~ /\G("(?:[^"\\]+|\\.)*")/gcs) { $blank->($1); next }
        if ($s =~ /\G"/gc) { print "unclosed-string $file\n"; $broken = 1; $blank->(substr($s, pos($s) - 1)); pos($s) = $n; next }
        if ($s =~ /\G(\x27(?:\\u\{[0-9A-Fa-f]+\}|\\.|[^\\\x27\n])\x27)/gc) { $blank->($1); next }
        if ($s =~ /\G([^\/"\x27br]+|.)/gcs) { $o .= $1; next }
      }
      my $line = sub { my $p = shift; return 1 + (substr($o, 0, $p) =~ tr/\n//) };
      # Lint attributes: allow( / expect( / warn( with their whole bracketed content, however
      # many lines it spans.
      while ($o =~ /\b(allow|expect|warn)\s*\(/g) {
        my ($kind, $start, $end) = ($1, $-[0], $+[0]);
        my ($j, $d) = ($end, 1);
        while ($j < length($o) && $d) {
          my $t = substr($o, $j, 1);
          $d++ if $t eq "("; $d-- if $t eq ")"; $j++;
        }
        my $inside = substr($o, $end, $j - $end);
        if ($inside =~ /\bunsafe(_code)?\b/) {
          printf "loosen %s:%d %s\n", $file, $line->($start), $kind;
          printf "at %s:%d loosen-%s\n", $file, $start, $kind;
        }
        pos($o) = $end;
      }
      while ($o =~ /\bunsafe\b/g) {
        printf "unsafe %s:%d\n", $file, $line->($-[0]);
        printf "at %s:%d unsafe\n", $file, $-[0];
      }
      while ($o =~ /\bfn\s+fcntl_int\b/g) {
        my $open = index($o, "{", pos($o));
        next if $open < 0;
        my ($j, $d) = ($open + 1, 1);
        while ($j < length($o) && $d) {
          my $t = substr($o, $j, 1);
          $d++ if $t eq "{"; $d-- if $t eq "}"; $j++;
        }
        printf "body %s %d %d\n", $file, $open, $j;
      }
    }
    exit($broken ? 3 : 0);
  ' "$@"
}

for toml in crates/*/Cargo.toml app/src-tauri/Cargo.toml; do
  [ -f "$toml" ] || continue
  dir="$(dirname "$toml")"
  name="$(basename "$dir")"
  srcs=()
  for d in src tests benches examples; do [ -d "$dir/$d" ] && srcs+=("$dir/$d"); done
  [ -f "$dir/build.rs" ] && srcs+=("$dir/build.rs")
  files=()
  if [ "${#srcs[@]}" -gt 0 ]; then
    while IFS= read -r f; do files+=("$f"); done < <(find "${srcs[@]}" -type f -name '*.rs' | sort)
  fi
  report=""
  if [ "${#files[@]}" -gt 0 ]; then
    report="$(scan "${files[@]}")" || bad "$dir: a source could not be read as Rust ($(echo "$report" | grep -E '^(unreadable|unclosed)' | head -3 | tr '\n' ' '))"
  fi

  if [ "$name" = "pctwin-lease" ]; then
    # May set unsafe_code only to "deny".
    if grep -E '^unsafe_code *=' "$toml" | grep -vq '"deny"'; then bad "$toml: unsafe_code must be \"deny\" here"; fi
    loosen_other=$(echo "$report" | grep -E '^loosen ' | grep -Ev ' expect$' || true)
    [ -z "$loosen_other" ] || bad "$dir: allow/warn of unsafe code (only one expect(unsafe_code) is permitted): $loosen_other"
    n_expect=$(echo "$report" | grep -cE '^loosen .* expect$' || true)
    n_unsafe=$(echo "$report" | grep -cE '^unsafe ' || true)
    n_body=$(echo "$report" | grep -cE '^body ' || true)
    [ "$n_expect" -eq 1 ] || bad "$dir: $n_expect expect(unsafe_code) found, exactly 1 is permitted: $(echo "$report" | grep -E '^loosen ' | tr '\n' ' ')"
    [ "$n_unsafe" -eq 1 ] || bad "$dir: $n_unsafe uses of unsafe found, exactly 1 is permitted: $(echo "$report" | grep -E '^unsafe ' | tr '\n' ' ')"
    [ "$n_body" -eq 1 ] || bad "$dir: $n_body definitions of fn fcntl_int found, exactly 1 is expected"
    if [ "$n_body" -eq 1 ]; then
      read -r _ bfile bstart bend <<<"$(echo "$report" | grep -E '^body ')"
      while read -r _ where kind; do
        [ -n "$where" ] || continue
        f="${where%:*}"; off="${where##*:}"
        if [ "$f" != "$bfile" ] || [ "$off" -le "$bstart" ] || [ "$off" -ge "$bend" ]; then
          bad "$dir: $kind at $where is outside the body of fn fcntl_int"
        fi
      done < <(echo "$report" | grep -E '^at ')
    fi
    continue
  fi

  # Everyone else: lints come from the workspace, and nothing in the file overrides unsafe_code.
  awk '/^\[/{s=($0=="[lints]")} s && /^workspace *= *true/{ok=1} END{exit !ok}' "$toml" \
    || bad "$toml: needs [lints] workspace = true"
  if grep -Eq 'unsafe' "$toml"; then bad "$toml: mentions unsafe in its lints"; fi
  loosen=$(echo "$report" | grep -E '^loosen ' || true)
  [ -z "$loosen" ] || bad "$dir: sources loosen unsafe_code: $(echo "$loosen" | tr '\n' ' ')"
done

[ "$fail" -eq 0 ] && echo "unsafe fence OK: unsafe is forbidden everywhere except fn fcntl_int in pctwin-lease."
exit $fail
