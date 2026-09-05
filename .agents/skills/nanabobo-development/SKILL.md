---
name: nanabobo-development
description: NanaBobo desktop development workflow for Bilibili QR login, account state, live-room data, danmaku assistant, future recording and replay features, credential security, and L3 host / nanabobo-core boundaries. Use when implementing or reviewing NanaBobo features, Bilibili adapters, login/session handling, room tools, or the app host / nanabobo-core boundaries.
---

# NanaBobo Development

## Workflow

1. Read `AGENTS.md`, the relevant feature, core command, and test files before editing.
2. Shell, renderer, and component system come from the NanaUI runtime (`.nanaui-pin/` snapshot). Keep NanaBobo business behavior in `crates/nanabobo-core` and L3 assembly in `app/src`.
3. Map Bilibili responses inside `crates/nanabobo-core/src/bilibili`; never expose upstream JSON, request headers, cookies, or tokens to the UI. Call core commands from `app/src/session.rs`.
4. Keep Bilibili session cookies in the Windows OS Keyring only. Use in-memory fakes for tests; never add real credentials, recordings, or raw upstream payloads to fixtures.
5. Add only controls and pages backed by real behavior. Every loading, empty, expired, unauthenticated, rate-limited, and upstream-failure state needs a truthful user-facing recovery action.
6. Keep QR sessions short-lived and in memory. Polling must stop on success, expiry, logout, or unrecoverable error.
7. For long-lived danmaku or replay work, define reconnection, backpressure, persistence, and replay contracts before adding UI.

## Events

- Danmaku state and messages leave the core through the `EventSink` trait; the L3 host enqueues them as session events. Event names stay `nanabobo://danmaku/message` and `nanabobo://danmaku/status`.
- Danmaku message text stays in page memory only; never persist it.

## Contracts and errors

- Account responses contain only `authenticated` and a safe account summary such as user id, display name, and avatar URL.
- QR responses contain a session id, scan payload (login URL), expiry, and an explicit `pending`, `scanned`, `success`, `expired`, or `failed` state. The host encodes the payload with `QrCode::encode`; `qrcode_key` stays in core memory.
- Room responses use NanaBobo-owned fields such as room id, title, live status, owner name, viewer count, cover URL, and update time.
- Map invalid input, missing login, expired QR codes, request limits, and upstream outages to stable error codes. UI messages must not include raw `reqwest`, Bilibili, or Keyring errors.

## Validation

- Run `cargo test` in the root workspace: core unit tests cover response mapping, QR state transitions, credential-store fakes, input validation, and redaction; the app crate covers snapshot trimming and session helpers.
- When the change crosses the native window, run `cargo build -p nanabobo-app` and smoke-run `target/debug/nanabobo.exe`.
- Run the skill validator on this directory after changing the skill itself.
