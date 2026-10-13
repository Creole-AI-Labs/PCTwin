# 2. Licence: Apache 2.0, code contributions closed

Date: 5 October 2026. Status: accepted.

## Context

People trust a tool that reads their whole laptop only if they can see what it does. The code
must be open to read, build and audit. A one-person company also needs to be able to answer for
every line it ships.

## Decision

- The code is under the Apache License 2.0 (`LICENSE`, `NOTICE`).
- "PCTwin" and the logo are trademarks and are not covered by the code licence.
- We do not accept code contributions: outside pull requests are closed automatically. Bug
  reports, ideas and security reports are welcome (`CONTRIBUTING.md`, `SECURITY.md`).

## Why

- Apache 2.0 is permissive and includes an explicit patent grant.
- Closing contributions keeps every line written and reviewed by the people who answer for it,
  and avoids contributor licence agreements.

## Consequences

- Dependencies must use licences compatible with Apache 2.0; `cargo-deny` enforces the list in
  `deny.toml`, and every release ships a licence notices file.
