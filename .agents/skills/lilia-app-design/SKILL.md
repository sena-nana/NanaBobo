---
name: lilia-app-design
description: Design and interaction standards for the NanaBobo native NanaUI L3 desktop app. Use when Codex designs, implements, reviews, or fixes application pages, sidebar entries, empty/loading/error states, cards, dialogs, menus, visual hierarchy, or any UI behavior in app/src.
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

- Use `DesktopShell` for sidebar, navigation, inspector, overlay, and native window chrome; do not rebuild shell structure inside a page.
- Keep per-page state in the session object; navigation switches the primary `mount`, not a second window.
- Avoid landing-page composition, hero blocks, oversized headings, decorative panels, marketing card streams, and nested cards.

## Visual Language

- Keep the interface quiet, dense enough for repeated work, and easy to scan.
- Use this hierarchy: main content > current state > process information and secondary actions.
- Use short, direct, actionable copy. Buttons name actions, status text states facts, hints explain impact.
- Rows and controls must have clear hover, active, muted, disabled, loading, empty, and error states without layout shift.

## Tokens And Styles

Use NanaUI semantic tokens and built-in controls. Do not create a second public color system inside the app. Component-system gaps belong to the NanaUI upstream — record them, do not patch the `.nanaui-pin/` snapshot.

## Honest State

- Important state must be visible as product state: pending login, blocked work, failed action, empty result, loading, unavailable upstream, and recoverable error all need clear user-facing states and real actions where applicable.
- If a visible action cannot be executed, show a truthful unavailable state or remove the action.

## Review Checklist

- The UI still feels like a restrained engineering tool.
- The strongest visual weight is on real user content and current state.
- Navigation and actions are real, reachable, and wired.
- No technical implementation explanation leaks into production UI.
- Light and dark appearances remain readable.
