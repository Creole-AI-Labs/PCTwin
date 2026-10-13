# 1. Desktop shell: Tauri 2 with React, TypeScript and Vite

Date: 5 October 2026. Status: accepted.

## Context

PCTwin runs on Windows, macOS and Linux and moves a person's files, apps and settings. The engine
must be fast, memory-safe and able to call each system's own interfaces. The screens must look
the same everywhere, follow the system's accessibility settings and be translated into many
languages.

## Decision

- The engine is a Rust workspace (`crates/`).
- The desktop shell is Tauri 2 (`app/src-tauri/`), which uses the system's own web view instead
  of shipping a browser.
- The screens are React with TypeScript, built with Vite (`app/src/`).

## Why

- One language, Rust, for the engine and the shell: no second runtime to secure or ship.
- Tauri apps are small (no bundled browser), and Tauri's capability permissions let each screen
  call only the commands it needs.
- React with TypeScript has the widest pool of components, accessibility tooling (axe-core) and
  testing tools; Vite keeps builds fast.

## Consequences

- Each system's web view differs a little (WebView2, WKWebView, WebKitGTK); the screens are
  tested on all three in CI.
- Linux builds need the WebKitGTK system libraries.
