#!/usr/bin/env bash
# Bad-network simulation on Linux with netem (part of the kernel's traffic control).
#
#   sudo fixtures/netsim/linux-netem.sh on  [interface] [delay] [jitter] [loss] [rate]
#   sudo fixtures/netsim/linux-netem.sh off [interface]
#
# Defaults: interface lo, 40ms delay, 10ms jitter, 1% loss, no rate limit. On `lo` the delay
# applies in each direction, so a round trip gets twice the delay. Examples:
#   home Wi-Fi with a weak signal:  on wlan0 30ms 15ms 2%
#   crowded café:                   on wlan0 80ms 40ms 5% 5mbit
#   loopback for tests (CI):        on lo 20ms 5ms 1%
set -euo pipefail

cmd="${1:-}"
dev="${2:-lo}"
case "$cmd" in
  on)
    delay="${3:-40ms}" jitter="${4:-10ms}" loss="${5:-1%}" rate="${6:-}"
    tc qdisc del dev "$dev" root 2>/dev/null || true
    if [ -n "$rate" ]; then
      tc qdisc add dev "$dev" root netem delay "$delay" "$jitter" distribution normal loss "$loss" rate "$rate"
    else
      tc qdisc add dev "$dev" root netem delay "$delay" "$jitter" distribution normal loss "$loss"
    fi
    tc qdisc show dev "$dev"
    ;;
  off)
    tc qdisc del dev "$dev" root 2>/dev/null || true
    tc qdisc show dev "$dev"
    ;;
  *)
    echo "usage: $0 on|off [interface] [delay] [jitter] [loss] [rate]" >&2
    exit 2
    ;;
esac
