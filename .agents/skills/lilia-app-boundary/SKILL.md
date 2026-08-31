---
name: lilia-app-boundary
description: Ownership rules for deciding whether a NanaBobo change belongs in the app/ host crate, crates/nanabobo-core, the src/ui facade, src/features business pages, or the NanaUI upstream (.nanaui-pin snapshot and its upstream repository). Use when Codex touches host API registration, Bilibili adapters, credentials, shell assembly, routing, settings, global CSS, build config, @nanaui packages, or business pages.
---

# Lilia App Boundary

## Decision Rule

Put behavior where its responsibility lives:

- `app/` (host bin crate): the native NanaUI host only — window setup, V8 runtime bootstrap, `HostApiRegistry` wiring in `app/src/host_api.rs`, storage write-through, and headless acceptance tests. No business logic beyond adapter glue.
- `crates/nanabobo-core`: all Bilibili third-party adaptation, credential access, business models, commands (`AppState`/`AppError`/error codes), and the `EventSink` event abstraction. Zero UI-framework dependencies.
- `src/ui` (facade): the stable app-facing UI surface — DOM-backed components, `nanaHost.ts` bridge collation, and `nana-styles.css` global styles. Business features import this facade, never `@nanaui/*` directly.
- `src/ui/nana/`: shell assembly — `NanaShell.vue`, memory-mode routing, and the settings model.
- `src/features/**`: business pages, workflows, state, and scoped styles.
- NanaUI upstream (`.nanaui-pin/` snapshot and its upstream repository): renderer, JS engine, native shell, and component system.

## NanaUI Upstream Rule

`.nanaui-pin/` is a build snapshot of the sibling NanaUI workspace referenced by path dependencies; the upstream repository is the NanaUI source repo.

- Do not edit files under `.nanaui-pin/` to fix app problems.
- When the defect or missing capability is in the NanaUI runtime, component system, or JS engine, record the upstream issue (file path, observed behavior, repro) and route around it in app code only if unavoidable.
- Fixes belong in the upstream repository, validated there first, then refreshed into the snapshot by whoever owns it.

## Common Decisions

- New business page or workflow: implement under `src/features`, wire it in `src/ui/nana/NanaShell.vue` (a permanently mounted page with route-driven visibility) plus the router stub list in `src/main.ts`.
- New host capability: implement the business logic in `crates/nanabobo-core` commands, register the entry in `app/src/host_api.rs`, and call it from the frontend only through `src/ui/nanaHost.ts`.
- Cross-page style, token, or shell-level change: `src/ui/nana-styles.css` or the `src/ui` facade; component-system gaps go upstream (record, don't patch the snapshot).
- One-off business visualization or workflow-specific style: keep scoped in the feature component.
- Window, runtime, rendering, or engine behavior: NanaUI upstream.

## Guardrails

- Bilibili raw responses, request headers, cookies, and tokens stop at the `nanabobo-core` adapter boundary; only mapped contracts in `src/contracts` reach Vue.
- Credentials reach only the Windows Keyring via `nanabobo-core`'s credential store; never into storage files, logs, or frontend responses.
- Do not duplicate shell or style code locally to make a quick fix.
- When unsure, inspect `app/src/host_api.rs`, `crates/nanabobo-core/src/lib.rs`, `src/ui/index.ts`, and `ui/vite.config.ts` before choosing a boundary.
- If both app and upstream must change, define the interface first, wire app behavior through it, and record the upstream follow-up.
