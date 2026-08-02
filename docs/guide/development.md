# 开发启动

## 项目结构

```text
Tauri-Template/
├── src/
│   ├── app.ts
│   ├── AppRoot.vue
│   ├── capabilities/
│   ├── commands.ts
│   ├── features/
│   ├── routes.ts
│   ├── ui/
│   └── main.ts
├── src-tauri/
├── app.config.json
├── lilia.tools.profile.mjs
└── tests/
```

## 本地运行

```bash
npm install --global corepack@0.35.0
corepack enable
corepack yarn install
corepack yarn dev
corepack yarn tauri:dev
```

本仓库统一使用 Node.js 26.5.0 与 Yarn 4.17.1。Corepack 从 Node.js 25 起不再随 Node.js 分发，因此首次使用前需要通过 npm 显式安装 Corepack 0.35.0。

## LiliaUI 本地联调

默认 `package.json` 和提交版 `yarn.lock` 固定使用 GitHub 上同一个 LiliaUI commit。普通 `yarn install` 不依赖本机存在 LiliaUI 源码仓库。

需要同时修改 LiliaUI 时，从模板仓库根目录运行：

```bash
yarn liliaui:local
```

该命令会通过 `yarn link --relative` 临时维护项目级 `resolutions`，把当前 manifest 中的 `@lilia/*` 包切到本地 `portal:` 依赖并刷新安装。如果 LiliaUI 不在相邻目录，可用 `LILIA_UI_LOCAL_PATH` 指定路径：

```powershell
$env:LILIA_UI_LOCAL_PATH = "C:\Files\workspace\LiliaUI"
yarn liliaui:local
Remove-Item Env:LILIA_UI_LOCAL_PATH
```

提交依赖或锁文件变更前，先切回固定 GitHub 依赖：

```bash
yarn liliaui:remote
yarn liliaui:status
```

`yarn liliaui:status` 会检查当前所有 LiliaUI 包来自本地 `portal:` 还是同一个固定 GitHub commit。提交策略是：默认远端 manifest 和锁文件可以入库，本地 `resolutions` / `portal:` lockfile 只作为个人联调状态，不随普通业务提交一起提交。

## 验证

```bash
yarn test
yarn build
cargo check --manifest-path src-tauri/Cargo.toml
yarn verify
yarn test:e2e # Nana 预设
```

`yarn agent:debug` 会读取 `lilia.tools.profile.mjs`，输出当前脚手架边界、关键文件、Agent 调试环境变量、应用声明的 `data-agent-id` 目标和桌面 replay 工具探测结果。

前端诊断由当前预设 adapter 按需装配。只有开发模式同时设置 `VITE_LILIA_AGENT_DEBUG=1`，或使用 `agent-debug` mode 时，才会异步加载诊断能力；普通生产构建不注册调试监听器。
