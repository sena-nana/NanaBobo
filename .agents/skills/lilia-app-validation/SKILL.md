---
name: lilia-app-validation
description: Validation strategy for NanaBobo desktop app changes. Use when Codex needs to choose or report checks after host API entries, core crate business logic, frontend UI, routes, contracts, dependencies, build config, documentation, or tests change.
---

# Lilia App Validation

## Choose The Smallest Meaningful Check

Run checks that validate real behavior affected by the change. Prefer targeted functional checks over broad or brittle assertions.

- Use `cargo test` in the root workspace for Rust changes: it runs the `nanabobo-core` business tests plus the `app` host's headless acceptance tests, which mount the real Vue IIFE in a V8 engine and assert the shell, home page, and per-page navigation via semantic snapshots.
- Use `yarn test` (vitest) for frontend contract mapping, route-driven visibility, component behavior, and business logic.
- Use `cd ui && npm run build` to validate the frontend bundle; then `cargo build -p nanabobo-app` embeds the fresh IIFE and CSS into the `target/debug/nanabobo.exe` host for a manual smoke run of the real window.
- Use `cargo fmt`/`cargo clippy` when touching Rust sources if the change is non-trivial.

## Test Quality

- Add tests only for behavior changes or meaningful regression risk.
- Do not add tests for documentation-only, comment-only, or formatting-only changes.
- Do not write low-value tests that only hard-match log text, incidental strings, implementation comments, or snapshot-like markup.
- Do not use raw string matching as the main assertion when it does not prove the feature works; assert the behavior through roles, events, state changes, host API effects, data results, or observable outcomes.
- Test user-visible behavior, host API results, route outcomes, and data-contract handling.
- Keep tests focused on the changed capability and existing public behavior.

## Build And Dependency Changes

When changing the `ui/` vite build config, npm dependencies, or the Rust dependency graph:

- Treat the change itself as the behavior under test; run the affected build end to end (`cd ui && npm install && npm run build`, then `cargo build -p nanabobo-app`).
- Remember the local prerequisites: the `.nanaui-pin/` snapshot next to the repo backs all `nana-*` path dependencies, and the V8 prebuilt library requires `RUSTY_V8_SKIP_DOWNLOAD=1` (already set in `.cargo/config.toml`); copy `rusty_v8.lib` into `target/release/gn_out/obj/` before release builds.
- Confirm no build artifact (`ui/dist`, `target/`) is committed accidentally.

## Reporting

- State exactly which checks ran and whether they passed.
- If a check was skipped, explain why it was not necessary for the change.
- If a check cannot run, include the blocking error and remaining risk.
- Do not treat unrelated full-suite failures as blockers without confirming they touch the edited surface.
