---
name: lilia-agent-debug
description: The LiliaUI agent-debug entry points (yarn agent:debug, data-agent-id targets, window.__liliaAgentDebug, tauri-driver) were removed with the NanaUI migration. Use when asked about agent debugging or diagnostics for this app; current diagnostics are the headless acceptance tests in app/src/main.rs and the host_api probes (spike_echo/probe_last).
---

# Lilia Agent Debug

Agent 调试入口(`yarn agent:debug`、`data-agent-id`、`window.__liliaAgentDebug`、`tauri-driver`)已随 LiliaUI 迁移移除;当前诊断手段是 `app/src/main.rs` 的无头验收测试与 `app/src/host_api.rs` 的探针(`spike_echo` / `probe_last`)。
