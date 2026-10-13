# 4. Private modules use tagged core versions

Date: 13 October 2026. Status: accepted.

## Context

Some parts of PCTwin are not open source (the AI layer and paid features). They live in a
private repository. The open core must never depend on them, and must build and work alone.

## Decision

- The core never depends on private code. CI checks this on every change
  (`scripts/public-only.sh`): every package must come from this repository or crates.io.
- Private modules depend on a tagged version of this repository, by git tag and the exact commit
  the tag points to (`git = "...", tag = "core-vX.Y.Z"` with the commit pinned in
  `Cargo.lock`). They never follow `main`.
- Private modules plug in through the interfaces the core defines (traits and typed messages).
  A private module can add behaviour; it cannot change what the core checks.
- The core crates are not published to crates.io until there is an outside use for them.

## Why

- A tag is a fixed, reviewable version: the private repository moves to a new core only when
  someone chooses to, and its own CI proves the pair works.
- Pinning the commit in `Cargo.lock` means a moved tag cannot change what is built.

## Consequences

- Core changes that break an interface need a new tag and a matching change in the private
  repository.
- The public release never contains private code, so its SBOM and licence notices cover the
  public core alone.
