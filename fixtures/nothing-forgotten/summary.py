"""Writes the "nothing forgotten" job summary (markdown) from the evidence folder.

Usage: summary.py <evidence folder>  (the folder holding facts.json, check.txt, sweep.json.gz)
"""
import collections
import gzip
import json
import os
import sys

root = sys.argv[1]


def load(name, gz=False):
    path = os.path.join(root, name)
    if not os.path.exists(path):
        return None
    opener = gzip.open if gz else open
    with opener(path, "rt", encoding="utf-8") as f:
        return json.load(f)


facts = load("facts.json") or {}
sweep = load("sweep.json.gz", gz=True)
print(f"### Nothing forgotten, {facts.get('os', '?')} (seed {facts.get('seed', '?')})")
print()
print(f"- wall time of the sweep: {facts.get('wall_seconds', 'n/a')} s")
print(f"- check-sweep exit: {facts.get('check_exit', 'n/a')}; leaks: {facts.get('leaks', 'n/a')}; "
      f"old laptop unchanged: {facts.get('compare_clean', 'n/a')}")
print(f"- macOS privacy: {facts.get('macos_privacy', 'n/a')}")
if sweep is None:
    print("- no sweep result was produced")
else:
    print(f"- top line: `{json.dumps(sweep.get('top_line'))}`, finished: {sweep.get('finished')}")
    print(f"- items in the move record: {len(sweep['record']['items'])}; places: {len(sweep['places'])}; "
          f"blind spots: {len(sweep['blind_spots'])}; links not followed: {sweep.get('links')}")
    print()
    print("| volume | system | file system | used | seen | unexplained | flagged |")
    print("|---|---|---|---|---|---|---|")
    for v in sweep["volumes"]:
        print(f"| `{v['mount']}` | {v['is_system']} | {v['file_system']} | {v['used_bytes']} | "
              f"{v['seen_bytes']} | {v['unexplained_bytes']} | {v['flagged']} |")
    print()
    reasons = collections.Counter(b.get("reason", "?") for b in sweep["blind_spots"])
    print("Blind spots by reason: " + (", ".join(f"{k} {n}" for k, n in sorted(reasons.items())) or "none"))
check = os.path.join(root, "check.txt")
if os.path.exists(check):
    print()
    print("check-sweep output:")
    print("```")
    print(open(check, encoding="utf-8", errors="replace").read()[-3000:])
    print("```")
print()
print("| element | status | note |")
print("|---|---|---|")
for l in facts.get("labels", []):
    print(f"| {l['element']} | {l['status']} | {l['note']} |")
