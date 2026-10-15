#!/usr/bin/env bash
# Bytes read from a block device since boot (field 6 of /proc/diskstats is 512-byte sectors read).
# usage: diskstat.sh <device name, e.g. loop5>
awk -v d="$1" '$3 == d { print $6 * 512 }' /proc/diskstats
