---
name: nanabobo-development
description: NanaBobo desktop development workflow for Bilibili QR login, account state, live-room data, future danmaku and replay features, Tauri commands, credential security, and Vue/Rust contract changes. Use when implementing or reviewing NanaBobo features, Bilibili adapters, login/session handling, room tools, or app-owned Tauri boundaries.
---

# NanaBobo Development

## Workflow

1. Read `AGENTS.md`, the relevant feature, contract, Tauri command, capability, and test files before editing.
2. Keep shared shell, theme, settings, build, and window behavior in LiliaUI. Keep NanaBobo business behavior in `src/features`, `src/contracts`, and `src-tauri`.
3. Define or update the stable internal contract first. Map Bilibili responses inside `src-tauri/src/bilibili`; never expose upstream JSON, request headers, cookies, or tokens to Vue.
4. Keep Bilibili session cookies in the OS credential store only. Use in-memory fakes for tests; never add real credentials, recordings, or raw upstream payloads to fixtures.
5. Add only controls and routes backed by real behavior. Every loading, empty, expired, unauthenticated, rate-limited, and upstream-failure state needs a truthful user-facing recovery action.
6. Keep QR sessions short-lived and in memory. Polling must stop on success, expiry, logout, unmount, or unrecoverable error.
7. For long-lived danmaku or replay work, define reconnection, backpressure, persistence, and replay contracts before adding UI.

## Contracts and errors

- Account responses contain only `authenticated` and a safe account summary such as user id, display name, and avatar URL.
- QR responses contain a session id, generated SVG, expiry, and an explicit `pending`, `scanned`, `success`, `expired`, or `failed` state.
- Room responses use NanaBobo-owned fields such as room id, title, live status, owner name, viewer count, cover URL, and update time.
- Map invalid input, missing login, expired QR codes, request limits, and upstream outages to stable error codes. Do not serialize raw `reqwest`, Bilibili, or Keyring errors to the frontend.

## Validation

- Run focused frontend behavior tests for contract mapping, account state transitions, room lookup, and truthful errors.
- Run Rust unit tests for response mapping, QR state transitions, credential-store fakes, input validation, and redaction.
- Run `yarn agent:debug --json`, `yarn test`, `yarn build`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `yarn verify` when the change crosses app, frontend, or Tauri boundaries.
- Run the skill validator on this directory after changing the skill itself.
