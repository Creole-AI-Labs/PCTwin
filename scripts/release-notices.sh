#!/usr/bin/env bash
# Writes the release's bill of materials and licence notices into the folder given ($1):
#   pctwin.cdx.json               CycloneDX SBOM of the app's Rust crates
#   THIRD_PARTY_LICENCES.txt      licence texts for those crates (cargo-about)
#   screens-licences.json         licences of the screens' npm packages that ship in the app
set -euo pipefail

out="${1:?usage: release-notices.sh <folder>}"
root="$(cd "$(dirname "$0")/.." && pwd)"
mkdir -p "$out"
out="$(cd "$out" && pwd)"
cd "$root"

cargo cyclonedx --format json --spec-version 1.5 --override-filename pctwin --target all
cp app/src-tauri/pctwin.json "$out/pctwin.cdx.json"
find . -name 'pctwin.json' -path '*/crates/*' -delete -o -name 'pctwin.json' -path './app/*' -delete

cargo about generate --locked --manifest-path app/src-tauri/Cargo.toml about.hbs -o "$out/THIRD_PARTY_LICENCES.txt"

(cd app && pnpm install --frozen-lockfile --silent && pnpm licenses list --prod --json > "$out/screens-licences.json")

ls -l "$out"
