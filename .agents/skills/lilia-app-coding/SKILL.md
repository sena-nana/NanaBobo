---
name: lilia-app-coding
description: Coding workflow for the NanaBobo native NanaUI desktop app. Use when Codex implements features, fixes bugs, refactors application code, adds routes or host API entries, changes cross-end data contracts, touches Vue pages under src/features, updates src/ui or src/main.ts, or modifies the app/ host crate and crates/nanabobo-core.
---

# Lilia App Coding

## Start With Context

- Read the relevant module, data contract, host API entry, tests, and the ownership boundary (`$lilia-app-boundary`) before editing.
- Use CodeGraph first when the repository is indexed and the task requires understanding code flow.
- Read `app/src/host_api.rs` and the matching contract in `src/contracts/` before wiring a new host call or event.
- For complex tasks, split work into clear sub-tasks and use subagents only where the boundary is clean enough for independent investigation or validation.

## Ownership

- NanaBobo owns app configuration, routes, host API registration, business pages, and the `app/` host crate plus `crates/nanabobo-core` business crate.
- Put new business pages and feature logic under `src/features/<feature>/`.
- Keep host API registration (via `HostApiRegistry`) in `app/src/host_api.rs`; keep Bilibili adapters, state, error codes, and credential storage in `crates/nanabobo-core`.
- Keep shell assembly, routing, and settings wiring in `src/ui/nana/`; the app entry that mounts the Vue IIFE is `src/main.ts`.
- Do not copy shell, theme, component-system, or runtime code from the NanaUI pin snapshot (`.nanaui-pin/`) into the app; upstream gaps are recorded, not patched locally.

## Implementation Rules

- Fix root causes at the correct boundary. Do not patch symptoms with local workarounds.
- Preserve existing structures, names, and visible behavior unless the task requires changing them.
- Keep changes scoped to the requested feature or bug.
- Before changing a cross-end contract, define the boundary first, then update `src/contracts`, the core command, the host API adapter, and functional tests together.
- Do not display technical explanations in the UI.
- Do not add controls, routes, sidebar entries, host API calls, or disabled placeholders that are not connected to real behavior.
- Keep provider-specific or experimental payloads behind core/adapter boundaries. UI should use app-level contracts and round-trip opaque provider context only when required.
- Prefer simple data flow over new abstractions. Add an abstraction only when it removes real duplication or matches an existing local pattern.
- Avoid comments that restate code. Put long-lived context, tradeoffs, or unresolved design notes in docs only when they are useful to future maintainers.
- Never overwrite user or other-agent changes. If nearby files are dirty, inspect and work with those changes.

## Frontend Pattern

- Start feature UI from `src/features/<feature>/`.
- Routing uses vue-router in memory-history mode: every page stays mounted inside `src/ui/nana/NanaShell.vue`, and `route.path` drives an `is-active` visibility class per page. Do not assume mount/unmount lifecycle on navigation.
- Import components, styles, layouts, settings, and state only through the `src/ui` facade; never import `@nanaui/*` directly from a feature.
- Use the facade's components, the CSS tokens in `src/ui/nana-styles.css`, and shell conventions before adding local UI.
- Keep business component styles scoped.
- Ensure text, controls, loading state, empty state, and error state are stable across window resizes within the 960x600 minimum.

## Host API Pattern

- Treat `crates/nanabobo-core` as the business boundary and `app/src/host_api.rs` as the thin adapter that registers core commands into `HostApiRegistry`.
- Keep host API names, payload shapes, and frontend `Nana.host.invoke(name, [payload])` calls synchronized through `src/ui/nanaHost.ts`; do not bypass the facade bridge.
- Errors cross to JS as structured `{ code, message }` exceptions with stable error codes; never include raw upstream responses, cookies, or tokens.
- Danmaku state and messages flow from core through the `EventSink` trait to the host adapter and arrive in JS via `Nana.host.on("nanabobo://danmaku/...")` events.
- Storage uses the `storage_load/save/remove/clear` host API (file-backed localStorage write-through), seeded by `src/main.ts` before the app mounts.

## Before Finishing

- Remove duplicate branches, dead state, unused helper functions, and comments that only narrate the code.
- Confirm no fake UI or unconnected action was introduced.
- Run the smallest meaningful validation for the changed behavior (see `$lilia-app-validation`), or explain why validation was not run.
