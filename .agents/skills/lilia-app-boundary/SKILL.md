---
name: lilia-app-boundary
description: Ownership rules for deciding whether a NanaBobo change belongs in the app/ L3 host crate, crates/nanabobo-core, or the NanaUI upstream (.nanaui-pin snapshot and its upstream repository). Use when Codex touches the native host, Bilibili adapters, credentials, shell assembly, settings, or business pages.
---

# Lilia App Boundary

## Decision Rule

Put behavior where its responsibility lives:

- `app/` (host bin crate): NanaUI L3 host — window setup, `RuntimeProgram`, `DesktopShell` assembly, session glue, and file persistence for non-credential settings. No Bilibili protocol logic.
- `crates/nanabobo-core`: all Bilibili third-party adaptation, credential access, business models, commands (`AppState`/`AppError`/error codes), and the `EventSink` event abstraction. Zero UI-framework dependencies.
- NanaUI upstream (`.nanaui-pin/` snapshot and its upstream repository): renderer, native shell, and component system.

## NanaUI Upstream Rule

`.nanaui-pin/` is a build snapshot of the sibling NanaUI workspace referenced by path dependencies; the upstream repository is the NanaUI source repo.

- Do not edit files under `.nanaui-pin/` to fix app problems.
- When the defect or missing capability is in the NanaUI runtime, component system, or host loop, record the upstream issue (file path, observed behavior, repro) and route around it in app code only if unavoidable.
- Fixes belong in the upstream repository, validated there first, then refreshed into the snapshot by whoever owns it.

## Common Decisions

- New business page or workflow: implement under `app/src/ui.rs` (or a focused module next to it) and wire it in the L3 shell; keep commands in `nanabobo-core`.
- New host capability: implement the business logic in `crates/nanabobo-core` commands and call it from `app/src/session.rs`.
- Window, runtime, rendering, or engine behavior: NanaUI upstream.

## Guardrails

- Bilibili raw responses, request headers, cookies, and tokens stop at the `nanabobo-core` adapter boundary; UI only sees mapped models.
- Credentials reach only the Windows Keyring via `nanabobo-core`'s credential store; never into storage files, logs, or UI state.
- Do not duplicate shell or style code locally to make a quick fix.
- When unsure, inspect `app/src/main.rs`, `app/src/session.rs`, and `crates/nanabobo-core/src/lib.rs` before choosing a boundary.
- If both app and upstream must change, define the interface first, wire app behavior through it, and record the upstream follow-up.
