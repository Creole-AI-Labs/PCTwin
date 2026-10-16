#!/usr/bin/env bash
# "Nothing forgotten" on a real machine (Engineering Plan 5.1 and 9.16), Linux and macOS.
#
# Makes four real local accounts (ada the mover, bea another person, cal with a locked home, dee
# removed but leaving her home behind), a second real volume, and plants the answer-key things
# as each account so the system records the owners. Then ada runs the real drive-wide sweep, and
# the answer key is checked: nothing of ada's or the shared things forgotten, nothing of the
# others' named, nothing invented, the old laptop untouched.
#
# Needs passwordless sudo (as on CI runners). Usage: run.sh <folder with pctwin-lab and
# pctwin-sweep> [work folder] [seed]. Evidence is left in <work>/report.
set -uo pipefail

bin="$1"
work="${2:-/tmp/pctwin-nf}"
seed="${3:-1}"
os="$(uname)"
report="$work/report"
fail=0

note() { echo "== $*"; }
bad() { echo "::error::$*"; fail=1; }

acct() { echo "pctwin-nf-$1"; }
if [ "$os" = Darwin ]; then
  home_of() { echo "/Users/$(acct "$1")"; }
else
  home_of() { echo "/home/$(acct "$1")"; }
fi

# Each account runs with its own settings: the runner's XDG folders must not leak in.
as_user() {
  local u="$1"
  shift
  sudo -u "$u" -H env -u XDG_CONFIG_HOME -u XDG_DATA_HOME -u XDG_CACHE_HOME -u XDG_STATE_HOME \
    -u XDG_RUNTIME_DIR -u TMPDIR "$@"
}
as_label() {
  local l="$1"
  shift
  as_user "$(acct "$l")" "$@"
}

sudo rm -rf "$work"
sudo mkdir -p "$work/out" "$report"
sudo chown "$(id -u):$(id -g)" "$work"
chmod 755 "$work"
sudo chmod 1777 "$work/out" "$report"
finish() {
  sudo chown -R "$(id -u)" "$report" 2>/dev/null
  sudo chmod -R a+rX "$report" 2>/dev/null
}
trap finish EXIT

# Facts the summary step reads.
facts="$report/facts.json"
echo '{"labels": [], "os": "'"$os"'", "seed": '"$seed"'}' >"$facts"
label_fact() { # element status note
  python3 - "$facts" "$1" "$2" "$3" <<'PY'
import json, sys
f = json.load(open(sys.argv[1]))
f["labels"].append({"element": sys.argv[2], "status": sys.argv[3], "note": sys.argv[4]})
json.dump(f, open(sys.argv[1], "w"), indent=1)
PY
}
set_fact() { # key json-value
  python3 - "$facts" "$1" "$2" <<'PY'
import json, sys
f = json.load(open(sys.argv[1]))
f[sys.argv[2]] = json.loads(sys.argv[3])
json.dump(f, open(sys.argv[1], "w"), indent=1)
PY
}

sudo install -m 0755 "$bin/pctwin-lab" "$bin/pctwin-sweep" /usr/local/bin/

# 2. The plan.
note "plan (seed $seed)"
pctwin-lab plan "$seed" 4 "$report/spec.json" || exit 1
cp "$report/spec.json" "$work/spec.json"
python3 - "$report/spec.json" <<'PY'
import json, sys
s = json.load(open(sys.argv[1]))
print("people:", [(p["label"], p["role"]) for p in s["people"]])
print("plants:", len(s["plants"]))
PY

# 3. Real accounts, with real home folders made by the system.
make_user() {
  local u
  u="$(acct "$1")"
  if id "$u" >/dev/null 2>&1; then return; fi
  case "$os" in
    Linux) sudo useradd --create-home --shell /bin/bash "$u" ;;
    Darwin)
      # A throwaway password for a throwaway account on a throwaway machine; never printed.
      sudo sysadminctl -addUser "$u" -password "$(openssl rand -hex 16)" >/dev/null 2>&1
      sudo createhomedir -c -u "$u" >/dev/null 2>&1 || true
      ;;
  esac
}
for l in ada bea cal dee; do
  make_user "$l"
  [ -d "$(home_of "$l")" ] || { bad "no home folder for $l at $(home_of "$l")"; exit 1; }
done
sudo chmod 700 "$(home_of ada)" "$(home_of cal)" "$(home_of dee)"
# bea's home is the system's default: world-readable on a Mac, and made so on Linux (newer Ubuntu
# creates homes private) so her "Readable" plant is really readable by others.
sudo chmod 755 "$(home_of bea)"
for l in ada bea cal dee; do ls -ld "$(home_of "$l")"; done

# 4. A second real volume.
case "$os" in
  Linux)
    other=/mnt/pctwin-nf
    sudo rm -f /tmp/pctwin-nf.img
    sudo truncate -s 512M /tmp/pctwin-nf.img
    sudo mkfs.ext4 -q -F /tmp/pctwin-nf.img
    sudo mkdir -p "$other"
    sudo mount -o loop /tmp/pctwin-nf.img "$other"
    sysroot=/
    ;;
  Darwin)
    other=/Volumes/PCTwinNF
    sudo rm -f /tmp/pctwin-nf.dmg
    hdiutil create -size 200m -fs APFS -volname PCTwinNF /tmp/pctwin-nf.dmg >/dev/null
    hdiutil attach /tmp/pctwin-nf.dmg >/dev/null
    sudo diskutil enableOwnership "$other" || true
    # / is read-only on a Mac: the system drive's writable side is the data volume.
    sysroot=/System/Volumes/Data
    ;;
esac
sudo chmod 1777 "$other"
mount | grep -i "pctwin" || true
ls -ld "$other"

python3 - "$work" "$sysroot" "$other" "$(home_of ada)" "$(home_of bea)" "$(home_of cal)" "$(home_of dee)" <<'PY'
import json, sys
work, sysroot, other, a, b, c, d = sys.argv[1:8]
json.dump({"system_root": sysroot, "other_drive": other,
           "homes": {"ada": a, "bea": b, "cal": c, "dee": d}},
          open(work + "/roots.json", "w"), indent=1)
PY
cp "$work/roots.json" "$report/roots.json"
sudo chmod 644 "$work/spec.json" "$work/roots.json"

# One line per plant: token, owner, access, spot, whose home, top folder.
python3 - "$work/spec.json" <<'PY' >"$work/tops.tsv"
import json, sys
for p in json.load(open(sys.argv[1]))["plants"]:
    print(p["token"], p["by"], p["opens"], p["spot"], p.get("person", "-"), p["folder"].split("/")[0], sep="\t")
PY
top_dir() { # spot person top
  case "$1" in
    system-root) echo "$sysroot/$3" ;;
    other-drive) echo "$other/$3" ;;
    home) echo "$(home_of "$2")/$3" ;;
  esac
}

# Plants at the system root need their top folder made by root, then handed to the plant's owner.
while IFS=$'\t' read -r token by opens spot person top; do
  if [ "$spot" = system-root ]; then
    sudo mkdir -p "$sysroot/$top"
    sudo chown "$(acct "$by")" "$sysroot/$top"
  fi
done <"$work/tops.tsv"

# 5. Each account plants its own things, so the system records them as the owners. Dee's
# account is removed after, keeping her home folder: she is a leftover.
for l in ada bea cal dee; do
  note "plant as $l"
  as_label "$l" pctwin-lab plant "$work/spec.json" "$work/roots.json" "$l" || bad "planting as $l failed"
done
case "$os" in
  Linux) sudo userdel "$(acct dee)" || bad "userdel failed" ;;
  Darwin) sudo dscl . -delete "/Users/$(acct dee)" || bad "dscl delete failed" ;;
esac
if id "$(acct dee)" >/dev/null 2>&1; then bad "dee's account still exists"; fi
ls -ld "$(home_of dee)"

# The folders really are as refused / open as the plan says, for the mover. A refusal that cannot
# be produced is recorded as such, never faked.
note "the mover's real view"
echo '{"not_reachable": []}' >"$report/observed.json"
mover_acct="$(acct ada)"
while IFS=$'\t' read -r token by opens spot person top; do
  dir="$(top_dir "$spot" "$person" "$top")"
  if sudo -u "$mover_acct" ls "$dir" >/dev/null 2>&1; then listed=yes; else listed=no; fi
  echo "ada lists $dir ($by, $opens): $listed"
  if [ "$by" = ada ] && [ "$opens" = nobody ] && [ "$listed" = yes ]; then
    python3 - "$report/observed.json" "$token" <<'PY'
import json, sys
f = json.load(open(sys.argv[1])); f["not_reachable"].append(sys.argv[2]); json.dump(f, open(sys.argv[1], "w"))
PY
    label_fact "refused folder $token" "not reachable" "the mover could list a folder set to refuse everyone"
  fi
done <"$work/tops.tsv"
if sudo -u "$mover_acct" ls "$(home_of cal)" >/dev/null 2>&1; then
  bad "ada can list cal's locked home; the test would prove nothing"
fi
if sudo -u "$mover_acct" ls "$(home_of bea)" >/dev/null 2>&1; then
  label_fact "bea's home listable by others" real "world-readable home, the plant inside is still never named"
else
  label_fact "bea's home listable by others" "not reachable" "the system refused ada bea's home"
fi

# 7. macOS privacy: does a Mac privacy setting refuse a command-line program in its own home?
privacy="none"
if [ "$os" = Darwin ]; then
  note "macOS privacy probe (as ada)"
  refused=""
  for d in Library/Messages Library/Mail Library/Safari Library/Calendars "Library/Application Support/AddressBook"; do
    as_label ada mkdir -p "$(home_of ada)/$d" 2>/dev/null || true
    out="$(as_label ada ls "$(home_of ada)/$d" 2>&1 >/dev/null)" && rc=0 || rc=$?
    echo "ada ls ~/$d -> rc=$rc $out"
    if [ "$rc" -ne 0 ] && echo "$out" | grep -qi "operation not permitted"; then refused="$refused $d"; fi
  done
  if [ -n "$refused" ]; then
    privacy="refused:$refused"
    label_fact "macOS privacy refusal" real "EPERM for ada on:$refused"
  else
    privacy="not-reachable"
    label_fact "macOS privacy refusal" "not reachable" "macOS privacy refusal: not reachable on this runner (nothing refused ada in her own home)"
  fi
else
  label_fact "macOS privacy refusal" "not reachable" "not a Mac"
fi
set_fact macos_privacy "\"$privacy\""

# 6. Before: the planted folders as an administrator sees them.
snap_all() { # outdir
  mkdir -p "$1"
  while IFS=$'\t' read -r token by opens spot person top; do
    sudo pctwin-lab snapshot "$(top_dir "$spot" "$person" "$top")" "$1/$token.json" \
      || bad "snapshot of $token failed"
  done <"$work/tops.tsv"
}
note "snapshot before"
snap_all "$report/before"

# 8. The mover sweeps the whole laptop, in a fresh process with her own environment.
note "sweep as ada"
start=$(date +%s)
as_label ada pctwin-sweep "$work/out/sweep.json" >"$report/sweep.stdout.txt" 2>"$report/sweep.stderr.txt"
sweep_rc=$?
end=$(date +%s)
wall=$((end - start))
cat "$report/sweep.stdout.txt"
head -50 "$report/sweep.stderr.txt"
set_fact wall_seconds "$wall"
set_fact sweep_exit "$sweep_rc"
if [ "$sweep_rc" -ne 0 ]; then
  bad "the sweep failed with exit $sweep_rc"
  exit 1
fi
sudo cp "$work/out/sweep.json" "$report/sweep.json"
sudo chmod 644 "$report/sweep.json"

# 9. After: the old laptop is unchanged.
note "snapshot after and compare"
snap_all "$report/after"
cmp_bad=0
: >"$report/compare.txt"
for f in "$report"/before/*.json; do
  t="$(basename "$f")"
  if ! pctwin-lab compare "$f" "$report/after/$t" >>"$report/compare.txt"; then cmp_bad=1; fi
done
if [ "$cmp_bad" -ne 0 ]; then
  bad "the sweep changed the old laptop"
  cat "$report/compare.txt"
fi
set_fact compare_clean "$([ "$cmp_bad" -eq 0 ] && echo true || echo false)"

# 10. The answer key, as an administrator who may look everywhere.
note "check-sweep"
sudo pctwin-lab check-sweep "$work/spec.json" "$work/roots.json" "$report/sweep.json" "$report/observed.json" \
  >"$report/check.txt" 2>&1
check_rc=$?
cat "$report/check.txt"
set_fact check_exit "$check_rc"
[ "$check_rc" -eq 0 ] || bad "check-sweep found problems"

# None of the other people's tokens may appear in what the sweep printed, or in its result.
leaks=0
for tok in $(python3 -c "
import json
s = json.load(open('$work/spec.json'))
print(' '.join(p['token'] for p in s['plants'] if p['by'] != s['people'][0]['label']))"); do
  if grep -q "$tok" "$report/sweep.stdout.txt" "$report/sweep.stderr.txt" "$report/sweep.json"; then
    echo "LEAK: $tok named in the sweep's output"
    leaks=$((leaks + 1))
  fi
done
set_fact leaks "$leaks"
[ "$leaks" -eq 0 ] || bad "$leaks other-people tokens leaked"

# 11. Evidence.
python3 - "$report" <<'PY'
import json, sys
r = sys.argv[1]
s = json.load(open(r + "/sweep.json"))
json.dump(s.get("census"), open(r + "/census.json", "w"), indent=1)
json.dump(s.get("top_line"), open(r + "/top_line.json", "w"))
PY
gzip -f "$report/sweep.json"
label_fact "accounts and home folders" real "four local accounts made by the system; dee removed, her home kept"
label_fact "second volume" real "$([ "$os" = Darwin ] && echo 'APFS disk image at /Volumes/PCTwinNF' || echo 'ext4 loop device at /mnt/pctwin-nf')"
label_fact "planted things" real "planted by each account, the system recorded the owners"
label_fact "snapshots before and after" real "the planted folders only (not the whole machine), taken as root"
exit "$fail"
