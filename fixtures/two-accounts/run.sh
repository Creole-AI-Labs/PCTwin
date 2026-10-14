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

# ---------------------------------------------------------------------------------------------
# Real-laptop fixtures: the same move between the accounts' real special folders, set up the way
# real laptops are. Linux: translated folder names on both sides (German, Japanese and Arabic on
# the old laptop, Spanish on the new). Both systems: the new laptop is already in use, with a
# file of its own at a path the move also brings; it must be left exactly as it was.
echo "== real special folders =="
profile="$work/profile"
sudo rm -rf "$profile"
sudo mkdir -p "$profile/handoff" "$profile/report"
sudo chmod 1777 "$profile/handoff" "$profile/report"

user_home() { eval echo "~$1"; }
old_home="$(user_home "$old_user")"
new_home="$(user_home "$new_user")"
if [ "$(uname)" = Darwin ]; then
  for u in "$old_user" "$new_user"; do sudo createhomedir -c -u "$u" >/dev/null 2>&1 || true; done
  old_home="/Users/$old_user"
  new_home="/Users/$new_user"
fi

if [ "$(uname)" = Linux ]; then
  as_old mkdir -p "$old_home/.config" "$old_home/Desktop" "$old_home/Dokumente" "$old_home/Downloads" \
    "$old_home/写真" "$old_home/موسيقى" "$old_home/Videos"
  as_old tee "$old_home/.config/user-dirs.dirs" >/dev/null <<'DIRS'
XDG_DESKTOP_DIR="$HOME/Desktop"
XDG_DOCUMENTS_DIR="$HOME/Dokumente"
XDG_DOWNLOAD_DIR="$HOME/Downloads"
XDG_PICTURES_DIR="$HOME/写真"
XDG_MUSIC_DIR="$HOME/موسيقى"
XDG_VIDEOS_DIR="$HOME/Videos"
DIRS
  as_new mkdir -p "$new_home/.config" "$new_home/Desktop" "$new_home/Documentos" "$new_home/Downloads" \
    "$new_home/Pictures" "$new_home/Music" "$new_home/Videos"
  as_new tee "$new_home/.config/user-dirs.dirs" >/dev/null <<'DIRS'
XDG_DESKTOP_DIR="$HOME/Desktop"
XDG_DOCUMENTS_DIR="$HOME/Documentos"
XDG_DOWNLOAD_DIR="$HOME/Downloads"
XDG_PICTURES_DIR="$HOME/Pictures"
XDG_MUSIC_DIR="$HOME/Music"
XDG_VIDEOS_DIR="$HOME/Videos"
DIRS
  new_docs="$new_home/Documentos"
else
  for d in Desktop Documents Downloads Pictures Music Videos; do
    as_old mkdir -p "$old_home/$d"
    as_new mkdir -p "$new_home/$d"
  done
  new_docs="$new_home/Documents"
fi

# Where each account's folders really are, against the plain guess; IDs stable across scans.
as_old pctwin-labmove probe "$profile/report/probe-old.json"
as_new pctwin-labmove probe "$profile/report/probe-new.json"

# The new laptop is already in use: its own file sits where an incoming one would land.
as_new mkdir -p "$new_docs"
printf 'mine, before the move\n' | as_new tee "$new_docs/empty.txt" >/dev/null

as_old pctwin-labmove fill-profile 31 80 2097152 >"$profile/report/made.json"
as_old pctwin-labmove snapshot-profile "$profile/report/before.json"
as_new pctwin-labmove receive "$profile/handoff" "$profile/report/received.json" \
  "$work/new/journal-profile.redb" &
receiver=$!
as_old pctwin-labmove send "$profile/handoff" "$profile/report/sent.json"
wait "$receiver"
as_old pctwin-labmove snapshot-profile "$profile/report/after.json"
as_new pctwin-labmove snapshot-profile "$profile/report/dest.json"

if [ "$(as_new cat "$new_docs/empty.txt")" != "mine, before the move" ]; then
  echo "the new laptop's own file was changed by the move" >&2
  exit 1
fi
pctwin-labmove check "$profile/report/before.json" "$profile/report/after.json" \
  "$profile/report/dest.json" "$profile/report/sent.json" "$profile/report/received.json"

# ---------------------------------------------------------------------------------------------
# Bad network (Linux): the lab move again with delay, jitter and loss on the loopback the two
# accounts talk over. It must finish with the same honest result.
if [ "$(uname)" = Linux ]; then
  echo "== bad network =="
  netsim="$(cd "$(dirname "$0")/../netsim" && pwd)/linux-netem.sh"
  bad="$work/bad"
  sudo rm -rf "$bad" "$work/new/home-bad"
  sudo mkdir -p "$bad/handoff" "$bad/report"
  sudo chmod 1777 "$bad/handoff" "$bad/report"
  sudo bash "$netsim" on lo 20ms 5ms 1%
  trap 'sudo bash "$netsim" off lo >/dev/null' EXIT
  as_new pctwin-labmove receive "$bad/handoff" "$bad/report/received.json" \
    "$work/new/journal-bad.redb" --lab "$work/new/home-bad" "$(id -u "$new_user")" &
  receiver=$!
  as_old pctwin-labmove send "$bad/handoff" "$bad/report/sent.json" \
    --lab "$work/old/laptop" "$(id -u "$old_user")"
  wait "$receiver"
  sudo bash "$netsim" off lo
  as_old pctwin-lab snapshot "$work/old/laptop" "$bad/report/after.json"
  as_new pctwin-lab snapshot "$work/new/home-bad" "$bad/report/dest.json"
  pctwin-labmove check "$work/report/before.json" "$bad/report/after.json" \
    "$bad/report/dest.json" "$bad/report/sent.json" "$bad/report/received.json"
fi
