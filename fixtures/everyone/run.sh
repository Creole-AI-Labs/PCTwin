#!/usr/bin/env bash
# Everyone in one move (Engineering Plan 5.1: "Several people in one move" and "Nothing
# forgotten"; Task List, moving everyone, step 8), Linux and macOS.
#
# Four people on one laptop: the mover (a fresh standard account), Bea and Carl (approved by the
# administrator) and Dan (not approved). About forty things are planted for them in ordinary and
# odd places; the answer key is the administrator's, where the mover cannot read it. This script
# is the administrator: it starts the helper the way the system's prompt would, from outside the
# product (there is no flag in the product to skip the prompt). Then:
#
# 1. A second helper while one runs is refused; another account calling the helper is refused.
# 2. The move, as the mover, with a planted link in a holding folder: each person's things land
#    in their own new account through the holding area and the hand-over; Dan's nowhere; the
#    mover's report and output never name another person's file; the old laptop is unchanged;
#    the link is never followed; nothing is invented; a blind spot is named and never hidden by
#    "everything is accounted for".
# 3. The mover cannot read the helper's memory (a debugger cannot attach).
# 4. macOS: each account PCTwin made says it has a Secure Token exactly when the system says so.
# 5. No administrator anywhere: the mover's own things all move; the others are listed, not
#    moved, and not named.
#
# Needs passwordless sudo (as on CI runners). Usage: run.sh <folder with pctwin-labmove and
# pctwin-helper> [work folder]. Leaves its evidence in <work>/admin.
set -euo pipefail

bin="$1"
work="${2:-/tmp/pctwin-everyone}"
here="$(cd "$(dirname "$0")" && pwd)"
mover=pctwinmover
people=(pctwinbea pctwincarl pctwindan)
mac=false
[ "$(uname)" = Darwin ] && mac=true

remove_user() {
  local u="$1"
  id "$u" >/dev/null 2>&1 || return 0
  if $mac; then sudo sysadminctl -deleteUser "$u" >/dev/null 2>&1 || true
  else sudo userdel --remove "$u" >/dev/null 2>&1 || true
  fi
}

make_user() {
  local u="$1"
  remove_user "$u"
  if $mac; then
    # A throwaway password for a throwaway account on a throwaway machine; never printed.
    sudo sysadminctl -addUser "$u" -password "$(openssl rand -hex 16)" >/dev/null 2>&1
    sudo createhomedir -c -u "$u" >/dev/null 2>&1 || true
  else
    sudo useradd --create-home --shell /bin/bash "$u"
  fi
}

for u in "$mover" "${people[@]}"; do make_user "$u"; done
for n in 1 2 3; do remove_user "pctwinnew$n"; done

# The helper runs the hand-over program from its own folder, as in the app.
sudo install -m 0755 "$bin/pctwin-labmove" "$bin/pctwin-helper" "$bin/pctwin-handover" /usr/local/bin/
sudo rm -rf "$work" /srv/pctwin-* /Users/Shared/pctwin-* 2>/dev/null || true
# The lab's own folder outside any home, open to everyone like the drive's top; each person's
# folder in it is theirs alone.
lab_root=/srv/pctwin-lab
$mac && lab_root=/Users/Shared/pctwin-lab
sudo mkdir -p "$lab_root"
sudo chmod 0755 "$lab_root"
admin="$work/admin"
sudo mkdir -p "$admin" "$work/mover"
# The administrator's folder: the key and the evidence. The mover's: what the mover can read.
sudo chown "$(id -u)" "$admin"
chmod 0700 "$admin"
sudo chown "$mover" "$work/mover"
sudo chmod 0700 "$work/mover"
# Every account must be able to reach its own folder through the work folder.
sudo chmod 0755 "$work"

as_user() {
  local u="$1"
  shift
  sudo -u "$u" -H env -u XDG_CONFIG_HOME -u XDG_DATA_HOME -u XDG_CACHE_HOME -u XDG_STATE_HOME \
    -u XDG_RUNTIME_DIR "$@"
}

spec="$here/spec-linux.tsv"
$mac && spec="$here/spec-mac.tsv"
sudo python3 "$here/plant.py" plant "$spec" "$admin/planted.json"
sudo chown "$(id -u)" "$admin/planted.json"

laptop="$(openssl rand -hex 16)"
token="$(openssl rand -hex 16)"
uid() { id -u "$1"; }
approve="$(uid pctwinbea),$(uid pctwincarl),pctwinnew1,pctwinnew2"
# The real folder, as the app uses: the helper refuses a path through a link (/var/run on Linux,
# /var on a Mac).
if $mac; then channels=/private/var/tmp/pctwin-everyone; else channels=/run/pctwin-everyone; fi
sudo rm -rf "$channels"

# The system's prompt, said yes to: the helper started as root for this move (`--dir` is made by
# the helper itself). Sets `started` to its process.
start_helper() {
  local channel="$1" log="$2"
  sudo pctwin-helper --session "$token" --mover "$(uid "$mover")" --approve "$approve" \
    --channel "$channel" --dir "$channels" --laptop "$laptop" >"$log" 2>&1 &
  started=$!
}

echo "== 1. one helper at a time; only the mover's account may call it =="
start_helper c1 "$admin/helper-1.txt"
first=$started
sleep 2
if sudo pctwin-helper --session "$token" --mover "$(uid "$mover")" --approve "$approve" \
  --channel c2 --dir "$channels-2" --laptop "$laptop" >"$admin/helper-second.txt" 2>&1; then
  echo "a second helper ran while one was running" >&2
  exit 1
fi
# Carl calls the helper started for the mover: refused, and the helper ends without serving.
if as_user pctwincarl pctwin-labmove hello "$token" c1 "$channels" >"$admin/carl.txt" 2>&1; then
  echo "another account was served by the mover's helper" >&2
  exit 1
fi
sudo kill "$first" 2>/dev/null || true
wait "$first" 2>/dev/null || true
for n in 1 2; do id "pctwinnew$n" >/dev/null 2>&1 && { echo "Carl's call made an account" >&2; exit 1; }; done

echo "== 2. the move of everyone =="
sudo rm -rf "$channels"
start_helper c3 "$admin/helper-3.txt"
helper=$started
# The helper's own process (not sudo's), for the debugger check while it runs.
sleep 1
helper_pid="$(pgrep -f "^pctwin-helper --session $token" | head -n 1 || true)"

echo "== 3. the mover cannot look into the helper =="
if [ -n "$helper_pid" ]; then
  if $mac; then
    if as_user "$mover" lldb --batch -p "$helper_pid" -o detach >"$admin/debugger.txt" 2>&1 &&
      ! grep -qi "error" "$admin/debugger.txt"; then
      echo "the mover attached a debugger to the helper" >&2
      exit 1
    fi
  else
    if as_user "$mover" cat "/proc/$helper_pid/environ" >"$admin/debugger.txt" 2>&1; then
      echo "the mover read the helper's memory" >&2
      exit 1
    fi
  fi
else
  echo "the helper's process was not found" >&2
  exit 1
fi

as_user "$mover" bash -c 'exec pctwin-labmove everyone "$0/report.json" "$@" 2>"$0/log.txt"' \
  "$work/mover" --laptop "$laptop" --new-home "$work/mover/new-home" \
  --journal "$work/mover/journal.redb" --new-names pctwinnew --hostile yes \
  --session "$token" --channel c3 --dir "$channels" >"$admin/evidence.json" &
engine=$!
# The mover's own engine, while it carries other people's things, is closed to the mover too
# (Linux: not dumpable, so even its environment is the system's to read).
if ! $mac; then
  sleep 5
  engine_pid="$(pgrep -u "$(uid "$mover")" -f "^pctwin-labmove everyone" | head -n 1 || true)"
  [ -n "$engine_pid" ] || { echo "the mover's engine was not found" >&2; exit 1; }
  if as_user "$mover" cat "/proc/$engine_pid/environ" >"$admin/engine-look.txt" 2>&1; then
    echo "the mover read their own engine while it carried other people's things" >&2
    exit 1
  fi
fi
if ! wait "$engine"; then
  # The move's own words, kept with the evidence and shown here.
  sudo cp "$work/mover/log.txt" "$admin/mover-log.txt" 2>/dev/null || true
  sudo cat "$work/mover/log.txt" >&2 || true
  # What was there to reach: the channel folder and the helper, still running or not.
  sudo ls -la "$channels" >&2 || true
  ps -o pid,user,stat,args -p "$helper" >&2 || true
  pgrep -a pctwin-helper >&2 || true
  echo "the move failed" >&2
  exit 1
fi
wait "$helper"

python3 "$here/plant.py" key "$admin/planted.json" "$mover" "$work/mover/new-home" \
  "$admin/evidence.json" "$admin/key.json"
# The administrator reads what the mover can read, to check it names nobody else's files.
sudo cp "$work/mover/report.json" "$admin/mover-report.json"
sudo cp "$work/mover/log.txt" "$admin/mover-log.txt"
sudo chown "$(id -u)" "$admin"/mover-*
sudo pctwin-labmove check-everyone "$admin/key.json" "$admin/evidence.json" \
  "$admin/mover-report.json" "$admin/mover-log.txt"

# The planted links pointed at the system's own files: they still belong to the system.
for f in /etc /etc/passwd; do
  owner="$(stat -c %u "$f" 2>/dev/null || stat -f %u "$f")"
  [ "$owner" = 0 ] || { echo "$f no longer belongs to the system" >&2; exit 1; }
done

if $mac; then
  echo "== 4. Secure Token, said as the system says =="
  python3 - "$admin/evidence.json" <<'PY'
import json, subprocess, sys
for m in json.load(open(sys.argv[1]))["made"]:
    out = subprocess.run(["sysadminctl", "-secureTokenStatus", m["name"]],
                         capture_output=True, text=True)
    has = "ENABLED" in (out.stdout + out.stderr)
    print(f'{m["name"]}: system says {has}, PCTwin said {m["secure_token"]}')
    if has != m["secure_token"]:
        sys.exit("PCTwin's word on the Secure Token differs from the system's")
PY
fi

echo "== 5. no administrator anywhere =="
none="$work/mover/none"
as_user "$mover" mkdir -p "$none"
as_user "$mover" bash -c 'exec pctwin-labmove everyone "$0/report.json" "$@" 2>"$0/log.txt"' \
  "$none" --laptop "$laptop" --new-home "$none/new-home" --journal "$none/journal.redb" \
  --new-names pctwinnone >"$admin/evidence-none.json"
# Without the helper only the mover's and the shared things move; everyone else's stay.
python3 - "$admin/planted.json" "$admin/planted-none.json" "$(id -u "$mover")" <<'PY'
import json, sys
planted = json.load(open(sys.argv[1]))
for p in planted["planted"]:
    if p["owner"] not in (sys.argv[3], "shared") and p["expect"] == "moves":
        p["expect"] = "stays"
json.dump(planted, open(sys.argv[2], "w"), indent=1)
PY
python3 "$here/plant.py" key "$admin/planted-none.json" "$mover" "$none/new-home" \
  "$admin/evidence-none.json" "$admin/key-none.json"
sudo cp "$none/report.json" "$admin/mover-report-none.json"
sudo cp "$none/log.txt" "$admin/mover-log-none.txt"
sudo chown "$(id -u)" "$admin"/mover-*
sudo pctwin-labmove check-everyone "$admin/key-none.json" "$admin/evidence-none.json" \
  "$admin/mover-report-none.json" "$admin/mover-log-none.txt"
python3 - "$admin/evidence-none.json" <<'PY'
import json, sys
e = json.load(open(sys.argv[1]))
if sorted(e["declined"]) != sorted(e["others"]) or not e["others"]:
    sys.exit("without an administrator, the others must be listed, and none scanned")
PY
echo "everyone in one move: passed"
