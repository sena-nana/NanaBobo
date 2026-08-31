# 开发流程

## 架构总览

应用由三层组成,均为本仓库或仓库同级快照内的真实代码:

- `app/`:NanaUI 宿主 bin crate(crate 名 `nanabobo-app`,bin 名 `nanabobo`)。`app/src/main.rs` 以 `VueRuntimeProgram::<V8Engine>::run` 启动 1200x800(最小 960x600)的原生窗口,`include_str!` 嵌入 `ui/dist/nanabobo.iife.js` 与 `nanabobo-ui.css`;`app/src/host_api.rs` 用 `HostApiRegistry` 注册业务命令(auth_qr_start/poll、auth_status、auth_logout、room_get_info、danmaku_start/stop/status)与 storage_load/save/remove/clear(localStorage 文件背书),并经 `EventSink` 把弹幕事件转发为 `nanabobo://danmaku/*` JS 事件。
- `crates/nanabobo-core/`:业务核心 crate。B站接口适配(`bilibili/`)、凭据存取(`credential_store`,Windows Keyring)、models、commands(AppState/AppError/ErrorCode)。零 UI 框架依赖。
- `src/`:Vue3 前端工程。`src/main.ts` 为 IIFE 入口(挂载、localStorage 写穿、memory 路由);`src/ui/` 是 UI 门面(DOM 包装组件 + `nanaHost.ts` 宿主桥收口 + `nana-styles.css` 全局样式),`src/ui/nana/` 是壳装配(NanaShell.vue + 路由 + 设置模型);业务页面在 `src/features/**`,稳定契约在 `src/contracts/**`。

前端使用 vue-router memory 模式:所有页面常驻在 NanaShell 内,由 `route.path` 驱动页面 class 切换可见性。

## 项目结构

`app/src/main.rs`:原生宿主入口与无头验收测试。

`app/src/host_api.rs`:host API 注册表(对应迁移前的 Tauri command 层)。

`crates/nanabobo-core/src/bilibili/`:B站第三方接口适配与字段映射。

`crates/nanabobo-core/src/credential_store/`:OS Keyring 与测试替身。

`crates/nanabobo-core/src/commands.rs`:业务命令、输入校验和错误映射。

`crates/nanabobo-core/src/events.rs`:`EventSink` 事件出口抽象。

`src/contracts/`:前端可见的稳定业务契约。

`src/features/`:业务页面(登录、房间、弹幕助手、统计、历史、设置等)。

`src/ui/`:UI 门面与壳装配;业务页面只准从这里导入 UI,禁止直接 import `@nanaui/*`。

`ui/`:vite 构建配置(lib IIFE,alias `vue` → runtime-core + vue-shim),产出 `dist/nanabobo.iife.js` 与 `dist/nanabobo-ui.css`。

## 本地前置

- Rust stable(≥ 1.92,`rust-toolchain.toml` 指向 stable)。
- Node.js 26:仓库根使用 Yarn 4(`corepack` 或预装),`ui/` 构建使用 npm。
- `.nanaui-pin/`:仓库同级目录下的 NanaUI 构建快照,所有 `nana-*` crate 与 `@nanaui/*` 包均以 path 依赖指向它,必须先就位。
- V8 预编译库:`.cargo/config.toml` 已设置 `RUSTY_V8_SKIP_DOWNLOAD=1`,链接使用 `target/<profile>/gn_out/obj/rusty_v8.lib`;release 构建前需手动把该库复制到 `target/release/gn_out/obj/`。

## 构建与运行

首次准备前端产物:

`cd ui && npm install && npm run build`

然后构建并运行桌面应用:

`cargo build -p nanabobo-app`

`target/debug/nanabobo.exe`

## 测试

`cargo test`(根 workspace):nanabobo-core 业务测试 + app 宿主无头验收(真实挂载 Vue IIFE,语义快照断言壳层、主页与逐页导航)。

`yarn test`:前端 vitest 行为测试。

## 边界

渲染器、原生壳与组件系统属于 NanaUI 上游(`.nanaui-pin/` 快照及其上游仓库),本仓库只记录上游问题,不修改快照;业务页面、路由、host API、B站适配器、凭据存储和 NanaBobo 专属契约属于本仓库。B站原始响应、请求头、Cookie 和 Token 不得越过 `nanabobo-core` 适配器边界;登录态只进入 Windows OS Keyring。
