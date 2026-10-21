# 3. Crate layout: one crate per job

Date: 13 October 2026. Status: accepted.

## Context

The engine has jobs with very different risks: checking everything that arrives, pairing two
laptops, moving files, recording what changed so it can be undone. Mixing them makes review and
testing harder.

## Decision

One workspace (`Cargo.toml`), the app in `app/src-tauri`, and one crate per job in `crates/`:

| Crate | Job |
|---|---|
| `pctwin-types` | Shared result statuses and their meaning |
| `pctwin-record` | The shared move record: every item found, where it goes, the approved revision |
| `pctwin-scan` | Scanning the old laptop: where each special folder really is, and what is in it |
| `pctwin-discovery` | Finding the other laptop on the local network |
| `pctwin-pairing` | Turning a short one-time code into an authenticated, encrypted link |
| `pctwin-link` | Running pairing over a real connection, then carrying sealed data |
| `pctwin-session` | Joining discovery and pairing into the journey up to the number pick |
| `pctwin-doctor` | Working out why two laptops cannot reach each other, in plain words |
| `pctwin-transfer` | Moving files in checked blocks: fingerprints, receipts and resume |
| `pctwin-gate` | The safety gate: the new laptop trusts nothing that arrives |
| `pctwin-journal` | The change journal, so a crash is reconciled and a move can be undone |
| `pctwin-power` | Keeping both laptops awake and watching the battery |
| `pctwin-lease` | The first place that calls the system unsafely (Linux write leases) |
| `pctwin-helper-os` | The administrator helper's system calls: the system's prompt, its private channel, who is calling (the second and last unsafe place, 20 October 2026) |

## Rules that come with it

- `unsafe` code is forbidden everywhere except the functions `scripts/unsafe-fence.sh` lists in
  `pctwin-lease` and `pctwin-helper-os`; CI checks this fence.
- Security-sensitive crates (`pctwin-gate`, `pctwin-pairing`, `pctwin-link`) get a second review
  in a fresh context.
- Dependencies point one way: the safety gate, the journal and the record never depend on the
  transfer or the network crates; the transfer builds on them.

## Who the administrator helper trusts (20 October 2026, after its first review)

- The helper authorises only on two facts: the caller's account, as the system records it when
  the connection is made (Windows: the token the client connected with, read through the pipe;
  macOS and Linux: the socket's peer credentials), and the private channel itself (a pipe or a
  socket folder only the mover's account and SYSTEM or root can reach).
- The caller's process ID and program path are read afterwards, by process ID, so a caller that
  starts another program, or ends and has its ID reused, can change them. They are kept as
  `program_hint`, for the record and the report, never for allowing anything.
- The later route to tie a request to a signed program, not built yet: on macOS, the peer's
  audit token (`LOCAL_PEERTOKEN`) checked against a code-signing requirement once the Developer
  ID exists; on Linux, a process handle for the peer (`SO_PEERPIDFD`, kernel 6.5 and later), so
  the program is read from the very process that connected.

## Consequences

- More crates to version together; they share one workspace version.
- New jobs (the recipe and executor crates for reinstalling apps) get their own crates when
  their work starts.
