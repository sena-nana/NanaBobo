---
name: lilia-app-coding
description: Coding workflow for the NanaBobo native NanaUI L3 desktop app. Use when Codex implements features, fixes bugs, refactors application code, changes host wiring, or modifies the app/ host crate and crates/nanabobo-core.
---

# Lilia App Coding

## Start With Context

- Read the relevant module, core command, and the ownership boundary (`$lilia-app-boundary`) before editing.
- Use CodeGraph first when the repository is indexed and the task requires understanding code flow.
- Read `app/src/session.rs` and `crates/nanabobo-core/src/commands.rs` before wiring a new business action.

## Ownership

- NanaBobo owns app configuration, L3 pages, and the `app/` host crate plus `crates/nanabobo-core`.
- Keep Bilibili adapters, state, error codes, and credential storage in `crates/nanabobo-core`.
- Keep shell assembly and page `mount` in `app/src/ui.rs`; keep session and persistence in `app/src/session.rs`.
- Do not copy shell, theme, component-system, or runtime code from the NanaUI pin snapshot (`.nanaui-pin/`) into the app; upstream gaps are recorded, not patched locally.

## Implementation Rules

- Fix root causes at the correct boundary. Do not patch symptoms with local workarounds.
- Preserve existing structures, names, and visible behavior unless the task requires changing them.
- Keep changes scoped to the requested feature or bug.
- Do not display technical explanations in the UI.
- Do not add controls, pages, sidebar entries, or disabled placeholders that are not connected to real behavior.
- Prefer `update_component` / `mount` over rebuilding the whole tree on each click.
- Click handlers dispatch a cheap `Wake`; drain session events in `RuntimeProgram::update`.
- Never overwrite user or other-agent changes. If nearby files are dirty, inspect and work with those changes.

## Before Finishing

- Remove duplicate branches, dead state, unused helper functions, and comments that only narrate the code.
- Confirm no fake UI or unconnected action was introduced.
- Run the smallest meaningful validation for the changed behavior (see `$lilia-app-validation`), or explain why validation was not run.
