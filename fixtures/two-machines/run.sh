#!/usr/bin/env bash
# Two machines over a network (Task List 1.8, decided 20 October 2026): two separate network
# stacks, each with its own interface, address and loopback (Linux network namespaces), joined by a
# virtual cable whose conditions copy poor Wi-Fi: delay with jitter, packet loss and a limited
# rate. A lab laptop pairs and moves across it, through the real link and engine, and the cable is
# cut completely for a short while part way through; the move must ride through the drop and the
# check must find every file whole.
#
# What this does not cover, said in every report: real home Wi-Fi and real routers (named gap,
# testers), discovery by announcement (the lab pairs through a handoff folder), and a drop long
# enough to end the connection (the reconnect suite in pctwin-move proves that over real sockets).
#
# Needs root through password-free sudo (as on CI runners) and iproute2 with netem.
# Usage: run.sh <folder with pctwin-lab and pctwin-labmove> <work folder>
set -euo pipefail
bin="$(cd "$1" && pwd)"
work="$2"
me="$(id -un)"
old=pctwin-old
new=pctwin-new

cleanup() {
  sudo ip netns del "$old" 2>/dev/null || true
  sudo ip netns del "$new" 2>/dev/null || true
}
trap cleanup EXIT
cleanup

# Each side runs as this account, in its own network.
in_ns() {
  local ns="$1"
  shift
  sudo ip netns exec "$ns" sudo -u "$me" -H env "PATH=$PATH" "$@"
}

echo "== 1. two machines and a poor cable between them =="
sudo modprobe sch_netem 2>/dev/null || true
sudo ip netns add "$old"
sudo ip netns add "$new"
sudo ip link add old0 type veth peer name new0
sudo ip link set old0 netns "$old"
sudo ip link set new0 netns "$new"
sudo ip -n "$old" addr add 10.77.0.1/24 dev old0
sudo ip -n "$new" addr add 10.77.0.2/24 dev new0
for ns in "$old" "$new"; do sudo ip -n "$ns" link set lo up; done
sudo ip -n "$old" link set old0 up
sudo ip -n "$new" link set new0 up
# Poor Wi-Fi, each way: 40 ms with 15 ms of jitter, 2% of packets lost, 40 megabits a second.
poor="delay 40ms 15ms loss 2% rate 40mbit"
sudo ip netns exec "$old" tc qdisc add dev old0 root netem $poor
sudo ip netns exec "$new" tc qdisc add dev new0 root netem $poor
sudo ip netns exec "$new" ping -c 2 -W 2 10.77.0.1 >/dev/null
echo "the new machine reaches the old one at 10.77.0.1"

echo "== 2. a lab laptop on the old machine =="
rm -rf "$work"
mkdir -p "$work/handoff" "$work/evidence"
"$bin/pctwin-lab" make "$work/laptop" 77 240 8388608 >"$work/evidence/made.json"
"$bin/pctwin-lab" snapshot "$work/laptop" "$work/evidence/before.json"

echo "== 3. pair and move across the cable =="
in_ns "$new" "$bin/pctwin-labmove" receive "$work/handoff" "$work/evidence/received.json" \
  "$work/journal.redb" --lab "$work/new" lab-person-new &
receiver=$!
in_ns "$old" env PCTWIN_LAB_BIND=10.77.0.1:0 "$bin/pctwin-labmove" send "$work/handoff" \
  "$work/evidence/sent.json" --lab "$work/laptop" lab-person &
sender=$!

echo "== 4. the cable is cut for 8 seconds part way through =="
# Once the first files have landed, nothing passes either way for 8 seconds, then the poor cable
# is back; the connection must carry on where it was.
for _ in $(seq 120); do
  [ -n "$(find "$work/new" -type f 2>/dev/null | head -n 1)" ] && break
  sleep 0.5
done
sudo ip netns exec "$old" tc qdisc change dev old0 root netem loss 100%
sudo ip netns exec "$new" tc qdisc change dev new0 root netem loss 100%
sleep 8
sudo ip netns exec "$old" tc qdisc change dev old0 root netem $poor
sudo ip netns exec "$new" tc qdisc change dev new0 root netem $poor
echo "the cable is back"

status=0
wait "$sender" || status=1
wait "$receiver" || status=1
[ "$status" = 0 ] || { echo "a side of the move failed" >&2; exit 1; }

echo "== 5. every file arrived whole =="
"$bin/pctwin-lab" snapshot "$work/laptop" "$work/evidence/after.json"
"$bin/pctwin-lab" snapshot "$work/new" "$work/evidence/dest.json"
"$bin/pctwin-labmove" check "$work/evidence/before.json" "$work/evidence/after.json" \
  "$work/evidence/dest.json" "$work/evidence/sent.json" "$work/evidence/received.json" \
  --only "$work/evidence/made.json" | tee "$work/evidence/check.txt"
