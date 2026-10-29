#!/usr/bin/env bash
# Checks that `unsafe` stays forbidden everywhere except in the named functions of two crates:
#   pctwin-lease      fn fcntl_int                        (approved by the user, October 2026)
#   pctwin-helper-os  the os_* functions listed below     (approved by the user, 20 October 2026)
#                     (os_token_elevated and os_session_on_screen added 27 October 2026, for the
#                     join of Phase 2.2 and 2.3; approved by the user, 27 October 2026)
#                     (os_win_verify_trust added 27 October 2026, second review findings B2 and
#                     B3: Windows' own signature check on a held file, replacing PowerShell;
#                     approved by the user, 27 October 2026)
#                     (os_disk_hotplug added 27 October 2026, second review low finding: whether
#                     a disk can be unplugged; approved by the user, 27 October 2026)
#                     (os_assoc_is_dangerous added 28 October 2026, fifth review residual of
#                     finding 1: Windows' own judgement of a file type; approved by the user,
#                     28 October 2026)
#                     (os_rm_holders added 29 October 2026, 2.5 decision on files live while
#                     their app is open: Windows Restart Manager names the app holding a file;
#                     asked for in the 2.6 wiring brief; the user's approval to be confirmed at
#                     its fresh-context review)
#
# Every other crate must say `[lints] workspace = true`, which brings in the workspace's
# `unsafe_code = "forbid"`. A crate that sets its own unsafe_code, or whose sources contain an
# allow/expect/warn of unsafe code, fails this check. The two exceptions may set
# unsafe_code = "deny"; each function on their list must exist exactly once and hold exactly one
# `expect(unsafe_code)` and exactly one `unsafe`, and no `unsafe` or loosening may appear anywhere
# else in the crate (an `unsafe fn` or `unsafe impl` outside those bodies fails too).
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

# The exceptions: crate -> the only functions in it that may hold unsafe code.
exception_fns() {
  case "$1" in
    pctwin-lease) echo "fcntl_int" ;;
    pctwin-helper-os) echo "os_shell_execute_runas os_wait os_create_pipe os_access_list os_accept os_client_process_id os_server_process_id os_open_process os_process_token os_client_token os_token_user os_image_path os_volume_drive_type os_native_machine os_net_user_add os_expire_password os_create_dir_with_sddl os_create_profile os_set_process_sddl os_daemon_register os_daemon_unregister os_daemon_status os_open_login_items os_od_create_user os_authorization_holds os_windows_dir os_machine_text os_token_elevated os_session_on_screen os_win_verify_trust os_disk_hotplug os_assoc_is_dangerous os_rm_holders" ;;
    *) echo "" ;;
  esac
}

# The workspace itself must still forbid it.
grep -Eq '^unsafe_code *= *"forbid"' Cargo.toml || bad "Cargo.toml no longer sets unsafe_code = \"forbid\""

# Reads Rust sources as code and reports, one per line:
#   loosen <file>:<line> <allow|expect|warn>   an attribute loosening unsafe_code
#   unsafe <file>:<line>                       the keyword `unsafe`
#   body <name> <file> <start> <end>           where each function named in FENCE_FNS has its
#                                              body (byte offsets)
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
      for my $name (split " ", ($ENV{FENCE_FNS} // "")) {
        while ($o =~ /\bfn\s+\Q$name\E\b/g) {
          my $open = index($o, "{", pos($o));
          next if $open < 0;
          my ($j, $d) = ($open + 1, 1);
          while ($j < length($o) && $d) {
            my $t = substr($o, $j, 1);
            $d++ if $t eq "{"; $d-- if $t eq "}"; $j++;
          }
          printf "body %s %s %d %d\n", $name, $file, $open, $j;
          pos($o) = $open + 1;
        }
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
  fns="$(exception_fns "$name")"
  report=""
  if [ "${#files[@]}" -gt 0 ]; then
    report="$(FENCE_FNS="$fns" scan "${files[@]}")" || bad "$dir: a source could not be read as Rust ($(echo "$report" | grep -E '^(unreadable|unclosed)' | head -3 | tr '\n' ' '))"
  fi

  if [ -n "$fns" ]; then
    # May set unsafe_code only to "deny".
    if grep -E '^unsafe_code *=' "$toml" | grep -vq '"deny"'; then bad "$toml: unsafe_code must be \"deny\" here"; fi
    loosen_other=$(echo "$report" | grep -E '^loosen ' | grep -Ev ' expect$' || true)
    [ -z "$loosen_other" ] || bad "$dir: allow/warn of unsafe code (only expect(unsafe_code) in a listed function is permitted): $loosen_other"
    for fn in $fns; do
      n_body=$(echo "$report" | grep -cE "^body $fn " || true)
      [ "$n_body" -eq 1 ] || bad "$dir: $n_body definitions of fn $fn found, exactly 1 is expected"
    done
    # Every unsafe and every expect must sit inside a listed body; each body holds exactly one of
    # each.
    counts=""
    while read -r _ where kind; do
      [ -n "$where" ] || continue
      f="${where%:*}"; off="${where##*:}"
      owner=""
      while read -r _ bname bfile bstart bend; do
        [ -n "$bname" ] || continue
        if [ "$f" = "$bfile" ] && [ "$off" -gt "$bstart" ] && [ "$off" -lt "$bend" ]; then owner="$bname"; fi
      done < <(echo "$report" | grep -E '^body ')
      if [ -z "$owner" ]; then
        bad "$dir: $kind at $where is outside the listed functions"
      else
        counts="$counts$owner $kind
"
      fi
    done < <(echo "$report" | grep -E '^at ')
    for fn in $fns; do
      nu=$(printf '%s' "$counts" | grep -cxF "$fn unsafe" || true)
      ne=$(printf '%s' "$counts" | grep -cxF "$fn loosen-expect" || true)
      [ "$nu" -eq 1 ] || bad "$dir: fn $fn holds $nu uses of unsafe, exactly 1 is permitted"
      [ "$ne" -eq 1 ] || bad "$dir: fn $fn holds $ne expect(unsafe_code), exactly 1 is permitted"
    done
    continue
  fi

  # Everyone else: lints come from the workspace, and nothing in the file overrides unsafe_code.
  awk '/^\[/{s=($0=="[lints]")} s && /^workspace *= *true/{ok=1} END{exit !ok}' "$toml" \
    || bad "$toml: needs [lints] workspace = true"
  if grep -Eq 'unsafe' "$toml"; then bad "$toml: mentions unsafe in its lints"; fi
  loosen=$(echo "$report" | grep -E '^loosen ' || true)
  [ -z "$loosen" ] || bad "$dir: sources loosen unsafe_code: $(echo "$loosen" | tr '\n' ' ')"
done

[ "$fail" -eq 0 ] && echo "unsafe fence OK: unsafe is forbidden everywhere except fn fcntl_int in pctwin-lease and the listed os_* functions in pctwin-helper-os."
exit $fail
