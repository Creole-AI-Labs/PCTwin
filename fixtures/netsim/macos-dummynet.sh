#!/usr/bin/env bash
# Bad-network simulation on macOS with dummynet (dnctl + pfctl), scriptable where Apple's
# Network Link Conditioner is a settings pane.
#
#   sudo fixtures/netsim/macos-dummynet.sh on  [delay ms] [loss 0..1] [bandwidth]
#   sudo fixtures/netsim/macos-dummynet.sh off
#
# Traffic to 127.0.0.1 can skip the rules, so the shaping applies to a second loopback address,
# 127.0.0.3: point the test's connection there. Not yet run on CI; check it with
# `ping -c 20 127.0.0.3` before relying on it.
set -euo pipefail

alias_ip=127.0.0.3
anchor=pctwin-netsim
case "${1:-}" in
  on)
    delay="${2:-40}" loss="${3:-0.01}" bw="${4:-50Mbit/s}"
    ifconfig lo0 alias "$alias_ip" up
    dnctl -q flush
    dnctl pipe 1 config bw "$bw" delay "$delay" plr "$loss"
    { cat /etc/pf.conf; echo "dummynet-anchor \"$anchor\""; echo "anchor \"$anchor\""; } | pfctl -q -f -
    printf 'dummynet in from any to %s pipe 1\n' "$alias_ip" | pfctl -q -a "$anchor" -f -
    pfctl -q -e 2>/dev/null || true
    dnctl list
    ;;
  off)
    pfctl -q -f /etc/pf.conf 2>/dev/null || true
    dnctl -q flush
    ifconfig lo0 -alias "$alias_ip" 2>/dev/null || true
    ;;
  *)
    echo "usage: $0 on|off [delay ms] [loss 0..1] [bandwidth]" >&2
    exit 2
    ;;
esac
