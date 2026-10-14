#!/usr/bin/env bash
# Same-machine move between two user accounts (Task List 0.7), Linux and macOS.
#
# Makes two ordinary accounts with different names, gives each a private folder the other cannot
# read, builds a synthetic old laptop in the first, then moves it to the second through the real
# engine: pairing code, number match, sealed link, safety gate and change journal. Passes only if
# every file is accounted for on both sides, every file that landed matches its original byte for
# byte, and the old laptop's folder is unchanged.
#
# Needs passwordless sudo (as on CI runners). Usage: run.sh <folder with pctwin-lab and
# pctwin-labmove> [work folder]. Leaves its evidence (reports and snapshots) in <work>/report.
set -euo pipefail

bin="$1"
work="${2:-/tmp/pctwin-two-accounts}"
old_user=pctwin-old-ada
new_user=pctwin-new-bob

make_user() {
  local u="$1"
  if id "$u" >/dev/null 2>&1; then return; fi
  case "$(uname)" in
    Linux) sudo useradd --create-home --shell /bin/bash "$u" ;;
    Darwin)
      # A throwaway password for a throwaway account on a throwaway machine; never printed.
      sudo sysadminctl -addUser "$u" -password "$(openssl rand -hex 16)" >/dev/null 2>&1
      ;;
    *) echo "unsupported system" >&2; exit 2 ;;
  esac
}

make_user "$old_user"
make_user "$new_user"

sudo install -m 0755 "$bin/pctwin-lab" "$bin/pctwin-labmove" /usr/local/bin/
sudo rm -rf "$work"
sudo mkdir -p "$work/old" "$work/new" "$work/handoff" "$work/report"
sudo chown "$old_user" "$work/old"
sudo chown "$new_user" "$work/new"
sudo chmod 0700 "$work/old" "$work/new"
sudo chmod 1777 "$work/handoff" "$work/report"

as_old() { sudo -u "$old_user" -H "$@"; }
as_new() { sudo -u "$new_user" -H "$@"; }

as_old pctwin-lab make "$work/old/laptop" 21 150 4194304 >"$work/report/made.json"
as_old pctwin-lab snapshot "$work/old/laptop" "$work/report/before.json"

# The accounts really are separate: the new laptop's account cannot read the old one's files.
if as_new ls "$work/old/laptop" >/dev/null 2>&1; then
  echo "the two accounts can read each other's folders; the test would prove nothing" >&2
  exit 1
fi

as_new pctwin-labmove receive "$work/handoff" "$work/report/received.json" \
  "$work/new/journal.redb" --lab "$work/new/home" "$(id -u "$new_user")" &
receiver=$!
as_old pctwin-labmove send "$work/handoff" "$work/report/sent.json" \
  --lab "$work/old/laptop" "$(id -u "$old_user")"
wait "$receiver"

as_old pctwin-lab snapshot "$work/old/laptop" "$work/report/after.json"
as_new pctwin-lab snapshot "$work/new/home" "$work/report/dest.json"
pctwin-labmove check "$work/report/before.json" "$work/report/after.json" \
  "$work/report/dest.json" "$work/report/sent.json" "$work/report/received.json"
