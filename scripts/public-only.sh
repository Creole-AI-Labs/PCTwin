#!/usr/bin/env bash
# Fails if any package the workspace builds comes from outside this repository other than
# crates.io: a git source, another registry, or a path dependency that points out of the checkout.
# The public core must build from the public repository alone.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cargo metadata --locked --format-version 1 | ROOT="$root" python3 -c '
import json, os, sys
root = os.path.realpath(os.environ["ROOT"])
bad = []
for p in json.load(sys.stdin)["packages"]:
    src = p.get("source")
    name, manifest = p["name"], p["manifest_path"]
    if src is None:
        if not os.path.realpath(manifest).startswith(root + os.sep):
            bad.append(f"{name}: path outside the repository ({manifest})")
    elif src != "registry+https://github.com/rust-lang/crates.io-index":
        bad.append(f"{name}: {src}")
if bad:
    print("Packages from outside the public repository and crates.io:")
    print("\n".join(bad))
    sys.exit(1)
print("Every package is from this repository or crates.io.")
'
