"""Prints one field of the last JSON line read from stdin (lists as compact JSON)."""
import json
import sys

lines = [line for line in sys.stdin.read().splitlines() if line.startswith("{")]
if not lines:
    sys.exit("jf.py: no JSON line on stdin")
value = json.loads(lines[-1]).get(sys.argv[1])
print(value if isinstance(value, (str, int, float)) else json.dumps(value, separators=(",", ":")))
