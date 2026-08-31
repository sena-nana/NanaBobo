---
name: nanabobo-development
description: NanaBobo desktop development workflow for Bilibili QR login, account state, live-room data, danmaku assistant, future recording and replay features, host API entries, credential security, and Vue/Rust contract changes. Use when implementing or reviewing NanaBobo features, Bilibili adapters, login/session handling, room tools, or the app host / nanabobo-core boundaries.
---

# NanaBobo Development

## Workflow

1. Read `AGENTS.md`, the relevant feature, contract, host API entry, and test files before editing.
2. Shell, renderer, and component system come from the NanaUI runtime (`.nanaui-pin/` snapshot). Keep NanaBobo business behavior in `src/features`, `src/contracts`, `crates/nanabobo-core`, and the `app/` host adapter.
3. Define or update the stable internal contract first. Map Bilibili responses inside `crates/nanabobo-core/src/bilibili`; never expose upstream JSON, request headers, cookies, or tokens to Vue. Register host entries in `app/src/host_api.rs` and call them from JS as `Nana.host.invoke(name, [payload])` through `src/ui/nanaHost.ts`.
4. Keep Bilibili session cookies in the Windows OS Keyring only. Use in-memory fakes for tests; never add real credentials, recordings, or raw upstream payloads to fixtures.
5. Add only controls and routes backed by real behavior. Every loading, empty, expired, unauthenticated, rate-limited, and upstream-failure state needs a truthful user-facing recovery action.
6. Keep QR sessions short-lived and in memory. Polling must stop on success, expiry, logout, unmount (page hidden), or unrecoverable error.
7. For long-lived danmaku or replay work, define reconnection, backpressure, persistence, and replay contracts before adding UI.

## Events

- Danmaku state and messages leave the core through the `EventSink` trait; the host adapter forwards them to JS, and the frontend listens via `Nana.host.on("nanabobo://danmaku/message")` and `Nana.host.on("nanabobo://danmaku/status")`. Event names are part of the stable contract.
- Danmaku message text stays in page memory only; never persist it.

## Contracts and errors

- Account responses contain only `authenticated` and a safe account summary such as user id, display name, and avatar URL.
- QR responses contain a session id, generated SVG, expiry, and an explicit `pending`, `scanned`, `success`, `expired`, or `failed` state.
- Room responses use NanaBobo-owned fields such as room id, title, live status, owner name, viewer count, cover URL, and update time.
- Map invalid input, missing login, expired QR codes, request limits, and upstream outages to stable error codes. Host API errors reach JS as structured `{ code, message }` exceptions; do not serialize raw `reqwest`, Bilibili, or Keyring errors to the frontend.

## Validation

- Run focused frontend behavior tests (`yarn test`, vitest) for contract mapping, account state transitions, room lookup, and truthful errors.
- Run `cargo test` in the root workspace: core unit tests cover response mapping, QR state transitions, credential-store fakes, input validation, and redaction; the app host tests mount the real Vue IIFE headlessly and assert shell and page navigation.
- When the change crosses the frontend bundle, run `cd ui && npm run build`, then `cargo build -p nanabobo-app` and smoke-run `target/debug/nanabobo.exe`.
- Run the skill validator on this directory after changing the skill itself.
