---
name: lilia-app-design
description: Design and interaction standards for the NanaBobo native NanaUI desktop app. Use when Codex designs, implements, reviews, or fixes application pages, sidebar entries, empty/loading/error states, cards, dialogs, menus, scoped styles, visual hierarchy, or any UI behavior in src/features or src/ui.
---

# Lilia App Design

## Core Direction

Design the application as a restrained engineering tool. Prioritize clear position, current state, available actions, and decisions that need user attention.

Every main view must answer:

- Where am I?
- What is the current state?
- Does the user need to act?

Never show technical implementation notes, roadmap placeholders, or UI that looks functional but is not wired to real behavior. Sidebar items, buttons, menu entries, disabled controls, and status surfaces must represent actual reachable state or real unavailable state.

## Layout

- Use the local `src/ui` facade components for controls and the `src/ui/nana/NanaShell.vue` shell for sidebar, navigation, and native window chrome; do not rebuild shell structure inside an app page.
- Business pages stay permanently mounted inside the shell; navigation toggles visibility via route-driven `is-active` classes, so avoid entrance animations on mount and keep per-page state stable across navigation.
- Avoid landing-page composition, hero blocks, oversized headings, decorative panels, marketing card streams, and nested cards.
- Use cards only for independent information groups, repeated items, dialogs, and actual tool containers.

## Visual Language

- Keep the interface quiet, dense enough for repeated work, and easy to scan.
- Use this hierarchy: main content > current state > process information and secondary actions.
- Use short, direct, actionable copy. Buttons name actions, status text states facts, hints explain impact.
- Prefer icon buttons for familiar tools such as collapse, search, settings, and expand.
- Use text buttons when the action needs explicit wording.
- Rows and controls must have clear hover, active, muted, disabled, loading, empty, and error states without layout shift.

## Tokens And Styles

Use CSS variables from `src/ui/nana-styles.css` (the migrated Lilia-style token set). Do not create a second public color system inside the app.

- Surface: `--bg`, `--bg-elev`, `--bg-subtle`
- Interaction: `--bg-hover`, `--bg-active`, `--border-soft`, `--border`, `--border-strong`
- Text: `--text`, `--text-muted`, `--text-faint`
- Accent: `--accent`, `--accent-strong`, `--accent-soft`, `--accent-text`
- State: `--ok`, `--warn`, `--err`, `--ok-soft`, `--warn-soft`, `--err-soft`

Use soft state tokens only for state backgrounds, selected backgrounds, dangerous hover, or confirmation states. Do not turn them into large page backgrounds or decorative blocks.

Keep app-specific CSS in the business component's scoped style. Move cross-page or shell-level styles into `src/ui/nana-styles.css`; component-system gaps belong to the NanaUI upstream — record them, do not patch the `.nanaui-pin/` snapshot.

## Page Surfaces

- Use the `.page-header`, `.card`, `.kv`, and other global page classes from `nana-styles.css` before adding local CSS.
- Page titles should stay compact: about 18px/600 for page headings, 13px muted text for descriptions.
- Keep page headers for orientation and state, not hero content.
- Cards use 8px radius, modest padding, `--border`, and stable dimensions where content can change.
- Card headings should feel like group labels, not marketing headlines.
- Key-value layouts must handle long paths, versions, and identifiers with wrapping.
- Menus, dropdowns, context menus, and confirm dialogs use the native shell's components and pass real items and handlers.
- Dangerous actions use error color only for dangerous hover, pending, or confirmation state.

## Motion And State

- Keep hover, active, border, and text color transitions around 0.12s.
- Avoid attention-grabbing movement, scaling, strong shadows, or saturated decorative color.
- Loading, empty, and disabled states must not resize controls or shift surrounding layout.

## Honest State

For design work, keep the user-facing UI normal and truthful:

- Important state must be visible as product state: pending login, blocked work, failed action, empty result, loading, unavailable upstream, and recoverable error all need clear user-facing states and real actions where applicable.
- If a visible action cannot be executed, show a truthful unavailable state or remove the action. Do not present placeholder buttons, fake menus, fake sidebar items, or unconnected affordances.

## Review Checklist

- The UI still feels like a restrained engineering tool.
- The strongest visual weight is on real user content and current state.
- New colors come from the `nana-styles.css` tokens or belong in `nana-styles.css` first.
- App-specific styles are scoped to business components.
- Navigation and actions are real, reachable, and wired.
- No technical implementation explanation leaks into production UI.
- Light and dark appearances remain readable.
- Hover, active, loading, empty, and disabled states keep stable geometry.
