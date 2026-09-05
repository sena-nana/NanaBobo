---
name: lilia-app-validation
description: Validation strategy for NanaBobo desktop app changes. Use when Codex needs to choose or report checks after host, core crate, UI, documentation, or tests change.
---

# Lilia App Validation

## Choose The Smallest Meaningful Check

- Use `cargo test` in the root workspace for Rust changes: it runs `nanabobo-core` business tests plus app session tests.
- Use `cargo build -p nanabobo-app` then smoke-run `target/debug/nanabobo.exe` when the window, shell, or page tree changed.
- Use `cargo fmt`/`cargo clippy` when touching Rust sources if the change is non-trivial.

## Test Quality

- Add tests only for behavior changes or meaningful regression risk.
- Do not add tests for documentation-only, comment-only, or formatting-only changes.
- Do not write low-value tests that only hard-match log text or incidental strings.
- Test user-visible behavior, command results, and data-contract handling.

## Reporting

- State exactly which checks ran and whether they passed.
- If a check was skipped, explain why it was not necessary for the change.
- If a check cannot run, include the blocking error and remaining risk.
