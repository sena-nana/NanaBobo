# 桌面应用脚手架

最小 Tauri 2 + Vue 3 + TypeScript 脚手架。通用 UI、配置、工具、构建流程和窗口状态插件由 LiliaUI 提供，本仓库只保留应用配置、路由、命令、业务页面目录和项目专属 Tauri 边界。

## 结构

```text
src/
  AppRoot.vue
  main.ts
  app.ts
  app.config.ts
  routes.ts
  commands.ts
  settings.ts
  overlays.ts
  runtime.ts
  diagnostics.ts
  features/
src-tauri/
tests/
lilia.tools.profile.mjs
```

## 命令

```bash
yarn install
yarn dev
yarn tauri:dev
yarn test
yarn build
yarn verify
```

## 配置

根目录 `app.config.json` 是应用名称、产品标题、版本和 Tauri 标识的同步来源。修改后运行：

```bash
yarn sync:app-config
```

应用在 `src/app.ts` 中显式创建 Vue App 和 Router，并装配 Shell、命令、设置、Overlay 与可选 UI 安装器。Shell 导航在 `src/app.config.ts`，业务路由在 `src/routes.ts`，命令在 `src/commands.ts`，设置模型在 `src/settings.ts`。开发诊断只会在开发模式且启用 `VITE_LILIA_AGENT_DEBUG=1` 时动态加载。

`lilia.tools.profile.mjs` 是本模板自己的工具与 Agent 调试检查 Profile；共享 `@lilia/tools` 不假设应用目录、首页或 Agent target。

## 公共依赖

```json
{
  "dependencies": {
    "@lilia/ui": "github:sena-nana/LiliaUI#workspace=@lilia/ui&head=main",
    "@lilia/config": "github:sena-nana/LiliaUI#workspace=@lilia/config&head=main",
    "@lilia/tools": "github:sena-nana/LiliaUI#workspace=@lilia/tools&head=main",
    "@lilia/build": "github:sena-nana/LiliaUI#workspace=@lilia/build&head=main",
    "vue": "^3.5.39",
    "vue-router": "^5.1.0"
  }
}
```

Rust 侧通过 Cargo git dependency 消费 `tauri-plugin-lilia`。
